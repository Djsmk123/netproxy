//! Integration tests for the mock rule runtime.

use futures::{SinkExt, StreamExt};
use netproxyd::{
    control,
    daemon::{Daemon, DaemonConfig},
};
use np_engine::HudsuckerEngine;
use np_model::*;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

struct Rig {
    _dir: tempfile::TempDir,
    daemon: Arc<Daemon>,
    ctl: control::ControlServer,
}

async fn rig_with_mocks(mocks_yaml: &str, fixtures: &[(&str, &[u8])]) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let data_path = dir.path();

    // Write mocks.yaml
    std::fs::write(data_path.join("mocks.yaml"), mocks_yaml).unwrap();

    // Create fixtures directory and write fixture files
    std::fs::create_dir_all(data_path.join("fixtures")).unwrap();
    for (name, content) in fixtures {
        std::fs::write(data_path.join("fixtures").join(name), content).unwrap();
    }

    // Create daemon with mock config
    let mut cfg = DaemonConfig::new(data_path.to_path_buf());
    cfg.default_listen = "127.0.0.1:0".parse().unwrap(); // Use random port

    // Load mock config if mocks.yaml exists
    let mock_config_path = data_path.join("mocks.yaml");
    if mock_config_path.exists() {
        cfg = cfg
            .with_mock_config(mock_config_path)
            .expect("load mocks.yaml");
    }

    let daemon = Daemon::new(cfg, Arc::new(HudsuckerEngine)).unwrap();
    let ctl = control::serve(daemon.clone(), &data_path.join("run"))
        .await
        .unwrap();

    Rig {
        _dir: dir,
        daemon,
        ctl,
    }
}

struct Client {
    f: Framed<UnixStream, LinesCodec>,
    next: u64,
    pending: std::collections::VecDeque<Value>,
}

impl Client {
    async fn connect(r: &Rig) -> Client {
        let mut c = Client {
            f: Framed::new(
                UnixStream::connect(&r.ctl.info.socket).await.unwrap(),
                LinesCodec::new_with_max_length(16 << 20),
            ),
            next: 0,
            pending: Default::default(),
        };
        let h = c
            .call(
                "hello",
                json!({
                    "token": r.ctl.info.token,
                    "api_version": API_VERSION,
                    "client": {"name": "test"}
                }),
            )
            .await
            .unwrap();
        assert_eq!(h["api_version"], API_VERSION);
        c
    }

    async fn line(&mut self) -> Option<Value> {
        match tokio::time::timeout(Duration::from_secs(10), self.f.next())
            .await
            .expect("timed out waiting for server")
        {
            Some(Ok(l)) => Some(serde_json::from_str(&l).unwrap()),
            _ => None,
        }
    }

    async fn call(&mut self, method: &str, params: Value) -> Result<Value, Value> {
        self.next += 1;
        let id = self.next;
        self.f
            .send(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params
                })
                .to_string(),
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
            self.pending.push_back(v);
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

#[tokio::test]
async fn mock_rules_from_yaml() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: test-mock
    priority: 100
    enabled: true
    description: "Test mock rule"
    match:
      method: GET
      host: "test.local"
      path: "/api/data"
    action:
      type: respond
      status: 200
      headers:
        content-type: application/json
        x-custom: test-value
      body:
        file: response.json
"#;

    let fixtures = [(
        "response.json",
        br#"{"status":"ok","data":{"id":123,"name":"test"}}"# as &[u8],
    )];

    let r = rig_with_mocks(mocks, &fixtures).await;
    let mut c = Client::connect(&r).await;

    // Start engine
    let st = c.call("engine.start", Value::Null).await.unwrap();
    assert_eq!(st["state"], "running");
    let listen = st["listen"].as_str().unwrap();

    // Subscribe to events
    c.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();

    // Make request through proxy
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    let resp = client
        .get("http://test.local/api/data")
        .send()
        .await
        .unwrap();

    // Verify response
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(resp.headers().get("x-custom").unwrap(), "test-value");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["data"]["id"], 123);
    assert_eq!(body["data"]["name"], "test");

    // Wait for events
    let mut mock_hit = None;
    let mut response_head = None;

    for _ in 0..20 {
        let e = c.event().await;
        match &e.event {
            Event::MockHit { flow: _, rule } => {
                mock_hit = Some(rule.clone());
            }
            Event::ResponseHead { mocked, .. } => {
                response_head = Some(*mocked);
            }
            Event::FlowCompleted { .. } => break,
            _ => {}
        }
    }

    // Verify MockHit event was emitted
    assert_eq!(
        mock_hit,
        Some("test-mock".to_string()),
        "MockHit event should be emitted"
    );
    assert_eq!(
        response_head,
        Some(true),
        "ResponseHead should have mocked=true"
    );

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_priority_ordering() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: low-priority
    priority: 10
    enabled: true
    match:
      host: "test.local"
    action:
      type: respond
      status: 200
      body:
        inline: "low priority"
  - id: high-priority
    priority: 100
    enabled: true
    match:
      host: "test.local"
    action:
      type: respond
      status: 200
      body:
        inline: "high priority"
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    c.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    let resp = client
        .get("http://test.local/any/path")
        .send()
        .await
        .unwrap();

    let body = resp.text().await.unwrap();
    assert_eq!(
        body, "high priority",
        "higher priority rule should match first"
    );

    // Verify MockHit event has high-priority rule
    let mut mock_rule = None;
    for _ in 0..10 {
        let e = c.event().await;
        if let Event::MockHit { rule, .. } = &e.event {
            mock_rule = Some(rule.clone());
            break;
        }
    }
    assert_eq!(mock_rule, Some("high-priority".to_string()));

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_inline_body() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: inline-mock
    priority: 100
    enabled: true
    match:
      host: "test.local"
      path: "/hello"
    action:
      type: respond
      status: 200
      headers:
        content-type: text/plain
      body:
        inline: "Hello from mock!"
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    let resp = client.get("http://test.local/hello").send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Hello from mock!");

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_wildcard_matching() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: api-wildcard
    priority: 100
    enabled: true
    match:
      host: "*.example.com"
      path: "/api/*"
    action:
      type: respond
      status: 200
      body:
        inline: "wildcard match"
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    // Test wildcard host
    let resp = client
        .get("http://api.example.com/api/users")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.text().await.unwrap(), "wildcard match");

    let resp = client
        .get("http://staging.example.com/api/data")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.text().await.unwrap(), "wildcard match");

    // Non-matching host should not match (but will fail in real proxy, so we skip)
    // let resp = client.get("http://other.com/api/data").send().await.unwrap();
    // assert_ne!(resp.text().await.unwrap(), "wildcard match");

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_disabled_rule() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: disabled-rule
    priority: 100
    enabled: false
    match:
      host: "test.local"
    action:
      type: respond
      status: 200
      body:
        inline: "should not match"
  - id: fallback-rule
    priority: 50
    enabled: true
    match:
      host: "test.local"
    action:
      type: respond
      status: 200
      body:
        inline: "fallback matched"
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    c.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    // Disabled rule should not match, fallback rule should match instead
    let resp = client.get("http://test.local/").send().await.unwrap();
    let body = resp.text().await.unwrap();
    assert_eq!(body, "fallback matched", "disabled rule should not match");

    // Verify MockHit event has fallback-rule, not disabled-rule
    let mut mock_rule = None;
    for _ in 0..10 {
        let e = c.event().await;
        if let Event::MockHit { rule, .. } = &e.event {
            mock_rule = Some(rule.clone());
            break;
        }
    }
    assert_eq!(
        mock_rule,
        Some("fallback-rule".to_string()),
        "should match fallback, not disabled"
    );

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_pass_action() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: pass-rule
    priority: 200
    enabled: true
    match:
      host: "test.local"
      path: "/pass"
    action:
      type: pass
  - id: respond-rule
    priority: 100
    enabled: true
    match:
      host: "test.local"
      path: "/respond"
    action:
      type: respond
      status: 200
      body:
        inline: "respond matched"
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    c.call("events.subscribe", json!({"since": 0}))
        .await
        .unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    // First test: respond rule should work normally
    let resp = client
        .get("http://test.local/respond")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.text().await.unwrap(), "respond matched");

    // Verify MockHit event was emitted for respond-rule
    let mut found_mock = false;
    for _ in 0..10 {
        let e = c.event().await;
        if let Event::MockHit { rule, .. } = &e.event {
            if rule == "respond-rule" {
                found_mock = true;
                break;
            }
        }
        if matches!(e.event, Event::FlowCompleted { .. }) {
            break;
        }
    }
    assert!(found_mock, "respond-rule should emit MockHit event");

    // Second test: pass rule lets request through without MockHit
    // Pass action means the request continues to normal flow (no mock intercept)
    // We just verify that no MockHit event is emitted for this request
    let _result = client.get("http://test.local/pass").send().await;

    // Check events - there should be NO MockHit for pass-rule
    let mut found_pass_mock = false;
    for _ in 0..20 {
        let e = c.event().await;
        if let Event::MockHit { rule, .. } = &e.event {
            if rule == "pass-rule" {
                found_pass_mock = true;
            }
        }
        // Stop when we see FlowFailed or FlowCompleted
        if matches!(
            e.event,
            Event::FlowCompleted { .. } | Event::FlowFailed { .. }
        ) {
            break;
        }
    }
    assert!(
        !found_pass_mock,
        "pass action should NOT emit MockHit event"
    );

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}

#[tokio::test]
async fn mock_fail_action() {
    let mocks = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: fail-rule
    priority: 100
    enabled: true
    match:
      host: "test.local"
    action:
      type: fail
      error: connection_refused
"#;

    let r = rig_with_mocks(mocks, &[]).await;
    let mut c = Client::connect(&r).await;

    let st = c.call("engine.start", Value::Null).await.unwrap();
    let listen = st["listen"].as_str().unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{}", listen)).unwrap())
        .build()
        .unwrap();

    let resp = client.get("http://test.local/").send().await.unwrap();

    // Fail action returns 503 for now (connection failure simulation)
    assert_eq!(resp.status(), 503);
    let body = resp.text().await.unwrap();
    assert!(body.contains("connection_refused"));

    c.call("engine.stop", Value::Null).await.unwrap();
    r.ctl.stop().await;
}
