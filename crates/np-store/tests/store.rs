use np_model::*;
use np_store::*;
use std::sync::Arc;
use tokio::sync::mpsc;

struct Seq {
    run: String,
    n: u64,
    out: Vec<Arc<Envelope>>,
}

impl Seq {
    fn new(run: &str) -> Self {
        Seq {
            run: run.into(),
            n: 0,
            out: vec![],
        }
    }
    fn push(&mut self, ts: u64, event: Event) {
        self.n += 1;
        self.out.push(Arc::new(Envelope {
            run: self.run.clone(),
            seq: self.n,
            ts_us: ts,
            event,
        }));
    }
    fn start(&mut self, ts: u64) {
        self.push(
            ts,
            Event::EngineStarted {
                listen: "127.0.0.1:1".into(),
                engine: "t".into(),
                engine_version: "0".into(),
            },
        );
    }
    fn req(&mut self, ts: u64, flow: u64, host: &str, path: &str, method: &str) {
        self.push(
            ts,
            Event::RequestHead {
                flow: FlowId(flow),
                conn: Some(ConnId(1)),
                client: "c".into(),
                method: method.into(),
                uri: format!("https://{host}{path}"),
                host: Some(host.into()),
                http_version: "HTTP/1.1".into(),
                headers: vec![],
            },
        );
    }
    fn ok(&mut self, ts: u64, flow: u64, status: u16, bytes: u64) {
        let f = FlowId(flow);
        self.push(
            ts,
            Event::ResponseHead {
                flow: f,
                status,
                ttfb_us: 5,
                headers: vec![],
                mocked: false,
            },
        );
        self.push(
            ts,
            Event::BodyEnd {
                flow: f,
                dir: BodyDir::Res,
                total: bytes,
                chunks: 1,
                sha256: Some("ab".into()),
                preview_b64: String::new(),
                stored_bytes: 0,
                truncated: false,
            },
        );
        self.push(
            ts,
            Event::FlowCompleted {
                flow: f,
                status: Some(status),
                mocked: false,
            },
        );
    }
}

async fn ingest(store: &Store, events: Vec<Arc<Envelope>>) {
    let (tx, rx) = mpsc::channel(1024);
    let w = store.spawn_writer(rx).unwrap();
    for e in events {
        tx.send(e).await.unwrap();
    }
    drop(tx);
    tokio::task::spawn_blocking(move || w.join()).await.unwrap();
}

#[tokio::test]
async fn projection_matches_event_stream() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("t.db")).unwrap();
    let mut s = Seq::new("r1");
    s.start(1_000);
    s.req(1_100, 1, "api.example.com", "/v1/users?id=3", "GET");
    s.ok(1_200, 1, 200, 321);
    s.req(
        1_300,
        2,
        "api.example.com",
        "/v1/pay",
        "post".to_uppercase().as_str(),
    );
    s.push(
        1_400,
        Event::FlowFailed {
            flow: FlowId(2),
            class: FlowFailClass::TlsExpired,
            detail: "expired".into(),
        },
    );
    s.req(1_500, 3, "cdn.example.com", "/big", "GET");
    s.push(
        1_510,
        Event::ResponseHead {
            flow: FlowId(3),
            status: 200,
            ttfb_us: 9,
            headers: vec![],
            mocked: false,
        },
    );
    s.push(
        1_600,
        Event::BodyAborted {
            flow: FlowId(3),
            dir: BodyDir::Res,
            total: 77,
        },
    );
    s.req(1_700, 4, "api.example.com", "/mock", "GET");
    s.push(
        1_710,
        Event::MockHit {
            flow: FlowId(4),
            rule: "r-1".into(),
        },
    );
    s.push(
        1_720,
        Event::ResponseHead {
            flow: FlowId(4),
            status: 200,
            ttfb_us: 0,
            headers: vec![],
            mocked: true,
        },
    );
    s.push(
        1_730,
        Event::FlowCompleted {
            flow: FlowId(4),
            status: Some(200),
            mocked: true,
        },
    );
    s.req(1_800, 5, "api.example.com", "/pending", "GET"); // still open
    s.push(
        2_000,
        Event::EngineStopped {
            reason: "shutdown".into(),
        },
    );
    let n_events = s.out.len();
    ingest(&store, s.out).await;

    let r = store.reader().unwrap();
    let f = |n| r.get_flow("r1", n).unwrap().unwrap();
    assert_eq!(
        (
            f(1).state.as_str(),
            f(1).status,
            f(1).res_bytes,
            f(1).path.as_deref()
        ),
        ("completed", Some(200), 321, Some("/v1/users"))
    );
    assert_eq!(f(1).res_sha256.as_deref(), Some("ab"));
    assert_eq!(
        (f(2).state.as_str(), f(2).fail_class.as_deref()),
        ("failed", Some("tls_expired"))
    );
    assert_eq!((f(3).state.as_str(), f(3).res_bytes), ("aborted", 77));
    assert!(f(4).mocked && f(4).rule.as_deref() == Some("r-1") && f(4).state == "completed");
    assert_eq!(f(5).state, "open");
    assert_eq!(f(1).first_seq, 2);

    let run = &r.runs().unwrap()[0];
    assert_eq!(
        (
            run.status.as_str(),
            run.ended_us,
            run.events as usize,
            run.flows
        ),
        ("stopped", Some(2_000), n_events, 5)
    );
    assert_eq!(r.last_seq("r1").unwrap() as usize, n_events);

    // events_since resumes exactly
    let tail = r.events_since("r1", (n_events - 2) as u64, 100).unwrap();
    assert_eq!(
        tail.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![n_events as u64 - 1, n_events as u64]
    );
    // per-flow event history round-trips through JSON
    assert_eq!(r.flow_events("r1", 2).unwrap().len(), 2);
}

#[tokio::test]
async fn interrupted_run_is_repaired_on_open() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("t.db");
    {
        let store = Store::open(&p).unwrap();
        let mut s = Seq::new("crashed");
        s.start(10);
        s.req(20, 1, "h", "/a", "GET");
        s.ok(30, 1, 200, 1);
        s.req(40, 2, "h", "/hang", "GET"); // process died here: no EngineStopped
        ingest(&store, s.out).await;
    }
    let store = Store::open(&p).unwrap(); // "relaunch"
    let r = store.reader().unwrap();
    let run = &r.runs().unwrap()[0];
    assert_eq!(run.status, "interrupted");
    assert!(run.ended_us.is_some());
    let hang = r.get_flow("crashed", 2).unwrap().unwrap();
    assert_eq!(
        (hang.state.as_str(), hang.fail_detail.as_deref()),
        ("aborted", Some("session interrupted"))
    );
    assert_eq!(
        r.get_flow("crashed", 1).unwrap().unwrap().state,
        "completed"
    );
}

#[tokio::test]
async fn filters_and_keyset_pagination() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("t.db")).unwrap();
    let mut s = Seq::new("r");
    s.start(1);
    for i in 1..=250u64 {
        let host = if i % 2 == 0 { "a.com" } else { "b.com" };
        s.req(
            100 + i / 5,
            i,
            host,
            &format!("/p/{i}"),
            if i % 10 == 0 { "POST" } else { "GET" },
        ); // timestamps tie on purpose
        s.ok(100 + i / 5, i, if i % 7 == 0 { 404 } else { 200 }, i);
    }
    ingest(&store, s.out).await;
    let r = store.reader().unwrap();

    // full walk in pages of 40: every flow exactly once, newest first, no duplicates despite ties
    let mut seen = vec![];
    let mut cur = None;
    loop {
        let (rows, next) = r.list_flows(&FlowFilter::default(), 40, cur).unwrap();
        seen.extend(rows.iter().map(|x| x.flow));
        match next {
            Some(n) => cur = Some(n),
            None => break,
        }
    }
    assert_eq!(seen.len(), 250);
    assert_eq!(seen[0], 250);
    let mut uniq = seen.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), 250);
    assert!(
        seen.windows(2).all(|w| w[0] > w[1]),
        "ordering newest-first"
    );

    let n = |f: FlowFilter| r.list_flows(&f, 1000, None).unwrap().0.len();
    assert_eq!(
        n(FlowFilter {
            host: Some("a.com".into()),
            ..Default::default()
        }),
        125
    );
    assert_eq!(
        n(FlowFilter {
            method: Some("post".into()),
            ..Default::default()
        }),
        25
    );
    assert_eq!(
        n(FlowFilter {
            status: Some(404),
            ..Default::default()
        }),
        35
    );
    assert_eq!(
        n(FlowFilter {
            status_class: Some(2),
            ..Default::default()
        }),
        215
    );
    assert_eq!(
        n(FlowFilter {
            text: Some("/P/2".into()),
            ..Default::default()
        }),
        1 + 10 + 51
    ); // /p/2, /p/20..29, /p/200..250
    assert_eq!(
        n(FlowFilter {
            text: Some("100%".into()),
            ..Default::default()
        }),
        0,
        "LIKE wildcards are escaped"
    );
    assert_eq!(
        n(FlowFilter {
            state: Some("completed".into()),
            mocked: Some(false),
            ..Default::default()
        }),
        250
    );
}

/// Plan target: 100,000 flows, p95 indexed query < 250 ms. Run: cargo test -p np-store --release -- --ignored
#[tokio::test]
#[ignore]
async fn hundred_thousand_flows_query_latency() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("big.db")).unwrap();
    let mut s = Seq::new("big");
    s.start(1);
    for i in 1..=100_000u64 {
        s.req(
            1_000 + i,
            i,
            &format!("host{}.example.com", i % 50),
            &format!("/api/v1/items/{i}"),
            "GET",
        );
        s.ok(1_000 + i, i, if i % 13 == 0 { 500 } else { 200 }, i % 4096);
    }
    let events = s.out.len();
    let t = std::time::Instant::now();
    ingest(&store, s.out).await;
    eprintln!("ingested {events} events / 100k flows in {:?}", t.elapsed());

    let r = store.reader().unwrap();
    let queries: Vec<(&str, FlowFilter)> = vec![
        ("latest page", FlowFilter::default()),
        (
            "host",
            FlowFilter {
                host: Some("host7.example.com".into()),
                ..Default::default()
            },
        ),
        (
            "status 500",
            FlowFilter {
                status: Some(500),
                ..Default::default()
            },
        ),
        (
            "5xx class",
            FlowFilter {
                status_class: Some(5),
                ..Default::default()
            },
        ),
        (
            "time window",
            FlowFilter {
                since_us: Some(50_000),
                until_us: Some(60_000),
                ..Default::default()
            },
        ),
        (
            "text",
            FlowFilter {
                text: Some("items/99999".into()),
                ..Default::default()
            },
        ),
        (
            "host+status",
            FlowFilter {
                host: Some("host3.example.com".into()),
                status: Some(500),
                ..Default::default()
            },
        ),
    ];
    let mut worst = std::time::Duration::ZERO;
    for (name, q) in &queries {
        let mut times = vec![];
        for _ in 0..20 {
            let t = std::time::Instant::now();
            let _ = r.list_flows(q, 100, None).unwrap();
            times.push(t.elapsed());
        }
        times.sort();
        let p95 = times[18];
        eprintln!("{name:12} p50={:?} p95={:?}", times[10], p95);
        worst = worst.max(p95);
    }
    assert!(
        worst < std::time::Duration::from_millis(250),
        "p95 {worst:?} exceeds the 250 ms target"
    );
}

#[tokio::test]
async fn retention_bounds_disk_and_keeps_data_consistent() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("t.db")).unwrap();
    // two finished sessions and one active session
    for (run, base) in [("old1", 1_000u64), ("old2", 2_000), ("live", 3_000)] {
        let mut s = Seq::new(run);
        s.start(base);
        for i in 1..=6_000u64 {
            s.req(base + i, i, "h.example.com", &format!("/p/{i}"), "GET");
            s.ok(base + i, i, 200, 10);
        }
        if run != "live" {
            s.push(base + 9_999, Event::EngineStopped { reason: "t".into() });
        }
        ingest(&store, s.out).await;
    }
    let before = store.used_bytes().unwrap();
    assert!(
        before > 4_000_000,
        "fixture should be a few MB, was {before}"
    );

    // 1) bound that requires dropping whole old sessions only
    let target = before * 2 / 3;
    let rep = store
        .prune(&PrunePolicy { max_bytes: target }, Some("live"))
        .unwrap();
    assert!(
        rep.runs_deleted >= 1 && store.used_bytes().unwrap() <= target,
        "{rep:?}"
    );
    let r = store.reader().unwrap();
    let runs: Vec<String> = r.runs().unwrap().into_iter().map(|x| x.run).collect();
    assert!(
        !runs.contains(&"old1".to_string()),
        "oldest session goes first: {runs:?}"
    );
    assert!(runs.contains(&"live".to_string()));
    assert_eq!(
        r.list_flows(
            &FlowFilter {
                run: Some("old1".into()),
                ..Default::default()
            },
            10,
            None
        )
        .unwrap()
        .0
        .len(),
        0,
        "no orphan flows"
    );

    // 2) a bound below the active run => its head is trimmed, tail intact, still queryable
    let live_full = r.last_seq("live").unwrap();
    let target2 = store.used_bytes().unwrap() / 4;
    store
        .prune(&PrunePolicy { max_bytes: target2 }, Some("live"))
        .unwrap();
    let r = store.reader().unwrap();
    assert!(store.used_bytes().unwrap() <= target2);
    assert_eq!(
        r.last_seq("live").unwrap(),
        live_full,
        "newest events are never pruned"
    );
    let first = r.first_seq("live").unwrap();
    assert!(first > 1, "head trimmed");
    let evs = r.events_since("live", 0, 100_000).unwrap();
    assert_eq!(evs.first().unwrap().seq, first);
    assert!(
        evs.windows(2).all(|w| w[1].seq == w[0].seq + 1),
        "retained range stays contiguous"
    );
    let (rows, _) = r
        .list_flows(
            &FlowFilter {
                run: Some("live".into()),
                ..Default::default()
            },
            10,
            None,
        )
        .unwrap();
    assert_eq!(rows[0].flow, 6_000, "newest flow survives");
    // the incrementally maintained counters agree with the rows after trimming
    let actual = r
        .list_flows(
            &FlowFilter {
                run: Some("live".into()),
                ..Default::default()
            },
            100_000,
            None,
        )
        .unwrap()
        .0
        .len() as i64;
    let run_row = r
        .runs()
        .unwrap()
        .into_iter()
        .find(|x| x.run == "live")
        .unwrap();
    assert_eq!(
        run_row.flows, actual,
        "flow_count drifted from the flows table"
    );
    assert_eq!(
        run_row.events as u64,
        r.last_seq("live").unwrap() - first + 1,
        "event span matches"
    );
    // every remaining flow's events are present (no dangling projections)
    for row in r
        .list_flows(
            &FlowFilter {
                run: Some("live".into()),
                ..Default::default()
            },
            50,
            None,
        )
        .unwrap()
        .0
    {
        assert!(row.first_seq >= first);
    }
}

/// Measures raw delete vs insert throughput (informational). Run: cargo test -p np-store --release -- --ignored --nocapture delete_vs_insert
#[tokio::test]
#[ignore]
async fn delete_vs_insert_throughput() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("t.db")).unwrap();
    let mut s = Seq::new("a");
    s.start(1);
    for i in 1..=100_000u64 {
        s.req(
            1_000 + i,
            i,
            "h.example.com",
            &format!("/api/v1/items/{i}"),
            "GET",
        );
        s.ok(1_000 + i, i, 200, 10);
    }
    let events = s.out.len() as f64;
    let t = std::time::Instant::now();
    ingest(&store, s.out).await;
    let ing = t.elapsed();
    eprintln!("insert: {:.0} events/s", events / ing.as_secs_f64());
    let used = store.used_bytes().unwrap();
    let t = std::time::Instant::now();
    let rep = store
        .prune(
            &PrunePolicy {
                max_bytes: used / 10,
            },
            Some("a"),
        )
        .unwrap();
    let del = t.elapsed();
    eprintln!(
        "delete: {} events + {} flows in {:?} => {:.0} events/s",
        rep.events_deleted,
        rep.flows_deleted,
        del,
        rep.events_deleted as f64 / del.as_secs_f64()
    );
}

/// runs() must stay cheap on huge stores (UIs poll it). Run: cargo test -p np-store --release -- --ignored --nocapture runs_list
#[tokio::test]
#[ignore]
async fn runs_list_is_cheap_on_large_stores() {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path().join("t.db")).unwrap();
    let mut s = Seq::new("big");
    s.start(1);
    for i in 1..=100_000u64 {
        s.req(1_000 + i, i, "h", &format!("/p/{i}"), "GET");
        s.ok(1_000 + i, i, 200, 1);
    }
    ingest(&store, s.out).await;
    let r = store.reader().unwrap();
    let t = std::time::Instant::now();
    let runs = r.runs().unwrap();
    let el = t.elapsed();
    eprintln!(
        "runs() over {} events / {} flows took {el:?}",
        runs[0].events, runs[0].flows
    );
    assert_eq!(runs[0].flows, 100_000);
    assert!(
        el < std::time::Duration::from_millis(20),
        "runs() took {el:?}"
    );
}
