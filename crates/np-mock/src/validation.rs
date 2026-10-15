//! Validation framework for mock rules.
//!
//! Detects conflicts, unreachable rules, and integrity issues before rules go live.

use crate::schema::{Action, BodySource, MatchSpec, Rule};
use std::collections::HashSet;
use std::path::Path;

/// Validation result with severity levels.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub kind: IssueKind,
    pub rule_id: String,
    pub message: String,
    /// Optional second rule involved (for conflicts)
    pub related_rule: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Severity {
    /// Rule will not work (missing fixture, invalid regex, etc.)
    Error,
    /// Rule may not behave as expected (shadowed, conflicting, etc.)
    Warning,
    /// Best practice violation (low priority, disabled rule, etc.)
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum IssueKind {
    /// Duplicate rule ID
    DuplicateId,
    /// Rule is shadowed by higher-priority rule and will never match
    Unreachable,
    /// Two rules with overlapping patterns but different actions
    Conflict,
    /// Fixture file referenced but not found
    MissingFixture,
    /// Invalid regex pattern in match spec
    InvalidRegex,
    /// Disabled rule (informational)
    DisabledRule,
    /// Rule priority is very low or zero
    LowPriority,
}

/// Validates a set of rules and returns all issues found.
pub struct RuleValidator<'a> {
    rules: &'a [Rule],
    fixtures_dir: &'a Path,
}

impl<'a> RuleValidator<'a> {
    pub fn new(rules: &'a [Rule], fixtures_dir: &'a Path) -> Self {
        Self { rules, fixtures_dir }
    }

    /// Run all validation checks and return issues sorted by severity.
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        issues.extend(self.check_duplicate_ids());
        issues.extend(self.check_fixture_integrity());
        issues.extend(self.check_regex_patterns());
        issues.extend(self.check_unreachable_rules());
        issues.extend(self.check_conflicts());
        issues.extend(self.check_disabled_rules());
        issues.extend(self.check_low_priority());

        // Sort by severity (errors first)
        issues.sort_by_key(|i| (i.severity, i.rule_id.clone()));
        issues
    }

    /// Check for duplicate rule IDs.
    fn check_duplicate_ids(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut seen = HashSet::new();

        for rule in self.rules {
            if !seen.insert(&rule.id) {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    kind: IssueKind::DuplicateId,
                    rule_id: rule.id.clone(),
                    message: format!("duplicate rule ID: {}", rule.id),
                    related_rule: None,
                });
            }
        }

        issues
    }

    /// Check that all fixture files exist.
    fn check_fixture_integrity(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for rule in self.rules {
            if let Action::Respond { body, .. } = &rule.action {
                if let BodySource::File { file } = body {
                    let path = self.fixtures_dir.join(file);
                    if !path.exists() {
                        issues.push(ValidationIssue {
                            severity: Severity::Error,
                            kind: IssueKind::MissingFixture,
                            rule_id: rule.id.clone(),
                            message: format!("fixture not found: {} (expected at {})", file, path.display()),
                            related_rule: None,
                        });
                    }
                }
            }
        }

        issues
    }

    /// Check that regex patterns compile.
    fn check_regex_patterns(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for rule in self.rules {
            if let Some(ref pattern) = rule.match_spec.path_regex {
                if let Err(e) = regex::Regex::new(pattern) {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        kind: IssueKind::InvalidRegex,
                        rule_id: rule.id.clone(),
                        message: format!("invalid regex pattern '{}': {}", pattern, e),
                        related_rule: None,
                    });
                }
            }

            // Check header value patterns if they look like regexes
            if let Some(ref headers) = rule.match_spec.headers {
                for (key, value) in headers {
                    if value.contains("^") || value.contains("$") || value.contains(".*") {
                        // Looks like a regex - validate it
                        if let Err(e) = regex::Regex::new(value) {
                            issues.push(ValidationIssue {
                                severity: Severity::Warning,
                                kind: IssueKind::InvalidRegex,
                                rule_id: rule.id.clone(),
                                message: format!("header '{}' pattern '{}' looks like regex but is invalid: {}", key, value, e),
                                related_rule: None,
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    /// Check for unreachable rules (shadowed by higher-priority broader rules).
    fn check_unreachable_rules(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Rules are assumed to be sorted by priority (descending)
        for i in 0..self.rules.len() {
            for j in 0..i {
                let earlier = &self.rules[j];
                let later = &self.rules[i];

                // Skip if priorities are equal (not shadowing)
                if earlier.priority == later.priority {
                    continue;
                }

                // Check if earlier rule completely shadows later
                if self.shadows(earlier, later) {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        kind: IssueKind::Unreachable,
                        rule_id: later.id.clone(),
                        message: format!(
                            "rule '{}' is unreachable - completely shadowed by higher-priority rule '{}'",
                            later.id, earlier.id
                        ),
                        related_rule: Some(earlier.id.clone()),
                    });
                    break; // Only report first shadowing rule
                }
            }
        }

        issues
    }

    /// Check for conflicting rules (similar patterns, different actions).
    fn check_conflicts(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for i in 0..self.rules.len() {
            for j in (i + 1)..self.rules.len() {
                let r1 = &self.rules[i];
                let r2 = &self.rules[j];

                if self.potentially_conflicts(r1, r2) {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        kind: IssueKind::Conflict,
                        rule_id: r1.id.clone(),
                        message: format!(
                            "rule '{}' may conflict with '{}' - overlapping match patterns but different actions/priorities",
                            r1.id, r2.id
                        ),
                        related_rule: Some(r2.id.clone()),
                    });
                }
            }
        }

        issues
    }

    /// Check for disabled rules (informational).
    fn check_disabled_rules(&self) -> Vec<ValidationIssue> {
        self.rules
            .iter()
            .filter(|r| !r.enabled)
            .map(|r| ValidationIssue {
                severity: Severity::Info,
                kind: IssueKind::DisabledRule,
                rule_id: r.id.clone(),
                message: format!("rule '{}' is disabled and will not be evaluated", r.id),
                related_rule: None,
            })
            .collect()
    }

    /// Check for rules with suspiciously low priority.
    fn check_low_priority(&self) -> Vec<ValidationIssue> {
        self.rules
            .iter()
            .filter(|r| r.priority <= 0 && r.enabled)
            .map(|r| ValidationIssue {
                severity: Severity::Info,
                kind: IssueKind::LowPriority,
                rule_id: r.id.clone(),
                message: format!("rule '{}' has low/zero priority ({})", r.id, r.priority),
                related_rule: None,
            })
            .collect()
    }

    /// Check if rule A completely shadows rule B.
    ///
    /// Returns true if any request that matches B also matches A,
    /// meaning B will never be evaluated (A has higher priority).
    fn shadows(&self, a: &Rule, b: &Rule) -> bool {
        // If A is disabled, it can't shadow anything
        if !a.enabled {
            return false;
        }

        // If B is disabled, don't report shadowing (already reported as disabled)
        if !b.enabled {
            return false;
        }

        // Check each match field: A must be equal or broader than B
        if !self.match_broader_or_equal(&a.match_spec, &b.match_spec) {
            return false;
        }

        true
    }

    /// Check if match spec A is broader than or equal to B.
    fn match_broader_or_equal(&self, a: &MatchSpec, b: &MatchSpec) -> bool {
        // Method: A must match all of B's methods
        if !self.field_broader_or_equal(&a.method, &b.method) {
            return false;
        }

        // Host: A must match all of B's hosts
        if !self.pattern_broader_or_equal(&a.host, &b.host) {
            return false;
        }

        // Path: A must match all of B's paths
        if !self.pattern_broader_or_equal(&a.path, &b.path) {
            return false;
        }

        // If B has path_regex and A doesn't, A is not broader
        if b.path_regex.is_some() && a.path_regex.is_none() {
            return false;
        }

        // If B has query constraints and A doesn't, A is not broader
        if b.query.is_some() && a.query.is_none() {
            return false;
        }

        // If B has header constraints and A doesn't, A is not broader
        if b.headers.is_some() && a.headers.is_none() {
            return false;
        }

        true
    }

    /// Check if field A is broader or equal to B.
    fn field_broader_or_equal(&self, a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            // A matches anything, B is specific -> A is broader
            (None, Some(_)) => true,
            // A matches anything, B matches anything -> equal
            (None, None) => true,
            // A is specific, B matches anything -> A is not broader
            (Some(_), None) => false,
            // Both specific -> must be equal
            (Some(a_val), Some(b_val)) => a_val.eq_ignore_ascii_case(b_val),
        }
    }

    /// Check if pattern A is broader or equal to B.
    fn pattern_broader_or_equal(&self, a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            // A matches anything -> broader
            (None, _) => true,
            (Some(a_val), None) if a_val == "*" => true,
            // A is specific, B matches anything -> not broader
            (Some(_), None) => false,
            // Both specific
            (Some(a_val), Some(b_val)) => {
                // Exact match
                if a_val.eq_ignore_ascii_case(b_val) {
                    return true;
                }
                // A is wildcard
                if a_val == "*" {
                    return true;
                }
                // Both wildcards - would need sophisticated analysis
                // For now, assume not broader unless exact match
                false
            }
        }
    }

    /// Check if two rules potentially conflict (overlap but differ).
    fn potentially_conflicts(&self, a: &Rule, b: &Rule) -> bool {
        // Skip if either is disabled
        if !a.enabled || !b.enabled {
            return false;
        }

        // Skip if same priority (execution order is deterministic)
        if a.priority == b.priority {
            return false;
        }

        // Check if match patterns overlap
        if !self.patterns_overlap(&a.match_spec, &b.match_spec) {
            return false;
        }

        // Check if actions differ
        if !self.actions_differ(&a.action, &b.action) {
            return false;
        }

        true
    }

    /// Check if two match specs can match the same request.
    fn patterns_overlap(&self, a: &MatchSpec, b: &MatchSpec) -> bool {
        // Method overlap
        if !self.optional_fields_overlap(&a.method, &b.method) {
            return false;
        }

        // Host overlap
        if !self.patterns_overlap_check(&a.host, &b.host) {
            return false;
        }

        // Path overlap
        if !self.patterns_overlap_check(&a.path, &b.path) {
            return false;
        }

        // If both have specific query/header constraints, check for overlap
        // For simplicity, assume overlap if both have constraints
        // (detailed overlap analysis would be complex)

        true
    }

    /// Check if two optional fields can match the same value.
    fn optional_fields_overlap(&self, a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            // Both match anything -> overlap
            (None, None) => true,
            // One matches anything -> overlap
            (None, Some(_)) | (Some(_), None) => true,
            // Both specific -> must match
            (Some(a_val), Some(b_val)) => a_val.eq_ignore_ascii_case(b_val),
        }
    }

    /// Check if two patterns can match the same string.
    fn patterns_overlap_check(&self, a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            // Both match anything -> overlap
            (None, None) => true,
            // One matches anything -> overlap
            (None, Some(_)) | (Some(_), None) => true,
            // Wildcards overlap with everything
            (Some(a_val), _) if a_val == "*" => true,
            (_, Some(b_val)) if b_val == "*" => true,
            // Both specific
            (Some(a_val), Some(b_val)) => {
                // Exact match -> overlap
                if a_val.eq_ignore_ascii_case(b_val) {
                    return true;
                }
                // Both contain wildcards -> assume overlap (conservative)
                if (a_val.contains('*') && b_val.contains('*'))
                    || a_val.contains('*')
                    || b_val.contains('*')
                {
                    return true;
                }
                // No overlap
                false
            }
        }
    }

    /// Check if two actions are meaningfully different.
    fn actions_differ(&self, a: &Action, b: &Action) -> bool {
        match (a, b) {
            // Same action type
            (Action::Pass, Action::Pass) => false,
            (Action::Fail { .. }, Action::Fail { .. }) => false,
            (
                Action::Respond { status: s1, .. },
                Action::Respond { status: s2, .. },
            ) => {
                // Consider different if status codes differ
                s1 != s2
            }
            // Different action types
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_rule(id: &str, priority: i32, enabled: bool) -> Rule {
        Rule {
            id: id.to_string(),
            priority,
            enabled,
            description: None,
            match_spec: MatchSpec {
                host: Some("test.local".to_string()),
                path: Some("/api".to_string()),
                ..Default::default()
            },
            action: Action::Respond {
                status: 200,
                headers: HashMap::new(),
                body: BodySource::Inline {
                    inline: "test".to_string(),
                },
                delay_ms: None,
            },
        }
    }

    #[test]
    fn detects_duplicate_ids() {
        let rules = vec![
            test_rule("dup", 100, true),
            test_rule("dup", 200, true),
        ];

        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let dups: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::DuplicateId)
            .collect();

        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].severity, Severity::Error);
    }

    #[test]
    fn detects_missing_fixture() {
        let mut rule = test_rule("test", 100, true);
        rule.action = Action::Respond {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: BodySource::File {
                file: "nonexistent.json".to_string(),
            },
            delay_ms: None,
        };

        let rules = vec![rule];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let missing: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::MissingFixture)
            .collect();

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].severity, Severity::Error);
    }

    #[test]
    fn detects_invalid_regex() {
        let mut rule = test_rule("test", 100, true);
        rule.match_spec.path_regex = Some("[invalid(".to_string());

        let rules = vec![rule];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let regex_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::InvalidRegex)
            .collect();

        assert_eq!(regex_issues.len(), 1);
        assert_eq!(regex_issues[0].severity, Severity::Error);
    }

    #[test]
    fn detects_unreachable_rule() {
        let mut broad = test_rule("broad", 200, true);
        broad.match_spec.path = Some("*".to_string()); // Matches everything

        let narrow = test_rule("narrow", 100, true);
        // narrow has same host, specific path

        let rules = vec![broad, narrow];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let unreachable: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::Unreachable)
            .collect();

        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0].rule_id, "narrow");
        assert_eq!(unreachable[0].related_rule, Some("broad".to_string()));
    }

    #[test]
    fn detects_conflicting_rules() {
        let mut rule1 = test_rule("rule1", 200, true);
        rule1.action = Action::Respond {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: BodySource::Inline {
                inline: "ok".to_string(),
            },
            delay_ms: None,
        };

        let mut rule2 = test_rule("rule2", 100, true);
        rule2.action = Action::Respond {
            status: 500,
            headers: std::collections::HashMap::new(),
            body: BodySource::Inline {
                inline: "error".to_string(),
            },
            delay_ms: None,
        };

        let rules = vec![rule1, rule2];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let conflicts: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::Conflict)
            .collect();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, Severity::Warning);
    }

    #[test]
    fn detects_disabled_rules() {
        let rule = test_rule("disabled", 100, false);

        let rules = vec![rule];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let disabled: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::DisabledRule)
            .collect();

        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0].severity, Severity::Info);
    }

    #[test]
    fn detects_low_priority() {
        let rule = test_rule("low", 0, true);

        let rules = vec![rule];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        let low: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::LowPriority)
            .collect();

        assert_eq!(low.len(), 1);
        assert_eq!(low[0].severity, Severity::Info);
    }

    #[test]
    fn no_issues_for_valid_rules() {
        let rule1 = test_rule("rule1", 100, true);
        let mut rule2 = test_rule("rule2", 200, true);
        rule2.match_spec.path = Some("/other".to_string()); // Different path

        let rules = vec![rule1, rule2];
        let validator = RuleValidator::new(&rules, Path::new("/tmp"));
        let issues = validator.validate();

        // Should only have info-level issues (if any)
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 0);
    }
}
