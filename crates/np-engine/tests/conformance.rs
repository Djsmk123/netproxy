//! M1 exit-gate conformance suite: real engine + real traffic, asserting the event contract.

use futures::{SinkExt, StreamExt};
use np_engine::*;
use np_model::*;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::mpsc;

struct PocMock;
impl MockResolver for PocMock {
    fn resolve(&self, r: &RequestView<'_>) -> MockDecision {
        if r.path == "/__np_mock" {
            MockDecision::Respond(MockResponse {
                rule: "poc-1".into(),
                status: 200,
                headers: vec![("content-type".into(), "application/json".into())],
                body: bytes::Bytes::from_static(br#"{"mocked":true}"#),
            })
        } else {
            MockDecision::Pass
        }
    }
}

struct Rig {
    up: np_harness::Upstream,
    proxy: SocketAddr,
    ca_pem: String,
    bus: Bus,
    handle: Option<Box<dyn EngineHandle>>,
}

impl Rig {
    /// `durable`: where sequenced envelopes go (the "store"). `queue`: producer queue capacity.
    async fn new(durable: mpsc::Sender<Arc<Envelope>>, queue: usize) -> Rig {
        let up = np_harness::start(np_harness::find_free_base()).await;
        let ca = CaMaterial::generate("Conformance CA").unwrap();
        let bus = Bus::spawn("run-test".into(), BusConfig { queue, live_capacity: 256 }, Some(durable));
        let mut cfg = EngineConfig::new("127.0.0.1:0".parse().unwrap(), ca.clone());
        cfg.upstream_roots_pem = Some(up.ca_pem.clone().into_bytes());
        cfg.mock = Arc::new(PocMock);
        let handle = HudsuckerEngine.start(cfg, bus.sink()).await.unwrap();
        Rig { up, proxy: handle.local_addr(), ca_pem: ca.cert_pem, bus, handle: Some(handle) }
    }

    fn url(&self, off: u16, path: &str) -> String {
        format!("https://localhost:{}{}", self.up.base_port + off, path)
    }

    fn client(&self, h2: bool) -> reqwest::Client {
        let cert = reqwest::Certificate::from_pem(self.ca_pem.as_bytes()).unwrap();
        let b = reqwest::Client::builder()
            .add_root_certificate(cert)
            .proxy(reqwest::Proxy::all(format!("http://{}", self.proxy)).unwrap());
        (if h2 { b.http2_prior_knowledge() } else { b.http1_only() }).build().unwrap()
    }

    async fn finish(mut self) {
        tokio::time::sleep(Duration::from_millis(400)).await; // let connection handlers drop
        self.handle.take().unwrap().shutdown().await;
        self.bus.close().await;
    }
}

fn collector() -> (mpsc::Sender<Arc<Envelope>>, tokio::task::JoinHandle<Vec<Arc<Envelope>>>) {
    let (tx, mut rx) = mpsc::channel(4096);
    (tx, tokio::spawn(async move {
        let mut v = vec![];
        while let Some(e) = rx.recv().await {
            v.push(e);
        }
        v
    }))
}

fn assert_contract(events: &[Arc<Envelope>], allow_drops: bool) -> HashMap<FlowId, FlowState> {
    // 1. contiguous seq, single run
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.seq, i as u64 + 1, "seq gap at index {i}");
        assert_eq!(e.run, "run-test");
    }
    if !allow_drops {
        assert!(!events.iter().any(|e| matches!(e.event, Event::EventsDropped { .. })), "unexpected EventsDropped");
    }
    // 2. per-flow state machine
    let mut flows: HashMap<FlowId, FlowState> = HashMap::new();
    for e in events {
        if let Some(f) = e.event.flow() {
            let next = FlowState::apply(flows.get(&f).copied(), &e.event)
                .unwrap_or_else(|v| panic!("flow {f:?} seq {}: {}", e.seq, v.0));
            flows.insert(f, next);
        }
    }
    flows
}

fn flows_by_uri(events: &[Arc<Envelope>]) -> HashMap<String, Vec<FlowId>> {
    let mut m: HashMap<String, Vec<FlowId>> = HashMap::new();
    for e in events {
        if let Event::RequestHead { flow, uri, .. } = &e.event {
            m.entry(uri.clone()).or_default().push(*flow);
        }
    }
    m
}

fn body_end(events: &[Arc<Envelope>], flow: FlowId, dir: BodyDir) -> Option<(u64, Option<String>)> {
    events.iter().find_map(|e| match &e.event {
        Event::BodyEnd { flow: f, dir: d, total, sha256, .. } if *f == flow && *d == dir => Some((*total, sha256.clone())),
        _ => None,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ordered_complete_capture_under_mixed_load() {
    let (dtx, collected) = collector();
    let rig = Rig::new(dtx, 262_144).await;
    let (h1, h2) = (rig.client(false), rig.client(true));
    let mut issued_ok = 0usize;

    // 200 concurrent small requests on each protocol
    for c in [&h1, &h2] {
        let mut hs = vec![];
        for _ in 0..200 {
            let (c, u) = (c.clone(), rig.url(0, "/small"));
            hs.push(tokio::spawn(async move { c.get(u).send().await.unwrap().bytes().await.unwrap().len() }));
        }
        for h in hs {
            assert_eq!(h.await.unwrap(), 18);
            issued_ok += 1;
        }
    }

    // big bodies: proxy-computed sha256 must equal upstream ground truth
    let mut big_truth = vec![];
    for i in 0..4 {
        let url = rig.url(0, &format!("/big?mb={}", 3 + i));
        let r = h2.get(&url).send().await.unwrap();
        let truth = r.headers()["x-np-sha256"].to_str().unwrap().to_owned();
        let body = r.bytes().await.unwrap();
        assert_eq!(hex(&Sha256::digest(&body)), truth, "client received corrupted body");
        big_truth.push((url, truth));
        issued_ok += 1;
    }

    // request body capture: 1 MiB POST
    let payload: Vec<u8> = (0..1_048_576u32).map(|i| (i % 251) as u8).collect();
    let echoed = h1.post(rig.url(0, "/echo")).body(payload.clone()).send().await.unwrap().bytes().await.unwrap();
    assert_eq!(echoed.len(), payload.len());
    issued_ok += 1;
    let payload_sha = hex(&Sha256::digest(&payload));

    // empty-body responses must still reach a terminal state
    let r = h1.get(rig.url(0, "/redirect")).send().await.unwrap(); // follows to /small
    assert_eq!(r.status(), 200);
    issued_ok += 2;

    // mock
    let r = h1.get(rig.url(0, "/__np_mock")).send().await.unwrap();
    assert_eq!(r.headers()["x-np-mock"], "poc-1");
    issued_ok += 1;

    // upstream reset mid-body => aborted flow, not a hang
    let _ = h2.get(rig.url(0, "/reset")).send().await.unwrap().bytes().await;
    issued_ok += 1;

    // client gives up mid-body => aborted
    let slow = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(rig.ca_pem.as_bytes()).unwrap())
        .proxy(reqwest::Proxy::all(format!("http://{}", rig.proxy)).unwrap())
        .http1_only().timeout(Duration::from_millis(500)).build().unwrap();
    let _ = slow.get(rig.url(0, "/slow?ms=5000")).send().await.and_then(|r| futures::executor::block_on(r.bytes()));
    issued_ok += 1;

    // upstream TLS failures
    for (off, want) in [(1, FlowFailClass::TlsUntrustedIssuer), (2, FlowFailClass::TlsHostnameMismatch), (3, FlowFailClass::TlsExpired)] {
        let r = h1.get(rig.url(off, "/small")).send().await.unwrap();
        assert_eq!(r.status(), 502, "port offset {off}");
        let _ = want;
        issued_ok += 1;
    }
    let r = h1.get("https://localhost:1/small").send().await.unwrap();
    assert_eq!(r.status(), 502);
    issued_ok += 1;

    // WebSocket echo through the MITM
    ws_echo(&rig, 20).await;
    issued_ok += 1;

    drop((h1, h2, slow));
    let expected_flows = issued_ok;
    rig.finish().await;
    let events = collected.await.unwrap();

    let flows = assert_contract(&events, false);

    // every flow is terminal
    let open: Vec<_> = flows.iter().filter(|(_, s)| !s.kind.is_terminal()).collect();
    assert!(open.is_empty(), "flows left open: {open:?}");
    assert_eq!(flows.len(), expected_flows, "flow count must equal requests issued");

    // body ground truth
    let by_uri = flows_by_uri(&events);
    for (url, truth) in &big_truth {
        let f = by_uri[url][0];
        let (total, sha) = body_end(&events, f, BodyDir::Res).expect("big body_end");
        assert_eq!(sha.as_deref(), Some(truth.as_str()), "proxy hash != upstream hash for {url}");
        assert!(total >= 3 * 1024 * 1024);
    }
    let post = by_uri[&uri_ending(&events, "/echo")][0];
    assert_eq!(body_end(&events, post, BodyDir::Req).unwrap().1.as_deref(), Some(payload_sha.as_str()));

    // terminal kinds
    let kind_of = |path: &str| flows[&by_uri.iter().find(|(u, _)| u.contains(path)).map(|(_, v)| v[0]).unwrap()].kind;
    assert_eq!(kind_of("/reset"), FlowStateKind::Aborted);
    assert_eq!(kind_of("/slow"), FlowStateKind::Aborted);
    assert_eq!(kind_of("/__np_mock"), FlowStateKind::Completed);
    let fail_classes: Vec<_> = events.iter().filter_map(|e| if let Event::FlowFailed { class, .. } = &e.event { Some(*class) } else { None }).collect();
    for want in [FlowFailClass::TlsUntrustedIssuer, FlowFailClass::TlsHostnameMismatch, FlowFailClass::TlsExpired, FlowFailClass::ConnectRefused] {
        assert!(fail_classes.contains(&want), "missing failure class {want:?} in {fail_classes:?}");
    }
    assert_eq!(events.iter().filter(|e| matches!(e.event, Event::MockHit { .. })).count(), 1);
    assert!(events.iter().filter(|e| matches!(e.event, Event::WsMessage { .. })).count() >= 40, "ws messages both directions");

    // connection accounting: opened == closed, request counts agree
    let mut opened = HashMap::new();
    let mut closed = HashMap::new();
    let mut per_conn: HashMap<ConnId, u64> = HashMap::new();
    for e in &events {
        match &e.event {
            Event::ConnectionOpened { conn, .. } => { opened.insert(*conn, e.seq); }
            Event::ConnectionClosed { conn, requests } => { closed.insert(*conn, *requests); }
            Event::RequestHead { conn: Some(c), .. } => *per_conn.entry(*c).or_default() += 1,
            _ => {}
        }
    }
    assert!(!opened.is_empty());
    for c in opened.keys() {
        let n = closed.get(c).unwrap_or_else(|| panic!("connection {c:?} never closed"));
        assert_eq!(*n, per_conn.get(c).copied().unwrap_or(0), "request count mismatch on {c:?}");
    }
    // lifecycle: engine_started first, engine_stopped last
    assert!(matches!(events.first().unwrap().event, Event::EngineStarted { .. }));
    assert!(matches!(events.last().unwrap().event, Event::EngineStopped { .. }));
}

fn uri_ending(events: &[Arc<Envelope>], path: &str) -> String {
    events.iter().find_map(|e| match &e.event { Event::RequestHead { uri, .. } if uri.ends_with(path) => Some(uri.clone()), _ => None }).unwrap()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

async fn ws_echo(rig: &Rig, n: usize) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::rustls::{pki_types::{pem::PemObject, CertificateDer, ServerName}, ClientConfig, RootCertStore};
    let mut tcp = tokio::net::TcpStream::connect(rig.proxy).await.unwrap();
    let target = format!("localhost:{}", rig.up.base_port);
    tcp.write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n").as_bytes()).await.unwrap();
    let mut buf = [0u8; 256];
    let n0 = tcp.read(&mut buf).await.unwrap();
    assert!(String::from_utf8_lossy(&buf[..n0]).contains("200"));
    let mut roots = RootCertStore::empty();
    for c in CertificateDer::pem_slice_iter(rig.ca_pem.as_bytes()) {
        roots.add(c.unwrap()).unwrap();
    }
    let cfg = ClientConfig::builder_with_provider(Arc::new(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions().unwrap().with_root_certificates(roots).with_no_client_auth();
    let tls = tokio_rustls::TlsConnector::from(Arc::new(cfg))
        .connect(ServerName::try_from("localhost").unwrap(), tcp).await.unwrap();
    let (mut ws, resp) = tokio_tungstenite::client_async(format!("wss://localhost:{}/ws", rig.up.base_port), tls).await.unwrap();
    assert_eq!(resp.status(), 101);
    for i in 0..n {
        ws.send(format!("msg-{i}").into()).await.unwrap();
        let m = ws.next().await.unwrap().unwrap();
        assert_eq!(m.into_text().unwrap().as_str(), format!("msg-{i}"));
    }
    let _ = ws.close(None).await;
}

/// A consumer that never reads (UI hung, store stalled) must not slow or break traffic; the loss
/// is explicit and the sequence stays contiguous.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stalled_consumers_never_affect_traffic() {
    let (dtx, mut drx) = mpsc::channel::<Arc<Envelope>>(1); // stalled "store"
    let rig = Rig::new(dtx, 512).await;
    let mut live = rig.bus.subscribe(); // never polled => lags
    let client = rig.client(false);

    let t = std::time::Instant::now();
    let mut hs = vec![];
    for _ in 0..1500 {
        let (c, u) = (client.clone(), rig.url(0, "/small"));
        hs.push(tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() }));
    }
    for h in hs {
        assert_eq!(h.await.unwrap(), 200, "traffic must succeed while consumers are stalled");
    }
    assert!(t.elapsed() < Duration::from_secs(30));
    assert!(rig.bus.stats().dropped.load(std::sync::atomic::Ordering::Relaxed) > 0, "expected explicit drops");

    drop(client);
    let drain = tokio::spawn(async move {
        let mut v = vec![];
        while let Some(e) = drx.recv().await { v.push(e); }
        v
    });
    let handle_stats = rig.bus.stats();
    rig.finish().await;
    let events = drain.await.unwrap();
    assert_contract(&events, true);

    // conservation: sequenced - markers + counted drops == everything the producers emitted
    let marked: u64 = events.iter().map(|e| if let Event::EventsDropped { count } = e.event { count } else { 0 }).sum();
    assert_eq!(marked, handle_stats.dropped.load(std::sync::atomic::Ordering::Relaxed), "every dropped event is accounted for");

    // the live subscriber was told it lagged instead of silently losing events
    let mut lagged = false;
    loop {
        match live.try_recv() {
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => { lagged = true; break; }
            Err(_) => break,
            Ok(_) => {}
        }
    }
    assert!(lagged, "slow live subscriber must observe Lagged");
}
