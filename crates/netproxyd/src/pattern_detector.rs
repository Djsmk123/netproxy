//! Smart pattern detection for URL paths.
//!
//! Analyzes URL paths to detect dynamic segments (UUIDs, IDs, timestamps) and suggests
//! match patterns with different specificity levels.

use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;

lazy_static! {
    /// UUID v4 pattern: 8-4-4-4-12 hex
    static ref UUID_REGEX: Regex = Regex::new(
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
    ).unwrap();

    /// Date pattern: YYYY-MM-DD
    static ref DATE_REGEX: Regex = Regex::new(
        r"^\d{4}-\d{2}-\d{2}$"
    ).unwrap();

    /// Slug pattern: lowercase alphanumeric with hyphens
    static ref SLUG_REGEX: Regex = Regex::new(
        r"^[a-z0-9]+(-[a-z0-9]+)+$"
    ).unwrap();
}

/// Common API path keywords that are typically static
const STATIC_KEYWORDS: &[&str] = &[
    "api", "v1", "v2", "v3", "admin", "public", "private",
    "users", "posts", "products", "orders", "items", "accounts",
    "search", "list", "detail", "profile", "settings", "config",
    "auth", "login", "logout", "signup", "register",
    "dashboard", "analytics", "reports", "exports",
    "messages", "notifications", "alerts", "comments",
];

/// Type of a path segment
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PathSegmentType {
    Static { value: String },
    NumericId,
    Uuid,
    Slug,
    Date,
    Timestamp,
    Hex,
    Other { value: String },
}

/// Analysis of a URL path
#[derive(Debug, Clone, Serialize)]
pub struct PathAnalysis {
    pub original: String,
    pub segments: Vec<PathSegmentType>,
    pub suggestions: Vec<PatternSuggestion>,
}

/// A suggested pattern with a specificity level
#[derive(Debug, Clone, Serialize)]
pub struct PatternSuggestion {
    pub specificity: Specificity,
    pub pattern: String,
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub example_matches: Vec<String>,
}

/// Specificity level for pattern matching
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Specificity {
    Exact,
    Moderate,
    Broad,
}

/// Pattern detector for URL paths
pub struct PatternDetector;

impl PatternDetector {
    /// Analyze a URL path and generate pattern suggestions
    pub fn analyze(path: &str) -> PathAnalysis {
        let segments: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let analyzed_segments: Vec<PathSegmentType> = segments
            .iter()
            .map(|seg| Self::classify_segment(seg))
            .collect();

        let suggestions = Self::generate_suggestions(path, &analyzed_segments);

        PathAnalysis {
            original: path.to_string(),
            segments: analyzed_segments,
            suggestions,
        }
    }

    /// Classify a single path segment
    fn classify_segment(segment: &str) -> PathSegmentType {
        // UUID: 8-4-4-4-12 hex pattern
        if UUID_REGEX.is_match(segment) {
            return PathSegmentType::Uuid;
        }

        // Numeric ID: all digits
        if !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()) {
            // Check if it's a 10-digit timestamp
            if segment.len() == 10 {
                return PathSegmentType::Timestamp;
            }
            return PathSegmentType::NumericId;
        }

        // Date: YYYY-MM-DD
        if DATE_REGEX.is_match(segment) {
            return PathSegmentType::Date;
        }

        // Hex: only 0-9a-f, at least 8 characters
        if segment.len() >= 8
            && segment.len() <= 64
            && segment.chars().all(|c| c.is_ascii_hexdigit())
            && !segment.chars().all(|c| c.is_ascii_digit())
        {
            return PathSegmentType::Hex;
        }

        // Slug: contains hyphens, lowercase alphanumeric
        if SLUG_REGEX.is_match(segment) {
            return PathSegmentType::Slug;
        }

        // Static segment (API keywords, etc.)
        if Self::is_static_keyword(segment) {
            return PathSegmentType::Static {
                value: segment.to_string(),
            };
        }

        // Unknown dynamic content
        PathSegmentType::Other {
            value: segment.to_string(),
        }
    }

    /// Check if a segment is a known static keyword
    fn is_static_keyword(segment: &str) -> bool {
        STATIC_KEYWORDS.contains(&segment.to_lowercase().as_str())
    }

    /// Generate pattern suggestions with different specificity levels
    fn generate_suggestions(
        path: &str,
        segments: &[PathSegmentType],
    ) -> Vec<PatternSuggestion> {
        let mut suggestions = Vec::new();

        // 1. Exact match (lowest risk)
        suggestions.push(PatternSuggestion {
            specificity: Specificity::Exact,
            pattern: path.to_string(),
            description: "Match this exact path only".to_string(),
            example_matches: vec![path.to_string()],
        });

        // 2. Moderate - replace detected IDs/UUIDs with wildcards
        let moderate_pattern: Vec<String> = segments
            .iter()
            .map(|seg| match seg {
                PathSegmentType::NumericId
                | PathSegmentType::Uuid
                | PathSegmentType::Hex
                | PathSegmentType::Date
                | PathSegmentType::Timestamp
                | PathSegmentType::Slug => "*".to_string(),
                PathSegmentType::Static { value } => value.clone(),
                PathSegmentType::Other { value } => value.clone(),
            })
            .collect();

        let moderate_path = format!("/{}", moderate_pattern.join("/"));

        // Only add moderate suggestion if it differs from exact
        if moderate_path != path {
            let example_variant = Self::generate_example_variant(path, segments);
            suggestions.push(PatternSuggestion {
                specificity: Specificity::Moderate,
                pattern: moderate_path,
                description: "Replace IDs/UUIDs with wildcards".to_string(),
                example_matches: vec![path.to_string(), example_variant],
            });
        }

        // 3. Broad - wildcard all segments after static prefix
        let static_prefix = Self::find_static_prefix(segments);
        if !static_prefix.is_empty() {
            let broad_path = format!("/{}/*", static_prefix);
            suggestions.push(PatternSuggestion {
                specificity: Specificity::Broad,
                pattern: broad_path.clone(),
                description: "Match all requests under this path".to_string(),
                example_matches: vec![
                    path.to_string(),
                    format!("/{}/anything/here", static_prefix),
                ],
            });
        }

        suggestions
    }

    /// Generate an example variant of the path with different dynamic values
    fn generate_example_variant(_original: &str, segments: &[PathSegmentType]) -> String {
        let variant_segments: Vec<String> = segments
            .iter()
            .map(|seg| match seg {
                PathSegmentType::NumericId => "67890".to_string(),
                PathSegmentType::Uuid => "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
                PathSegmentType::Hex => "deadbeef".to_string(),
                PathSegmentType::Date => "2024-12-31".to_string(),
                PathSegmentType::Timestamp => "1735689600".to_string(),
                PathSegmentType::Slug => "another-example-slug".to_string(),
                PathSegmentType::Static { value } => value.clone(),
                PathSegmentType::Other { value } => value.clone(),
            })
            .collect();

        format!("/{}", variant_segments.join("/"))
    }

    /// Find the static prefix for broad matching
    /// Stops after version indicators (v1, v2, etc.) or after the first resource name
    fn find_static_prefix(segments: &[PathSegmentType]) -> String {
        let mut prefix_segments = Vec::new();
        let mut seen_version = false;

        for seg in segments {
            match seg {
                PathSegmentType::Static { value } => {
                    // Check if this is a version indicator
                    if value.starts_with('v') && value[1..].chars().all(|c| c.is_ascii_digit()) {
                        prefix_segments.push(value.clone());
                        seen_version = true;
                    } else if value == "api" || !seen_version {
                        // Include "api" or segments before version
                        prefix_segments.push(value.clone());
                        // If we've seen a version and this is not "api", stop here
                        if seen_version && value != "api" {
                            break;
                        }
                    } else {
                        // After version, stop at the first non-api segment
                        break;
                    }
                }
                _ => break, // Stop at first dynamic segment
            }
        }

        prefix_segments.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_numeric_id() {
        let analysis = PatternDetector::analyze("/api/users/12345");
        assert_eq!(analysis.segments.len(), 3);
        assert!(matches!(
            analysis.segments[0],
            PathSegmentType::Static { .. }
        ));
        assert!(matches!(
            analysis.segments[1],
            PathSegmentType::Static { .. }
        ));
        assert!(matches!(analysis.segments[2], PathSegmentType::NumericId));

        let moderate = analysis
            .suggestions
            .iter()
            .find(|s| s.specificity == Specificity::Moderate)
            .unwrap();
        assert_eq!(moderate.pattern, "/api/users/*");
    }

    #[test]
    fn detects_uuid() {
        let analysis = PatternDetector::analyze("/orders/550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(analysis.segments.len(), 2);
        assert!(matches!(analysis.segments[1], PathSegmentType::Uuid));

        let moderate = analysis
            .suggestions
            .iter()
            .find(|s| s.specificity == Specificity::Moderate)
            .unwrap();
        assert_eq!(moderate.pattern, "/orders/*");
    }

    #[test]
    fn detects_date() {
        let analysis = PatternDetector::analyze("/logs/2024-10-17/errors");
        assert_eq!(analysis.segments.len(), 3);
        assert!(matches!(analysis.segments[1], PathSegmentType::Date));

        let moderate = analysis
            .suggestions
            .iter()
            .find(|s| s.specificity == Specificity::Moderate)
            .unwrap();
        assert_eq!(moderate.pattern, "/logs/*/errors");
    }

    #[test]
    fn detects_timestamp() {
        let analysis = PatternDetector::analyze("/events/1697558400");
        assert_eq!(analysis.segments.len(), 2);
        assert!(matches!(
            analysis.segments[1],
            PathSegmentType::Timestamp
        ));
    }

    #[test]
    fn detects_hex() {
        let analysis = PatternDetector::analyze("/objects/a1b2c3d4ef");
        assert_eq!(analysis.segments.len(), 2);
        assert!(matches!(analysis.segments[1], PathSegmentType::Hex));
    }

    #[test]
    fn detects_slug() {
        let analysis = PatternDetector::analyze("/posts/my-awesome-post-2024");
        assert_eq!(analysis.segments.len(), 2);
        assert!(matches!(analysis.segments[1], PathSegmentType::Slug));
    }

    #[test]
    fn generates_three_specificity_levels() {
        let analysis = PatternDetector::analyze("/api/v1/users/123/posts/456");
        assert_eq!(analysis.suggestions.len(), 3);
        assert!(analysis
            .suggestions
            .iter()
            .any(|s| s.specificity == Specificity::Exact));
        assert!(analysis
            .suggestions
            .iter()
            .any(|s| s.specificity == Specificity::Moderate));
        assert!(analysis
            .suggestions
            .iter()
            .any(|s| s.specificity == Specificity::Broad));
    }

    #[test]
    fn handles_path_with_no_dynamic_segments() {
        let analysis = PatternDetector::analyze("/api/v1/users");
        // Should only have exact and broad (no moderate since nothing to replace)
        assert!(analysis
            .suggestions
            .iter()
            .any(|s| s.specificity == Specificity::Exact));
        assert!(analysis
            .suggestions
            .iter()
            .any(|s| s.specificity == Specificity::Broad));
    }

    #[test]
    fn handles_mixed_segments() {
        let analysis = PatternDetector::analyze("/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000/comments");

        // Check segments
        assert!(matches!(analysis.segments[0], PathSegmentType::Static { .. }));
        assert!(matches!(analysis.segments[1], PathSegmentType::Static { .. }));
        assert!(matches!(analysis.segments[2], PathSegmentType::Static { .. }));
        assert!(matches!(analysis.segments[3], PathSegmentType::NumericId));
        assert!(matches!(analysis.segments[4], PathSegmentType::Static { .. }));
        assert!(matches!(analysis.segments[5], PathSegmentType::Uuid));
        assert!(matches!(analysis.segments[6], PathSegmentType::Static { .. }));

        // Check moderate pattern
        let moderate = analysis
            .suggestions
            .iter()
            .find(|s| s.specificity == Specificity::Moderate)
            .unwrap();
        assert_eq!(moderate.pattern, "/api/v1/users/*/posts/*/comments");

        // Check broad pattern
        let broad = analysis
            .suggestions
            .iter()
            .find(|s| s.specificity == Specificity::Broad)
            .unwrap();
        assert_eq!(broad.pattern, "/api/v1/*");
    }

    #[test]
    fn exact_pattern_always_matches_original() {
        let paths = vec![
            "/api/users/123",
            "/orders/550e8400-e29b-41d4-a716-446655440000",
            "/logs/2024-10-17",
            "/posts/my-slug",
        ];

        for path in paths {
            let analysis = PatternDetector::analyze(path);
            let exact = analysis
                .suggestions
                .iter()
                .find(|s| s.specificity == Specificity::Exact)
                .unwrap();
            assert_eq!(exact.pattern, path);
        }
    }
}
