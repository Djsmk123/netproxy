//! Dynamic mock registry with hot reload support.
//!
//! The registry manages both static rules (from mocks.yaml) and dynamic rules (added at runtime).
//! It provides thread-safe access for concurrent reads (hot path) and exclusive writes.

use crate::resolver::RuleMockResolver;
use crate::schema::{MockConfig, Rule};
use crate::validation::{RuleValidator, ValidationIssue};
use np_engine::engine::{MockDecision, MockResolver, RequestView};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Dynamic registry that combines static and runtime-added rules.
///
/// Rules are evaluated in this order:
/// 1. Dynamic rules (sorted by priority)
/// 2. Static rules from mocks.yaml (sorted by priority)
///
/// This allows runtime-added rules to override config file rules when needed.
pub struct DynamicMockRegistry {
    /// Path to mocks.yaml for reloading
    config_path: PathBuf,

    /// Fixtures directory for validation
    fixtures_dir: PathBuf,

    /// Static rules loaded from mocks.yaml
    static_resolver: Arc<RwLock<Option<RuleMockResolver>>>,

    /// Dynamic rules added at runtime
    dynamic_rules: Arc<RwLock<Vec<Rule>>>,

    /// Fallback resolver (typically NoMocks)
    fallback: Arc<dyn MockResolver>,
}

impl DynamicMockRegistry {
    /// Create a new registry from a config file.
    pub fn new(
        config_path: PathBuf,
        fallback: Arc<dyn MockResolver>,
    ) -> Result<Self, RegistryError> {
        let (static_resolver, fixtures_dir) = if config_path.exists() {
            let yaml = std::fs::read_to_string(&config_path)?;
            let config: MockConfig = serde_yaml::from_str(&yaml)
                .map_err(|e| RegistryError::Config(format!("invalid YAML: {}", e)))?;

            // Resolve fixtures_dir relative to config file
            let config_dir = config_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let mut config = config;
            if !config.fixtures_dir.is_absolute() {
                config.fixtures_dir = config_dir.join(&config.fixtures_dir);
            }

            let fixtures_dir = config.fixtures_dir.clone();
            let resolver = RuleMockResolver::new(config, fallback.clone())
                .map_err(|e| RegistryError::Config(format!("resolver init: {}", e)))?;

            (Some(resolver), fixtures_dir)
        } else {
            // Default fixtures dir if no config
            let default_dir = config_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("fixtures");
            (None, default_dir)
        };

        Ok(Self {
            config_path,
            fixtures_dir,
            static_resolver: Arc::new(RwLock::new(static_resolver)),
            dynamic_rules: Arc::new(RwLock::new(Vec::new())),
            fallback,
        })
    }

    /// Reload rules from the config file.
    ///
    /// This preserves dynamic rules and only reloads static rules from disk.
    pub fn reload(&self) -> Result<(), RegistryError> {
        if !self.config_path.exists() {
            // Config file deleted - clear static rules
            *self.static_resolver.write().unwrap() = None;
            tracing::info!("config file removed, static rules cleared");
            return Ok(());
        }

        let yaml = std::fs::read_to_string(&self.config_path)?;
        let config: MockConfig = serde_yaml::from_str(&yaml)
            .map_err(|e| RegistryError::Config(format!("invalid YAML: {}", e)))?;

        // Resolve fixtures_dir relative to config file
        let config_dir = self
            .config_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let mut config = config;
        if !config.fixtures_dir.is_absolute() {
            config.fixtures_dir = config_dir.join(&config.fixtures_dir);
        }

        let new_resolver = RuleMockResolver::new(config, self.fallback.clone())
            .map_err(|e| RegistryError::Config(format!("resolver init: {}", e)))?;

        *self.static_resolver.write().unwrap() = Some(new_resolver);

        tracing::info!("reloaded static rules from {}", self.config_path.display());
        Ok(())
    }

    /// Add a dynamic rule at runtime.
    ///
    /// The rule is validated before adding. Returns error if validation fails.
    pub fn add_rule(&self, rule: Rule) -> Result<(), RegistryError> {
        if !rule.enabled {
            return Err(RegistryError::Validation("cannot add disabled rule".into()));
        }

        let mut dynamic = self.dynamic_rules.write().unwrap();

        // Check for duplicate ID
        if dynamic.iter().any(|r| r.id == rule.id) {
            return Err(RegistryError::DuplicateId(rule.id.clone()));
        }

        dynamic.push(rule.clone());
        // Re-sort by priority (higher first)
        dynamic.sort_by(|a, b| b.priority.cmp(&a.priority));

        tracing::info!("added dynamic rule: {}", rule.id);
        Ok(())
    }

    /// Remove a dynamic rule by ID.
    ///
    /// Returns Ok(true) if removed, Ok(false) if not found.
    pub fn remove_rule(&self, id: &str) -> Result<bool, RegistryError> {
        let mut dynamic = self.dynamic_rules.write().unwrap();
        let before = dynamic.len();
        dynamic.retain(|r| r.id != id);
        let removed = dynamic.len() < before;

        if removed {
            tracing::info!("removed dynamic rule: {}", id);
        }

        Ok(removed)
    }

    /// Update an existing dynamic rule.
    ///
    /// Returns Ok(true) if updated, Ok(false) if not found.
    pub fn update_rule(&self, rule: Rule) -> Result<bool, RegistryError> {
        let mut dynamic = self.dynamic_rules.write().unwrap();

        if let Some(existing) = dynamic.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule.clone();
            // Re-sort by priority
            dynamic.sort_by(|a, b| b.priority.cmp(&a.priority));
            tracing::info!("updated dynamic rule: {}", rule.id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all rules (dynamic + static).
    pub fn list_rules(&self) -> Vec<Rule> {
        let mut rules = Vec::new();

        // Add dynamic rules
        rules.extend(self.dynamic_rules.read().unwrap().clone());

        // Add static rules (from resolver)
        if let Some(ref resolver) = *self.static_resolver.read().unwrap() {
            rules.extend(resolver.rules().clone());
        }

        rules
    }

    /// List only dynamic rules.
    pub fn list_dynamic_rules(&self) -> Vec<Rule> {
        self.dynamic_rules.read().unwrap().clone()
    }

    /// List only static rules (from config file).
    pub fn list_static_rules(&self) -> Vec<Rule> {
        if let Some(ref resolver) = *self.static_resolver.read().unwrap() {
            resolver.rules().clone()
        } else {
            Vec::new()
        }
    }

    /// Get a rule by ID (checks both dynamic and static).
    pub fn get_rule(&self, id: &str) -> Option<Rule> {
        // Check dynamic first
        if let Some(rule) = self
            .dynamic_rules
            .read()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
        {
            return Some(rule.clone());
        }

        // Check static
        if let Some(ref resolver) = *self.static_resolver.read().unwrap() {
            if let Some(rule) = resolver.rules().iter().find(|r| r.id == id) {
                return Some(rule.clone());
            }
        }

        None
    }

    /// Clear all dynamic rules (static rules unchanged).
    pub fn clear_dynamic_rules(&self) {
        let mut dynamic = self.dynamic_rules.write().unwrap();
        let count = dynamic.len();
        dynamic.clear();
        tracing::info!("cleared {} dynamic rules", count);
    }

    /// Validate all current rules (dynamic + static).
    pub fn validate_all(&self) -> Vec<ValidationIssue> {
        let rules = self.list_rules();
        let validator = RuleValidator::new(&rules, &self.fixtures_dir);
        validator.validate()
    }

    /// Validate a single rule in the context of existing rules.
    ///
    /// This checks the rule itself plus potential conflicts with existing rules.
    pub fn validate_rule(&self, rule: &Rule) -> Vec<ValidationIssue> {
        // Combine with existing rules for validation
        let mut rules = self.list_rules();
        rules.push(rule.clone());

        let validator = RuleValidator::new(&rules, &self.fixtures_dir);
        let all_issues = validator.validate();

        // Filter to issues involving the new rule
        all_issues
            .into_iter()
            .filter(|issue| {
                issue.rule_id == rule.id || issue.related_rule.as_deref() == Some(&rule.id)
            })
            .collect()
    }
}

impl MockResolver for DynamicMockRegistry {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
        // Try dynamic rules first (already sorted by priority)
        for rule in self.dynamic_rules.read().unwrap().iter() {
            if rule.matches(req) {
                tracing::debug!("matched dynamic rule: {}", rule.id);
                return self.apply_dynamic_rule(rule, req);
            }
        }

        // Try static rules
        if let Some(ref resolver) = *self.static_resolver.read().unwrap() {
            let decision = resolver.resolve(req);
            if !matches!(decision, MockDecision::Pass) {
                return decision;
            }
        }

        // Fallback
        self.fallback.resolve(req)
    }
}

impl DynamicMockRegistry {
    fn apply_dynamic_rule(&self, rule: &Rule, req: &RequestView<'_>) -> MockDecision {
        // Delegate to static resolver's action application if we have one
        if let Some(ref resolver) = *self.static_resolver.read().unwrap() {
            resolver.apply_action_for_rule(rule, req)
        } else {
            // Minimal implementation for when there's no static resolver
            use crate::schema::Action;
            use bytes::Bytes;
            use np_engine::engine::MockResponse;

            match &rule.action {
                Action::Respond {
                    status,
                    headers,
                    body,
                    ..
                } => {
                    // Load body (simplified - no fixture support without resolver)
                    let body_bytes = match body {
                        crate::schema::BodySource::Inline { inline } => Bytes::from(inline.clone()),
                        _ => Bytes::from(r#"{"error":"fixture not supported in minimal mode"}"#),
                    };

                    MockDecision::Respond(MockResponse {
                        rule: rule.id.clone(),
                        status: *status,
                        headers: headers
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                        body: body_bytes,
                    })
                }
                Action::Pass => MockDecision::Pass,
                Action::Fail { error } => MockDecision::Respond(MockResponse {
                    rule: rule.id.clone(),
                    status: 503,
                    headers: vec![("content-type".to_string(), "text/plain".to_string())],
                    body: Bytes::from(format!("Simulated error: {}", error)),
                }),
            }
        }
    }
}

/// Errors that can occur in the registry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("duplicate rule ID: {0}")]
    DuplicateId(String),

    #[error("validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Action, BodySource, MatchSpec};
    use np_engine::engine::NoMocks;
    use std::collections::HashMap;

    fn test_rule(id: &str, priority: i32) -> Rule {
        Rule {
            id: id.to_string(),
            priority,
            enabled: true,
            description: None,
            match_spec: MatchSpec {
                host: Some("test.local".to_string()),
                ..Default::default()
            },
            action: Action::Respond {
                status: 200,
                headers: HashMap::new(),
                body: BodySource::Inline {
                    inline: format!("response from {}", id),
                },
                delay_ms: None,
            },
        }
    }

    #[test]
    fn add_and_list_dynamic_rules() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let registry = DynamicMockRegistry::new(config_path, Arc::new(NoMocks)).unwrap();

        let rule1 = test_rule("rule1", 100);
        let rule2 = test_rule("rule2", 200);

        registry.add_rule(rule1.clone()).unwrap();
        registry.add_rule(rule2.clone()).unwrap();

        let rules = registry.list_dynamic_rules();
        assert_eq!(rules.len(), 2);

        // Should be sorted by priority (higher first)
        assert_eq!(rules[0].id, "rule2");
        assert_eq!(rules[1].id, "rule1");
    }

    #[test]
    fn remove_dynamic_rule() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let registry = DynamicMockRegistry::new(config_path, Arc::new(NoMocks)).unwrap();

        let rule = test_rule("test", 100);
        registry.add_rule(rule).unwrap();

        assert_eq!(registry.list_dynamic_rules().len(), 1);

        let removed = registry.remove_rule("test").unwrap();
        assert!(removed);
        assert_eq!(registry.list_dynamic_rules().len(), 0);

        // Removing again returns false
        let removed = registry.remove_rule("test").unwrap();
        assert!(!removed);
    }

    #[test]
    fn duplicate_id_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let registry = DynamicMockRegistry::new(config_path, Arc::new(NoMocks)).unwrap();

        let rule = test_rule("dup", 100);
        registry.add_rule(rule.clone()).unwrap();

        let result = registry.add_rule(rule);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::DuplicateId(_)));
    }

    #[test]
    fn update_rule() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let registry = DynamicMockRegistry::new(config_path, Arc::new(NoMocks)).unwrap();

        let rule = test_rule("test", 100);
        registry.add_rule(rule).unwrap();

        let mut updated = test_rule("test", 200);
        updated.description = Some("updated".to_string());

        let result = registry.update_rule(updated).unwrap();
        assert!(result);

        let retrieved = registry.get_rule("test").unwrap();
        assert_eq!(retrieved.priority, 200);
        assert_eq!(retrieved.description, Some("updated".to_string()));
    }

    #[test]
    fn clear_dynamic_rules() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let registry = DynamicMockRegistry::new(config_path, Arc::new(NoMocks)).unwrap();

        registry.add_rule(test_rule("rule1", 100)).unwrap();
        registry.add_rule(test_rule("rule2", 200)).unwrap();

        assert_eq!(registry.list_dynamic_rules().len(), 2);

        registry.clear_dynamic_rules();
        assert_eq!(registry.list_dynamic_rules().len(), 0);
    }
}
