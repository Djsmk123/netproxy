//! Mock resolver implementation.

use crate::fixtures::FixtureResolver;
use crate::schema::{Action, MockConfig, Rule};
use bytes::Bytes;
use np_engine::engine::{MockDecision, MockResolver, MockResponse, RequestView};
use std::sync::{Arc, Mutex};

/// Rule-based mock resolver that evaluates rules in priority order.
pub struct RuleMockResolver {
    rules: Vec<Rule>,
    fixtures: Arc<Mutex<FixtureResolver>>,
    inner: Arc<dyn MockResolver>,
}

impl RuleMockResolver {
    /// Create a new resolver from configuration.
    /// Rules are sorted by priority (higher priority evaluated first).
    pub fn new(config: MockConfig, inner: Arc<dyn MockResolver>) -> Result<Self, ConfigError> {
        let mut rules = config.rules;
        // Sort by priority descending (higher priority first)
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Resolve fixtures_dir relative to config location if relative
        let fixtures_dir = if config.fixtures_dir.is_absolute() {
            config.fixtures_dir
        } else {
            // For now, treat as relative to current directory
            // In production, this would be relative to mocks.yaml location
            std::env::current_dir()
                .map_err(|e| ConfigError::Io(e))?
                .join(config.fixtures_dir)
        };

        let fixtures = Arc::new(Mutex::new(FixtureResolver::new(fixtures_dir)));

        Ok(Self {
            rules,
            fixtures,
            inner,
        })
    }

    fn apply_action(&self, rule: &Rule, _req: &RequestView<'_>) -> MockDecision {
        match &rule.action {
            Action::Respond {
                status,
                headers,
                body,
                delay_ms,
            } => {
                // Resolve fixture body
                let body_bytes = match self.fixtures.lock().unwrap().load(body) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        eprintln!("fixture error for rule {}: {}", rule.id, e);
                        Bytes::from(r#"{"error":"fixture not found"}"#)
                    }
                };

                // TODO: delay_ms handling (deferred to M3.5)
                if let Some(delay) = delay_ms {
                    if *delay > 0 {
                        // In M3, delays are not implemented
                        // In M3.5, this would insert a tokio::time::sleep
                        eprintln!("warning: delay_ms={} not implemented in M3", delay);
                    }
                }

                // Convert headers from HashMap to Vec<(String, String)>
                let response_headers: Vec<(String, String)> = headers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                MockDecision::Respond(MockResponse {
                    rule: rule.id.clone(),
                    status: *status,
                    headers: response_headers,
                    body: body_bytes,
                })
            }
            Action::Pass => MockDecision::Pass,
            Action::Fail { error } => {
                // Simulate connection failure with a 503 response
                // TODO: In the future, this could signal the engine to drop the connection
                MockDecision::Respond(MockResponse {
                    rule: rule.id.clone(),
                    status: 503,
                    headers: vec![("content-type".to_string(), "text/plain".to_string())],
                    body: Bytes::from(format!("Simulated error: {}", error)),
                })
            }
        }
    }
}

impl MockResolver for RuleMockResolver {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
        // Find first matching rule (rules already sorted by priority)
        for rule in &self.rules {
            if rule.matches(req) {
                return self.apply_action(rule, req);
            }
        }

        // No match, delegate to inner resolver
        self.inner.resolve(req)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{BodySource, MatchSpec};
    use np_engine::engine::NoMocks;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn priority_ordering() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("low.txt"), b"low priority").unwrap();
        fs::write(dir.path().join("high.txt"), b"high priority").unwrap();

        let config = MockConfig {
            version: 1,
            fixtures_dir: dir.path().to_path_buf(),
            rules: vec![
                Rule {
                    id: "low".to_string(),
                    priority: 10,
                    enabled: true,
                    description: None,
                    match_spec: MatchSpec {
                        host: Some("test.com".to_string()),
                        ..Default::default()
                    },
                    action: Action::Respond {
                        status: 200,
                        headers: HashMap::new(),
                        body: BodySource::File {
                            file: "low.txt".to_string(),
                        },
                        delay_ms: None,
                    },
                },
                Rule {
                    id: "high".to_string(),
                    priority: 100,
                    enabled: true,
                    description: None,
                    match_spec: MatchSpec {
                        host: Some("test.com".to_string()),
                        ..Default::default()
                    },
                    action: Action::Respond {
                        status: 200,
                        headers: HashMap::new(),
                        body: BodySource::File {
                            file: "high.txt".to_string(),
                        },
                        delay_ms: None,
                    },
                },
            ],
        };

        let resolver =
            RuleMockResolver::new(config, Arc::new(NoMocks)).unwrap();

        let headers = vec![];
        let req = RequestView {
            method: "GET",
            uri: "http://test.com/",
            host: Some("test.com"),
            path: "/",
            headers: &headers,
        };

        match resolver.resolve(&req) {
            MockDecision::Respond(resp) => {
                assert_eq!(resp.rule, "high");
                assert_eq!(resp.body, "high priority");
            }
            _ => panic!("expected respond decision"),
        }
    }

    #[test]
    fn pass_action() {
        let dir = tempdir().unwrap();
        let config = MockConfig {
            version: 1,
            fixtures_dir: dir.path().to_path_buf(),
            rules: vec![Rule {
                id: "pass-rule".to_string(),
                priority: 0,
                enabled: true,
                description: None,
                match_spec: MatchSpec {
                    host: Some("test.com".to_string()),
                    ..Default::default()
                },
                action: Action::Pass,
            }],
        };

        let resolver =
            RuleMockResolver::new(config, Arc::new(NoMocks)).unwrap();

        let headers = vec![];
        let req = RequestView {
            method: "GET",
            uri: "http://test.com/",
            host: Some("test.com"),
            path: "/",
            headers: &headers,
        };

        match resolver.resolve(&req) {
            MockDecision::Pass => {}
            _ => panic!("expected pass decision"),
        }
    }

    #[test]
    fn fail_action() {
        let dir = tempdir().unwrap();
        let config = MockConfig {
            version: 1,
            fixtures_dir: dir.path().to_path_buf(),
            rules: vec![Rule {
                id: "fail-rule".to_string(),
                priority: 0,
                enabled: true,
                description: None,
                match_spec: MatchSpec {
                    host: Some("test.com".to_string()),
                    ..Default::default()
                },
                action: Action::Fail {
                    error: "connection_refused".to_string(),
                },
            }],
        };

        let resolver =
            RuleMockResolver::new(config, Arc::new(NoMocks)).unwrap();

        let headers = vec![];
        let req = RequestView {
            method: "GET",
            uri: "http://test.com/",
            host: Some("test.com"),
            path: "/",
            headers: &headers,
        };

        match resolver.resolve(&req) {
            MockDecision::Respond(resp) => {
                assert_eq!(resp.rule, "fail-rule");
                assert_eq!(resp.status, 503);
                assert!(String::from_utf8_lossy(&resp.body).contains("connection_refused"));
            }
            _ => panic!("expected respond decision with error"),
        }
    }

    #[test]
    fn no_match_delegates() {
        let dir = tempdir().unwrap();
        let config = MockConfig {
            version: 1,
            fixtures_dir: dir.path().to_path_buf(),
            rules: vec![Rule {
                id: "specific".to_string(),
                priority: 0,
                enabled: true,
                description: None,
                match_spec: MatchSpec {
                    host: Some("specific.com".to_string()),
                    ..Default::default()
                },
                action: Action::Pass,
            }],
        };

        let resolver =
            RuleMockResolver::new(config, Arc::new(NoMocks)).unwrap();

        let headers = vec![];
        let req = RequestView {
            method: "GET",
            uri: "http://other.com/",
            host: Some("other.com"),
            path: "/",
            headers: &headers,
        };

        // Should delegate to NoMocks, which returns Pass
        match resolver.resolve(&req) {
            MockDecision::Pass => {}
            _ => panic!("expected pass decision from inner resolver"),
        }
    }
}
