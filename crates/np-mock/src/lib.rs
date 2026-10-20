//! Mock rule runtime for deterministic request interception.
//!
//! This crate provides a YAML-based rule system for matching HTTP requests and returning
//! mock responses. Rules are evaluated in priority order (higher priority first), and the
//! first matching rule determines the response.

mod fixtures;
mod matcher;
mod registry;
mod resolver;
mod schema;
mod validation;

pub use fixtures::{FixtureError, FixtureResolver};
pub use registry::{DynamicMockRegistry, RegistryError};
pub use resolver::RuleMockResolver;
pub use schema::{Action, BodySource, MatchSpec, MockConfig, Rule};
pub use validation::{IssueKind, RuleValidator, Severity, ValidationIssue};
