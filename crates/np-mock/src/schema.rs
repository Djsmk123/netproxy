//! YAML schema for mock rule configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Root configuration loaded from mocks.yaml.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MockConfig {
    pub version: u32,
    pub fixtures_dir: PathBuf,
    pub rules: Vec<Rule>,
}

/// A single mock rule with matching criteria and action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "match")]
    pub match_spec: MatchSpec,
    pub action: Action,
}

fn default_true() -> bool {
    true
}

/// Criteria for matching a request.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MatchSpec {
    pub method: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
    pub path_regex: Option<String>,
    pub query: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
}

/// Action to take when a rule matches.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Respond {
        #[serde(default = "default_status")]
        status: u16,
        #[serde(default)]
        headers: HashMap<String, String>,
        body: BodySource,
        #[serde(default)]
        delay_ms: Option<u64>,
    },
    Pass,
    Fail {
        error: String,
    },
}

fn default_status() -> u16 {
    200
}

/// Source of response body content.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BodySource {
    File { file: String },
    Inline { inline: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_full_config() {
        let yaml = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: test-rule
    priority: 100
    enabled: true
    description: "Test rule"
    match:
      method: GET
      host: api.example.com
      path: /v1/users
    action:
      type: respond
      status: 200
      headers:
        content-type: application/json
      body:
        file: response.json
      delay_ms: 50
"#;
        let config: MockConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].id, "test-rule");
        assert_eq!(config.rules[0].priority, 100);
        assert!(config.rules[0].enabled);
    }

    #[test]
    fn deserialize_minimal_config() {
        let yaml = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: minimal
    match:
      host: test.com
    action:
      type: pass
"#;
        let config: MockConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.rules[0].priority, 0); // default
        assert!(config.rules[0].enabled); // default true
    }

    #[test]
    fn deserialize_inline_body() {
        let yaml = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: inline-body
    match:
      host: test.com
    action:
      type: respond
      body:
        inline: "Hello, world!"
"#;
        let config: MockConfig = serde_yaml::from_str(yaml).unwrap();
        match &config.rules[0].action {
            Action::Respond { body, status, .. } => {
                assert_eq!(*status, 200); // default
                match body {
                    BodySource::Inline { inline } => assert_eq!(inline, "Hello, world!"),
                    _ => panic!("expected inline body"),
                }
            }
            _ => panic!("expected respond action"),
        }
    }

    #[test]
    fn deserialize_fail_action() {
        let yaml = r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: fail-rule
    match:
      host: test.com
    action:
      type: fail
      error: connection_refused
"#;
        let config: MockConfig = serde_yaml::from_str(yaml).unwrap();
        match &config.rules[0].action {
            Action::Fail { error } => assert_eq!(error, "connection_refused"),
            _ => panic!("expected fail action"),
        }
    }
}
