//! Mock rule runtime for deterministic request interception.
//!
//! This crate provides a YAML-based rule system for matching HTTP requests and returning
//! mock responses. Rules are evaluated in priority order (higher priority first), and the
//! first matching rule determines the response.

mod schema;
mod matcher;
mod fixtures;
mod resolver;
mod registry;
mod validation;

pub use schema::{MockConfig, Rule, MatchSpec, Action, BodySource};
pub use fixtures::{FixtureResolver, FixtureError};
pub use resolver::RuleMockResolver;
pub use registry::{DynamicMockRegistry, RegistryError};
pub use validation::{RuleValidator, ValidationIssue, Severity, IssueKind};
