//! Integration tests for dynamic mock registry and control API.

use futures::{SinkExt, StreamExt};
use netproxyd::{
    control,
    daemon::{Daemon, DaemonConfig},
};
use np_engine::HudsuckerEngine;
use serde_json::{json, Value};
use std::{fs, sync::Arc};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

struct Rig {
    daemon: Arc<Daemon>,
    ctl: control::ControlServer,
}

async fn rig(data_path: &std::path::Path) -> Rig {
    let cfg = DaemonConfig::new(data_path.to_path_buf())
        .with_mock_config(data_path.join("mocks.yaml"))
        .unwrap();
    let daemon = Daemon::new(cfg, Arc::new(HudsuckerEngine)).unwrap();
    daemon.spawn_maintenance(std::time::Duration::from_secs(300));
    let ctl = control::serve(daemon.clone(), &data_path.join("run"))
        .await
        .unwrap();
    Rig { daemon, ctl }
}

struct Client {
    sock: Framed<UnixStream, LinesCodec>,
    next_id: u64,
}

impl Client {
    async fn connect(r: &Rig) -> Self {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Give server time to start
        let sock = UnixStream::connect(&r.ctl.info.socket).await.unwrap();
        let mut sock = Framed::new(sock, LinesCodec::new_with_max_length(1 << 20));
        let token = std::fs::read_to_string(&r.ctl.info.token_file)
            .unwrap()
            .trim()
            .to_string();
        sock.send(
            json!({"jsonrpc": "2.0", "method": "hello", "params": {"token": token}}).to_string(),
        )
        .await
        .unwrap();
        let resp = sock.next().await.unwrap().unwrap();
        let _: Value = serde_json::from_str(&resp).unwrap(); // Verify hello response
        Self { sock, next_id: 1 }
    }

    async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.sock
            .send(
                json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
            )
            .await
            .unwrap();
        let line = self.sock.next().await.unwrap().unwrap();
        let resp: Value = serde_json::from_str(&line).unwrap();
        if let Some(result) = resp.get("result") {
            Ok(result.clone())
        } else if let Some(error) = resp.get("error") {
            Err(error["message"]
                .as_str()
                .unwrap_or("unknown error")
                .to_string())
        } else {
            Err("invalid response".to_string())
        }
    }
}

#[tokio::test]
async fn dynamic_mocks_add_remove() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    // Create fixtures directory
    let fixtures = data.join("fixtures");
    fs::create_dir(&fixtures).unwrap();
    fs::write(fixtures.join("response.json"), r#"{"dynamic":"rule"}"#).unwrap();

    // Create minimal mocks.yaml
    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules: []
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    c.call("engine.start", Value::Null).await.unwrap();

    // List should be empty initially
    let resp = c.call("mocks.list", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 0);

    // Add a dynamic rule
    let rule = json!({
        "id": "dynamic-test",
        "priority": 100,
        "enabled": true,
        "match": {
            "host": "dynamic.test",
            "path": "/api"
        },
        "action": {
            "type": "respond",
            "status": 200,
            "headers": {
                "content-type": "application/json"
            },
            "body": {
                "file": "response.json"
            }
        }
    });

    let resp = c.call("mocks.add", json!({"rule": rule})).await.unwrap();
    assert_eq!(resp["success"], true);

    // List should now have 1 rule
    let resp = c.call("mocks.list_dynamic", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 1);

    // Get the rule
    let resp = c
        .call("mocks.get", json!({"id": "dynamic-test"}))
        .await
        .unwrap();
    assert_eq!(resp["rule"]["id"], "dynamic-test");

    // Remove the rule
    let resp = c
        .call("mocks.remove", json!({"id": "dynamic-test"}))
        .await
        .unwrap();
    assert_eq!(resp["removed"], true);

    // List should be empty again
    let resp = c.call("mocks.list_dynamic", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn dynamic_mocks_validation() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules: []
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    c.call("engine.start", Value::Null).await.unwrap();

    // Try to add a rule with missing fixture
    let rule = json!({
        "id": "bad-fixture",
        "priority": 100,
        "enabled": true,
        "match": {
            "host": "test.local"
        },
        "action": {
            "type": "respond",
            "status": 200,
            "body": {
                "file": "nonexistent.json"
            }
        }
    });

    let resp = c.call("mocks.add", json!({"rule": rule})).await;
    assert!(resp.is_err(), "should fail validation");
}

#[tokio::test]
async fn dynamic_mocks_update() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules: []
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    c.call("engine.start", Value::Null).await.unwrap();

    // Add a rule
    let rule = json!({
        "id": "test-update",
        "priority": 100,
        "enabled": true,
        "match": {
            "host": "test.local"
        },
        "action": {
            "type": "pass"
        }
    });

    c.call("mocks.add", json!({"rule": rule})).await.unwrap();

    // Update the rule
    let updated_rule = json!({
        "id": "test-update",
        "priority": 200,
        "enabled": true,
        "match": {
            "host": "updated.local"
        },
        "action": {
            "type": "pass"
        }
    });

    let resp = c
        .call("mocks.update", json!({"rule": updated_rule}))
        .await
        .unwrap();
    assert_eq!(resp["updated"], true);

    // Verify update
    let resp = c
        .call("mocks.get", json!({"id": "test-update"}))
        .await
        .unwrap();
    assert_eq!(resp["rule"]["priority"], 200);
    assert_eq!(resp["rule"]["match"]["host"], "updated.local");
}

#[tokio::test]
async fn static_and_dynamic_rules_combined() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    // Create static rule
    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: static-rule
    priority: 50
    enabled: true
    match:
      host: static.test
    action:
      type: pass
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    c.call("engine.start", Value::Null).await.unwrap();

    // Should have 1 static rule
    let resp = c.call("mocks.list_static", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 1);

    // Add dynamic rule
    let rule = json!({
        "id": "dynamic-rule",
        "priority": 100,
        "enabled": true,
        "match": {
            "host": "dynamic.test"
        },
        "action": {
            "type": "pass"
        }
    });

    c.call("mocks.add", json!({"rule": rule})).await.unwrap();

    // List all should show both
    let resp = c.call("mocks.list", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 2);

    // List dynamic should show 1
    let resp = c.call("mocks.list_dynamic", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 1);

    // List static should still show 1
    let resp = c.call("mocks.list_static", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn clear_dynamic_rules() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules: []
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    c.call("engine.start", Value::Null).await.unwrap();

    // Add multiple dynamic rules
    for i in 1..=3 {
        let rule = json!({
            "id": format!("rule-{}", i),
            "priority": 100,
            "enabled": true,
            "match": {
                "host": format!("test{}.local", i)
            },
            "action": {
                "type": "pass"
            }
        });
        c.call("mocks.add", json!({"rule": rule})).await.unwrap();
    }

    // Should have 3 rules
    let resp = c.call("mocks.list_dynamic", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 3);

    // Clear all dynamic rules
    c.call("mocks.clear_dynamic", Value::Null).await.unwrap();

    // Should have 0 rules
    let resp = c.call("mocks.list_dynamic", Value::Null).await.unwrap();
    assert_eq!(resp["rules"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn validation_api() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    fs::write(
        data.join("mocks.yaml"),
        r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: dup
    priority: 100
    enabled: true
    match:
      host: test.local
    action:
      type: pass
  - id: dup
    priority: 200
    enabled: true
    match:
      host: other.local
    action:
      type: pass
"#,
    )
    .unwrap();

    let r = rig(data).await;
    let mut c = Client::connect(&r).await;

    // Start will fail due to duplicate IDs
    let result = c.call("engine.start", Value::Null).await;
    assert!(result.is_err(), "should fail validation on startup");
}
