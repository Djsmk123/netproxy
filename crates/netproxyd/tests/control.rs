use futures::{SinkExt, StreamExt};
use netproxyd::{
    control,
    daemon::{Daemon, DaemonConfig},
};
use np_engine::HudsuckerEngine;
use np_model::*;
use serde_json::{json, Value};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc, time::Duration};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

struct Rig {
    _dir: tempfile::TempDir,
    daemon: Arc<Daemon>,
    ctl: control::ControlServer,
    up: np_harness::Upstream,
}

async fn rig() -> Rig {
    rig_with(|_| {}).await
}

async fn rig_with(f: impl FnOnce(&mut DaemonConfig)) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let up = np_harness::start(np_harness::find_free_base()).await;
    let mut cfg = DaemonConfig::new(dir.path().to_path_buf());
    f(&mut cfg);
    cfg.default_listen = "127.0.0.1:0".parse().unwrap();
    cfg.upstream_roots_pem = Some(up.ca_pem.clone().into_bytes());
    let daemon = Daemon::new(cfg, Arc::new(HudsuckerEngine)).unwrap();
    let ctl = control::serve(daemon.clone(), &dir.path().join("run"))
        .await
        .unwrap();
    Rig {
        _dir: dir,
        daemon,
        ctl,
        up,
    }
}

struct Client {
    f: Framed<UnixStream, LinesCodec>,
    next: u64,
    pending: std::collections::VecDeque<Value>,
}

impl Client {
    async fn raw(socket: &PathBuf) -> Client {
        Client {
            f: Framed::new(
                UnixStream::connect(socket).await.unwrap(),
                LinesCodec::new_with_max_length(16 << 20),
            ),
            next: 0,
            pending: Default::default(),
        }
    }
    async fn connect(r: &Rig) -> Client {
        let mut c = Self::raw(&r.ctl.info.socket).await;
        let h = c.call("hello", json!({"token": r.ctl.info.token, "api_version": API_VERSION, "client": {"name": "test"}})).await.unwrap();
        assert_eq!(h["api_version"], API_VERSION);
        c
    }
    async fn line(&mut self) -> Option<Value> {
        match tokio::time::timeout(Duration::from_secs(20), self.f.next())
            .await
            .expect("timed out waiting for server")
        {
            Some(Ok(l)) => Some(serde_json::from_str(&l).unwrap()),
            _ => None,
        }
    }
    /// Sends a request and returns its result; notifications received meanwhile go to `pending`.
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, Value> {
        self.next += 1;
        let id = self.next;
        self.f
            .send(
                json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
            )
            .await
            .unwrap();
        loop {
            let v = self.line().await.expect("connection closed");
            if v.get("id") == Some(&json!(id)) {
                return if v.get("error").is_some() {
                    Err(v["error"].clone())
                } else {
                    Ok(v["result"].clone())
                };
            }
            self.pending.push_back(v); // a notification that arrived while waiting for the response
        }
    }
    async fn event(&mut self) -> Envelope {
        loop {
            let v = match self.pending.pop_front() {
                Some(v) => v,
                None => self.line().await.expect("stream closed"),
            };
            if v["method"] == "event" {
                return serde_json::from_value(v["params"]["event"].clone()).unwrap();
            }
        }
    }
}

async fn is_closed_silently(socket: &PathBuf, first: &str) -> bool {
    let mut c = Client::raw(socket).await;
    let _ = c.f.send(first.to_string()).await;
    matches!(
        tokio::time::timeout(Duration::from_secs(8), c.f.next()).await,
        Ok(None) | Ok(Some(Err(_)))
    )
}

#[tokio::test]
async fn control_endpoint_is_private_and_authenticated() {
    let r = rig().await;
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&r.ctl.info.socket), 0o600);
    assert_eq!(mode(&r.ctl.info.token_file), 0o600);
    assert_eq!(mode(r.ctl.info.socket.parent().unwrap()), 0o700);
    assert_eq!(
        std::fs::read_to_string(&r.ctl.info.token_file).unwrap(),
        r.ctl.info.token
    );

    let s = &r.ctl.info.socket;
    // no hello first => closed, no reply (does not even reveal that the method exists)
    assert!(is_closed_silently(s, r#"{"jsonrpc":"2.0","id":1,"method":"engine.status"}"#).await);
    assert!(is_closed_silently(s, r#"{"jsonrpc":"2.0","id":1,"method":"hello","params":{"token":"wrong","api_version":1}}"#).await);
    assert!(
        is_closed_silently(
            s,
            r#"{"jsonrpc":"2.0","id":1,"method":"hello","params":{"api_version":1}}"#
        )
        .await
    );
    assert!(is_closed_silently(s, "not json at all").await);
    // oversized first line
    assert!(is_closed_silently(s, &"x".repeat((1 << 20) + 10)).await);
    // correct token, wrong API version => explicit error, then closed
    let mut c = Client::raw(s).await;
    let e = c
        .call(
            "hello",
            json!({"token": r.ctl.info.token, "api_version": 99}),
        )
        .await
        .unwrap_err();
    assert_eq!(e["code"], -32002);
    assert!(c.line().await.is_none());
    // silent client is dropped after the handshake deadline
    let mut idle = Client::raw(s).await;
    assert!(tokio::time::timeout(Duration::from_secs(8), idle.f.next())
        .await
        .unwrap()
        .is_none());

    // authenticated client works; bad method / params produce JSON-RPC errors, not disconnects
    let mut c = Client::connect(&r).await;
    assert_eq!(c.call("ping", Value::Null).await.unwrap()["pong"], true);
    assert_eq!(
        c.call("nope", Value::Null).await.unwrap_err()["code"],
        -32601
    );
    assert_eq!(
        c.call("flows.get", json!({"flow": "x"})).await.unwrap_err()["code"],
        -32602
    );
    assert_eq!(
        c.call("engine.stop", Value::Null).await.unwrap_err()["code"],
        -32000
    );
    r.ctl.stop().await;
}

fn proxy_client(proxy: &str, ca: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(ca.as_bytes()).unwrap())
        .proxy(reqwest::Proxy::all(format!("http://{proxy}")).unwrap())
        .build()
        .unwrap()
}

async fn traffic(r: &Rig, listen: &str, n: usize, path: &str) {
    let c = proxy_client(listen, r.daemon.ca_cert_pem());
    let url = format!("https://localhost:{}{}", r.up.base_port, path);
    let mut hs = vec![];
    for _ in 0..n {
        let (c, u) = (c.clone(), url.clone());
        hs.push(tokio::spawn(async move {
            c.get(u).send().await.unwrap().bytes().await.unwrap().len()
        }));
    }
    for h in hs {
        h.await.unwrap();
    }
}

fn assert_contiguous(evs: &[Envelope], from: u64) {
    for (i, e) in evs.iter().enumerate() {
        assert_eq!(e.seq, from + i as u64, "gap/dup/reorder at position {i}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn capture_through_the_socket_end_to_end() {
    let r = rig().await;
    let mut c = Client::connect(&r).await;
    let st = c.call("engine.start", Value::Null).await.unwrap();
    assert_eq!(st["state"], "running");
    let listen = st["listen"].as_str().unwrap().to_owned();
    let run = st["run"].as_str().unwrap().to_owned();
    assert_eq!(
        c.call("engine.start", Value::Null).await.unwrap_err()["code"],
        -32000,
        "double start refused"
    );

    // tail subscription from the beginning of this run
    let sub = c
        .call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();
    assert_eq!(sub["run"], run);

    traffic(&r, &listen, 300, "/small").await;

    // read the live stream until all 300 flows are complete
    let mut evs: Vec<Envelope> = vec![];
    let mut completed = 0;
    while completed < 300 {
        let e = c.event().await;
        if matches!(e.event, Event::FlowCompleted { .. }) {
            completed += 1;
        }
        evs.push(e);
    }
    assert_contiguous(&evs, 1);
    assert!(matches!(evs[0].event, Event::EngineStarted { .. }));

    // queries agree with the stream
    let l = c
        .call(
            "flows.list",
            json!({"filter": {"run": "current", "state": "completed"}, "limit": 100}),
        )
        .await
        .unwrap();
    assert_eq!(l["flows"].as_array().unwrap().len(), 100);
    assert!(l["next"].is_object());
    let first = &l["flows"][0];
    let g = c
        .call("flows.get", json!({"run": run, "flow": first["flow"]}))
        .await
        .unwrap();
    assert_eq!(g["flow"]["state"], "completed");
    assert!(g["events"].as_array().unwrap().len() >= 4);
    let mid = evs[100].seq;
    let since = c
        .call("events.since", json!({"seq": mid, "limit": 1000}))
        .await
        .unwrap();
    let since: Vec<Envelope> = serde_json::from_value(since["events"].clone()).unwrap();
    assert_contiguous(&since, mid + 1);
    assert_eq!(
        since[0], evs[101],
        "events.since returns the same events the stream did"
    );

    // stop => stream ends with engine_stopped; run is marked stopped
    let stopped = c.call("engine.stop", Value::Null).await.unwrap();
    assert_eq!(stopped["state"], "stopped");
    loop {
        let e = c.event().await;
        let last = matches!(e.event, Event::EngineStopped { .. });
        evs.push(e);
        if last {
            break;
        }
    }
    assert_contiguous(&evs, 1);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let runs = c.call("runs.list", Value::Null).await.unwrap();
    assert_eq!(runs[0]["status"], "stopped");
    assert_eq!(runs[0]["events"].as_u64().unwrap() as usize, evs.len());
    assert_eq!(runs[0]["flows"], 300);

    // diagnostics history comes from the partial index, oldest first, only rare kinds
    let diag = c
        .call("events.diagnostics", json!({"run": run}))
        .await
        .unwrap();
    let diag: Vec<Envelope> = serde_json::from_value(diag["events"].clone()).unwrap();
    assert!(!diag.is_empty());
    assert!(diag.windows(2).all(|w| w[0].seq < w[1].seq));
    assert!(diag.iter().all(|e| matches!(
        e.event,
        Event::TlsHello { .. }
            | Event::ConnectionOpened { .. }
            | Event::ConnectionClosed { .. }
            | Event::EventsDropped { .. }
            | Event::ClientTlsFailed { .. }
    )));

    // storage accounting + manual prune over the API
    let ss = c.call("storage.status", Value::Null).await.unwrap();
    assert!(ss["used_bytes"].as_u64().unwrap() > 100_000);
    let rep = c
        .call("storage.prune", json!({"max_bytes": 1u64 << 40}))
        .await
        .unwrap();
    assert_eq!(rep["runs_deleted"], 0, "nothing to prune under the cap");

    // restart => a new run; the old one stays queryable
    let st2 = c.call("engine.start", Value::Null).await.unwrap();
    assert_ne!(st2["run"], run);
    c.call("engine.stop", Value::Null).await.unwrap();
    assert_eq!(
        c.call("runs.list", Value::Null)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    r.ctl.stop().await;
}

/// A client that stops reading must not slow capture, and a reconnecting client must see exactly
/// the same gapless history.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slow_subscriber_is_paced_and_reconnect_resumes_exactly() {
    let r = rig().await;
    let mut a = Client::connect(&r).await;
    let st = a.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap().to_owned();
    a.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();

    // `a` never reads while 2000 requests flow (several MB of event JSON queue up behind its socket)
    let t = std::time::Instant::now();
    traffic(&r, &listen, 2000, "/small").await;
    assert!(
        t.elapsed() < Duration::from_secs(30),
        "capture must not be slowed by a non-reading client"
    );
    assert_eq!(
        r.daemon.status().dropped,
        0,
        "no events may be dropped because a client is slow"
    );

    // a second client connects late and replays from the start
    let mut b = Client::connect(&r).await;
    b.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();
    let mut from_b = vec![];
    let mut done = 0;
    while done < 2000 {
        let e = b.event().await;
        if matches!(e.event, Event::FlowCompleted { .. }) {
            done += 1;
        }
        from_b.push(e);
    }
    assert_contiguous(&from_b, 1);

    // the stalled client finally reads: identical stream, nothing lost or reordered
    let mut from_a = vec![];
    while from_a.len() < from_b.len() {
        from_a.push(a.event().await);
    }
    assert_eq!(from_a, from_b);

    // reconnect + resume from the middle yields the exact suffix
    let cut = from_b[from_b.len() / 2].seq;
    drop(a);
    let mut c = Client::connect(&r).await;
    c.call("events.subscribe", json!({"since": cut}))
        .await
        .unwrap();
    let mut resumed = vec![];
    while resumed.len() < from_b.len() - cut as usize {
        resumed.push(c.event().await);
    }
    assert_eq!(resumed, from_b[cut as usize..].to_vec());

    // unsubscribe stops delivery
    let sub = c.call("events.subscribe", Value::Null).await.unwrap()["subscription"].clone();
    assert_eq!(
        c.call("events.unsubscribe", json!({"subscription": sub}))
            .await
            .unwrap()["unsubscribed"],
        true
    );
    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

async fn fetch_body(c: &mut Client, run: &str, flow: u64, dir: &str) -> (Vec<u8>, Value) {
    use base64::Engine as _;
    let (mut out, mut off) = (vec![], 0u64);
    loop {
        let r = c
            .call(
                "bodies.get",
                json!({"run": run, "flow": flow, "dir": dir, "offset": off, "length": 256 * 1024}),
            )
            .await
            .unwrap();
        if r["evicted"] == true {
            return (vec![], r);
        }
        let chunk = base64::engine::general_purpose::STANDARD
            .decode(r["data_b64"].as_str().unwrap())
            .unwrap();
        off += chunk.len() as u64;
        out.extend(chunk);
        if r["eof"] == true {
            return (out, r);
        }
    }
}

fn find_body_end(evs: &[Envelope], flow: u64, dir: BodyDir) -> (u64, Option<String>, u64, bool) {
    evs.iter()
        .find_map(|e| match &e.event {
            Event::BodyEnd {
                flow: f,
                dir: d,
                total,
                sha256,
                stored_bytes,
                truncated,
                ..
            } if f.0 == flow && *d == dir => {
                Some((*total, sha256.clone(), *stored_bytes, *truncated))
            }
            _ => None,
        })
        .expect("body_end")
}

async fn all_events(c: &mut Client, run: &str) -> Vec<Envelope> {
    let r = c
        .call("events.since", json!({"run": run, "seq": 0, "limit": 1000}))
        .await
        .unwrap();
    serde_json::from_value(r["events"].clone()).unwrap()
}

async fn flow_for(c: &mut Client, path_suffix: &str) -> u64 {
    let l = c
        .call(
            "flows.list",
            json!({"filter": {"run": "current", "text": path_suffix}, "limit": 5}),
        )
        .await
        .unwrap();
    l["flows"][0]["flow"].as_u64().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bodies_are_stored_verifiably_capped_and_evictable() {
    use sha2::{Digest, Sha256};
    let r = rig_with(|c| {
        c.bodies = Some(netproxyd::daemon::BodyConfig {
            per_body_cap: 1 << 20,
            total_cap: 1 << 30,
        })
    })
    .await;
    let mut c = Client::connect(&r).await;
    let st = c.call("engine.start", Value::Null).await.unwrap();
    let (listen, run) = (
        st["listen"].as_str().unwrap().to_owned(),
        st["run"].as_str().unwrap().to_owned(),
    );
    let client = proxy_client(&listen, r.daemon.ca_cert_pem());
    let base = format!("https://localhost:{}", r.up.base_port);

    // small body: stored whole, bytes and hash identical to what the client received
    let small = client
        .get(format!("{base}/small"))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    // request body round-trip
    let payload: Vec<u8> = (0..600_000u32).map(|i| (i % 251) as u8).collect();
    let echoed = client
        .post(format!("{base}/echo"))
        .body(payload.clone())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(echoed.len(), payload.len());
    // 3 MiB body: exceeds the 1 MiB per-body cap => truncated prefix
    let big = client.get(format!("{base}/big?mb=3")).send().await.unwrap();
    let big_truth = big.headers()["x-np-sha256"].to_str().unwrap().to_owned();
    let big_body = big.bytes().await.unwrap();
    assert_eq!(hex(&Sha256::digest(&big_body)), big_truth);
    drop(client);
    tokio::time::sleep(Duration::from_millis(800)).await; // let the body writer thread flush

    let evs = all_events(&mut c, &run).await;

    let f_small = flow_for(&mut c, "/small").await;
    let (total, sha, stored, trunc) = find_body_end(&evs, f_small, BodyDir::Res);
    let (bytes, meta) = fetch_body(&mut c, &run, f_small, "res").await;
    assert_eq!(
        (bytes.as_slice(), total, stored, trunc),
        (
            small.as_ref(),
            small.len() as u64,
            small.len() as u64,
            false
        )
    );
    assert_eq!(hex(&Sha256::digest(&bytes)), sha.unwrap());
    assert_eq!(meta["size"], small.len());

    let f_echo = flow_for(&mut c, "/echo").await;
    let (_, req_sha, req_stored, req_trunc) = find_body_end(&evs, f_echo, BodyDir::Req);
    let (req_bytes, _) = fetch_body(&mut c, &run, f_echo, "req").await;
    assert_eq!(
        req_bytes, payload,
        "request body stored byte-exact (600 KB read back in 256 KiB ranges)"
    );
    assert_eq!((req_stored, req_trunc), (payload.len() as u64, false));
    assert_eq!(hex(&Sha256::digest(&req_bytes)), req_sha.unwrap());

    let f_big = flow_for(&mut c, "/big").await;
    let (total, sha, stored, trunc) = find_body_end(&evs, f_big, BodyDir::Res);
    let (prefix, _) = fetch_body(&mut c, &run, f_big, "res").await;
    assert_eq!(total, 3 << 20);
    assert_eq!(
        sha.as_deref(),
        Some(big_truth.as_str()),
        "hash covers the full body even though storage is capped"
    );
    assert_eq!((stored, trunc), (1 << 20, true));
    assert_eq!(
        prefix.as_slice(),
        &big_body[..1 << 20],
        "stored bytes are an exact prefix"
    );

    // total cap: shrinking it evicts the oldest files; readers get `evicted`, not an error
    c.call("engine.stop", Value::Null).await.unwrap();
    let freed = netproxyd::bodies::enforce_cap(&r.daemon.bodies_dir(), 1000, &|_| true).unwrap();
    assert!(freed > 1_000_000);
    let (gone, meta) = fetch_body(&mut c, &run, f_big, "res").await;
    assert!(gone.is_empty() && meta["evicted"] == true);
    // unknown runs' directories are removed
    let freed =
        netproxyd::bodies::enforce_cap(&r.daemon.bodies_dir(), u64::MAX, &|_| false).unwrap();
    let _ = freed;
    assert!(!r.daemon.bodies_dir().join(&run).exists());
    r.ctl.stop().await;
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_setup_pages_and_batched_flow_refresh() {
    let r = rig().await;
    let mut c = Client::connect(&r).await;
    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap().to_owned();
    let run = st["run"].as_str().unwrap().to_owned();
    // plain HTTP through the proxy, exactly as a phone would do before trusting the CA
    let plain = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{listen}")).unwrap())
        .build()
        .unwrap();
    let page = plain.get("http://np.pro/").send().await.unwrap();
    assert_eq!(page.status(), 200);
    assert!(page.text().await.unwrap().contains("ca.mobileconfig"));
    let pem = plain
        .get("http://np.pro/ca.pem")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(pem, r.daemon.ca_cert_pem(), "served CA is the daemon's CA");
    let der = plain.get("http://np.pro/ca.crt").send().await.unwrap();
    assert_eq!(der.headers()["content-type"], "application/x-x509-ca-cert");
    assert_eq!(der.bytes().await.unwrap()[0], 0x30);
    let prof = plain
        .get("http://np.pro/ca.mobileconfig")
        .send()
        .await
        .unwrap();
    assert_eq!(
        prof.headers()["content-type"],
        "application/x-apple-aspen-config"
    );
    assert!(prof
        .text()
        .await
        .unwrap()
        .contains("com.apple.security.root"));
    assert_eq!(
        plain
            .get("http://np.pro/nope")
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    // the pages are visible in the traffic list as mocked flows from rule np-setup
    // (persistence is asynchronous, so poll briefly instead of assuming it is instant)
    let mut l = Value::Null;
    for _ in 0..60 {
        l = c
            .call(
                "flows.list",
                json!({"filter": {"run": "current", "mocked": true}, "limit": 50}),
            )
            .await
            .unwrap();
        if l["flows"].as_array().map_or(0, |a| {
            a.iter().filter(|f| f["state"] == "completed").count()
        }) == 5
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let flows = l["flows"].as_array().unwrap();
    assert_eq!(flows.len(), 5);
    assert!(flows.iter().all(|f| f["rule"] == "np-setup"));

    // batched refresh
    let ids: Vec<u64> = flows.iter().map(|f| f["flow"].as_u64().unwrap()).collect();
    let many = c
        .call(
            "flows.get_many",
            json!({"run": run, "flows": ids.iter().chain(&[9999]).collect::<Vec<_>>()}),
        )
        .await
        .unwrap();
    assert_eq!(
        many["flows"].as_array().unwrap().len(),
        ids.len(),
        "unknown ids are skipped"
    );
    assert_eq!(
        c.call(
            "flows.get_many",
            json!({"flows": (0..501).collect::<Vec<u64>>()})
        )
        .await
        .unwrap_err()["code"],
        -32602
    );
    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn second_daemon_on_same_data_dir_is_refused_and_does_not_disturb_the_first() {
    let r = rig().await;
    let before = std::fs::read_to_string(&r.ctl.info.token_file).unwrap();
    let dir = r.ctl.info.socket.parent().unwrap().to_path_buf();
    let cfg = DaemonConfig::new(dir.parent().unwrap().to_path_buf());
    let second = Daemon::new(cfg, Arc::new(HudsuckerEngine)).unwrap();
    let e = control::serve(second, &dir)
        .await
        .err()
        .expect("second serve must fail");
    assert_eq!(e.kind(), std::io::ErrorKind::AddrInUse);
    assert_eq!(
        std::fs::read_to_string(&r.ctl.info.token_file).unwrap(),
        before,
        "token untouched"
    );
    let mut c = Client::connect(&r).await; // first daemon still serves
    assert_eq!(c.call("ping", Value::Null).await.unwrap()["pong"], true);
    r.ctl.stop().await;
    // after a clean stop the directory can be claimed again
    let third = Daemon::new(
        DaemonConfig::new(dir.parent().unwrap().to_path_buf()),
        Arc::new(HudsuckerEngine),
    )
    .unwrap();
    control::serve(third, &dir).await.unwrap().stop().await;
}

#[tokio::test]
async fn long_data_dir_paths_fall_back_to_a_short_private_socket() {
    // A data dir deep enough that `<dir>/run/control.sock` exceeds the 104-byte sockaddr_un limit.
    let base = tempfile::tempdir().unwrap();
    let deep = base
        .path()
        .join("a".repeat(40))
        .join("b".repeat(40))
        .join("c".repeat(30));
    let cfg = DaemonConfig::new(deep.clone());
    let d = Daemon::new(cfg, Arc::new(HudsuckerEngine)).unwrap();
    let ctl = control::serve(d, &deep.join("run"))
        .await
        .expect("must not fail on long paths");
    assert!(
        deep.join("run/control.sock").as_os_str().len() > 100,
        "fixture must be too long for sockaddr_un"
    );
    assert!(
        ctl.info.socket.as_os_str().len() <= 100,
        "real socket path is short: {:?}",
        ctl.info.socket
    );
    assert!(ctl
        .info
        .socket
        .starts_with(format!("/tmp/np-{}", unsafe { libc::geteuid() })));
    assert_eq!(
        std::fs::read_to_string(&ctl.info.endpoint_file).unwrap(),
        ctl.info.socket.to_string_lossy()
    );
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&ctl.info.socket), 0o600);
    assert_eq!(mode(ctl.info.socket.parent().unwrap()), 0o700);
    // and it actually works through the advertised endpoint
    let mut c = Client::raw(&ctl.info.socket).await;
    c.call("hello", json!({"token": ctl.info.token, "api_version": 1}))
        .await
        .unwrap();
    assert_eq!(c.call("ping", Value::Null).await.unwrap()["pong"], true);
    let sock = ctl.info.socket.clone();
    ctl.stop().await;
    assert!(!sock.exists(), "short socket removed on stop");
}
