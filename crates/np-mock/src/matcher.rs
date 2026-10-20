//! Pattern matching logic for mock rules.

use crate::schema::{MatchSpec, Rule};
use np_engine::engine::RequestView;
use np_model::Headers;
use regex::Regex;
use std::collections::HashMap;

impl Rule {
    /// Check if this rule matches the given request.
    pub fn matches(&self, req: &RequestView<'_>) -> bool {
        if !self.enabled {
            return false;
        }

        self.match_spec.matches(req)
    }
}

impl MatchSpec {
    /// Check if this match specification matches the given request.
    pub fn matches(&self, req: &RequestView<'_>) -> bool {
        // Method matching (case-insensitive)
        if let Some(ref method) = self.method {
            if !req.method.eq_ignore_ascii_case(method) {
                return false;
            }
        }

        // Host matching (supports wildcards)
        if let Some(ref host_pattern) = self.host {
            if !matches_pattern(req.host, host_pattern) {
                return false;
            }
        }

        // Path matching (exact or wildcard)
        if let Some(ref path_pattern) = self.path {
            if !matches_pattern(Some(req.path), path_pattern) {
                return false;
            }
        }

        // Path regex matching
        if let Some(ref path_regex) = self.path_regex {
            match Regex::new(path_regex) {
                Ok(re) => {
                    if !re.is_match(req.path) {
                        return false;
                    }
                }
                Err(_) => {
                    // Invalid regex in config, fail to match
                    return false;
                }
            }
        }

        // Query param matching
        if let Some(ref query_params) = self.query {
            if !matches_query(req.uri, query_params) {
                return false;
            }
        }

        // Header matching (supports wildcards)
        if let Some(ref header_patterns) = self.headers {
            if !matches_headers(req.headers, header_patterns) {
                return false;
            }
        }

        true
    }
}

/// Match a value against a pattern that may contain wildcards.
fn matches_pattern(value: Option<&str>, pattern: &str) -> bool {
    match (value, pattern) {
        (_, "*") => true, // Wildcard matches anything, including None
        (Some(v), p) if p.contains('*') => glob_match(v, p),
        (Some(v), p) => v.eq_ignore_ascii_case(p),
        (None, _) => false,
    }
}

/// Simple glob matching with * wildcard support.
/// Patterns like "*.example.com" or "/api/*" are supported.
fn glob_match(value: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            if i == 0 || i == parts.len() - 1 {
                // Leading or trailing wildcard
                continue;
            }
            // Empty part in the middle (consecutive *)
            continue;
        }

        // Find this part in the remaining value
        if let Some(idx) = value[pos..].to_lowercase().find(&part.to_lowercase()) {
            // For the first part (no leading *), it must start at position 0
            if i == 0 && !pattern.starts_with('*') && idx != 0 {
                return false;
            }
            pos += idx + part.len();
        } else {
            return false;
        }
    }

    // If pattern doesn't end with *, the value must be fully consumed
    if !pattern.ends_with('*') && pos != value.len() {
        return false;
    }

    true
}

/// Match query parameters in the URI.
fn matches_query(uri: &str, params: &HashMap<String, String>) -> bool {
    // Parse query string from URI
    let query = match uri.find('?') {
        Some(idx) => &uri[idx + 1..],
        None => return params.is_empty(),
    };

    // Parse query params into a map
    let uri_params: HashMap<String, String> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((
                parts.next()?.to_string(),
                parts.next().unwrap_or("").to_string(),
            ))
        })
        .collect();

    // Check that all required params match
    for (key, value) in params {
        match uri_params.get(key) {
            Some(v) if matches_pattern(Some(v), value) => {}
            _ => return false,
        }
    }

    true
}

/// Match headers against patterns.
fn matches_headers(headers: &Headers, patterns: &HashMap<String, String>) -> bool {
    for (pattern_key, pattern_value) in patterns {
        // Find header with matching key (case-insensitive)
        let found = headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case(pattern_key) && matches_pattern(Some(v), pattern_value)
        });

        if !found {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use np_engine::engine::RequestView;

    fn test_request<'a>(
        method: &'a str,
        host: Option<&'a str>,
        path: &'a str,
        uri: &'a str,
        headers: &'a Headers,
    ) -> RequestView<'a> {
        RequestView {
            method,
            uri,
            host,
            path,
            headers,
        }
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("api.example.com", "*.example.com"));
        assert!(glob_match("staging.example.com", "*.example.com"));
        assert!(!glob_match("other.com", "*.example.com"));

        assert!(glob_match("/api/users", "/api/*"));
        assert!(glob_match("/api/posts", "/api/*"));
        assert!(!glob_match("/v2/users", "/api/*"));

        assert!(glob_match("test.example.com", "*.*.com"));
        assert!(glob_match("api.staging.example.com", "*example.com"));

        // Exact match when no wildcard
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("notexact", "exact"));
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern(Some("test"), "*"));
        assert!(matches_pattern(None, "*"));
        assert!(matches_pattern(Some("test.com"), "test.com"));
        assert!(matches_pattern(Some("Test.Com"), "test.com")); // case-insensitive
        assert!(matches_pattern(Some("api.test.com"), "*.test.com"));
        assert!(!matches_pattern(Some("other.com"), "test.com"));
        assert!(!matches_pattern(None, "test.com"));
    }

    #[test]
    fn test_method_matching() {
        let spec = MatchSpec {
            method: Some("GET".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req_get = test_request("GET", None, "/", "http://test.com/", &headers);
        let req_post = test_request("POST", None, "/", "http://test.com/", &headers);

        assert!(spec.matches(&req_get));
        assert!(!spec.matches(&req_post));
    }

    #[test]
    fn test_host_matching() {
        let spec = MatchSpec {
            host: Some("api.example.com".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            Some("api.example.com"),
            "/",
            "http://api.example.com/",
            &headers,
        );
        let req2 = test_request("GET", Some("other.com"), "/", "http://other.com/", &headers);

        assert!(spec.matches(&req1));
        assert!(!spec.matches(&req2));
    }

    #[test]
    fn test_wildcard_host() {
        let spec = MatchSpec {
            host: Some("*.example.com".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            Some("api.example.com"),
            "/",
            "http://api.example.com/",
            &headers,
        );
        let req2 = test_request(
            "GET",
            Some("staging.example.com"),
            "/",
            "http://staging.example.com/",
            &headers,
        );
        let req3 = test_request("GET", Some("other.com"), "/", "http://other.com/", &headers);

        assert!(spec.matches(&req1));
        assert!(spec.matches(&req2));
        assert!(!spec.matches(&req3));
    }

    #[test]
    fn test_path_matching() {
        let spec = MatchSpec {
            path: Some("/api/users".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            None,
            "/api/users",
            "http://test.com/api/users",
            &headers,
        );
        let req2 = test_request(
            "GET",
            None,
            "/api/posts",
            "http://test.com/api/posts",
            &headers,
        );

        assert!(spec.matches(&req1));
        assert!(!spec.matches(&req2));
    }

    #[test]
    fn test_wildcard_path() {
        let spec = MatchSpec {
            path: Some("/api/*".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            None,
            "/api/users",
            "http://test.com/api/users",
            &headers,
        );
        let req2 = test_request(
            "GET",
            None,
            "/api/posts",
            "http://test.com/api/posts",
            &headers,
        );
        let req3 = test_request(
            "GET",
            None,
            "/v2/users",
            "http://test.com/v2/users",
            &headers,
        );

        assert!(spec.matches(&req1));
        assert!(spec.matches(&req2));
        assert!(!spec.matches(&req3));
    }

    #[test]
    fn test_path_regex() {
        let spec = MatchSpec {
            path_regex: Some(r"/v1/users/\d+".to_string()),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            None,
            "/v1/users/123",
            "http://test.com/v1/users/123",
            &headers,
        );
        let req2 = test_request(
            "GET",
            None,
            "/v1/users/abc",
            "http://test.com/v1/users/abc",
            &headers,
        );

        assert!(spec.matches(&req1));
        assert!(!spec.matches(&req2));
    }

    #[test]
    fn test_query_matching() {
        let mut query = HashMap::new();
        query.insert("api_key".to_string(), "test123".to_string());

        let spec = MatchSpec {
            query: Some(query),
            ..Default::default()
        };

        let headers = vec![];
        let req1 = test_request(
            "GET",
            None,
            "/api",
            "http://test.com/api?api_key=test123",
            &headers,
        );
        let req2 = test_request(
            "GET",
            None,
            "/api",
            "http://test.com/api?api_key=other",
            &headers,
        );
        let req3 = test_request("GET", None, "/api", "http://test.com/api", &headers);

        assert!(spec.matches(&req1));
        assert!(!spec.matches(&req2));
        assert!(!spec.matches(&req3));
    }

    #[test]
    fn test_header_matching() {
        let mut headers_pattern = HashMap::new();
        headers_pattern.insert("authorization".to_string(), "Bearer *".to_string());

        let spec = MatchSpec {
            headers: Some(headers_pattern),
            ..Default::default()
        };

        let headers = vec![("Authorization".to_string(), "Bearer token123".to_string())];
        let req = test_request("GET", None, "/", "http://test.com/", &headers);

        assert!(spec.matches(&req));

        let headers2 = vec![("Authorization".to_string(), "Basic abc".to_string())];
        let req2 = test_request("GET", None, "/", "http://test.com/", &headers2);

        assert!(!spec.matches(&req2));
    }

    #[test]
    fn test_disabled_rule() {
        use crate::schema::{Action, Rule};

        let rule = Rule {
            id: "test".to_string(),
            priority: 0,
            enabled: false,
            description: None,
            match_spec: MatchSpec::default(),
            action: Action::Pass,
        };

        let headers = vec![];
        let req = test_request("GET", None, "/", "http://test.com/", &headers);
        assert!(!rule.matches(&req));
    }
}
