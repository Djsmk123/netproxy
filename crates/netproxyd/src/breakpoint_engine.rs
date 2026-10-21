//! Breakpoint engine for pausing and inspecting requests.

use np_model::Headers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// A request that has been paused by a breakpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PausedRequest {
    pub id: String,
    pub flow_id: u64,
    pub method: String,
    pub uri: String,
    pub host: Option<String>,
    pub headers: Headers,
    pub body_preview: Option<String>,
    pub breakpoint_id: String,
    pub paused_at: std::time::SystemTime,
}

/// Action to take on a paused request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RequestAction {
    /// Continue with the request as-is
    Continue,
    /// Continue with modifications
    Modify {
        method: Option<String>,
        uri: Option<String>,
        headers: Option<Headers>,
    },
    /// Cancel the request and return an error
    Cancel {
        status: u16,
        message: String,
    },
}

/// Waiter for a paused request to be released.
struct RequestWaiter {
    tx: oneshot::Sender<RequestAction>,
    request: PausedRequest,
}

/// Store for paused requests.
#[derive(Default)]
pub struct PausedRequestStore {
    requests: HashMap<String, RequestWaiter>,
}

impl PausedRequestStore {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }

    /// Pause a request and wait for an action.
    pub fn pause_request(
        &mut self,
        request: PausedRequest,
    ) -> oneshot::Receiver<RequestAction> {
        let id = request.id.clone();
        let (tx, rx) = oneshot::channel();

        self.requests.insert(
            id,
            RequestWaiter {
                tx,
                request,
            },
        );

        rx
    }

    /// List all paused requests.
    pub fn list(&self) -> Vec<PausedRequest> {
        self.requests
            .values()
            .map(|w| w.request.clone())
            .collect()
    }

    /// Release a paused request with an action.
    pub fn release(&mut self, id: &str, action: RequestAction) -> Result<(), String> {
        if let Some(waiter) = self.requests.remove(id) {
            waiter
                .tx
                .send(action)
                .map_err(|_| "request already released".to_string())?;
            Ok(())
        } else {
            Err(format!("paused request not found: {}", id))
        }
    }

    /// Cancel all paused requests (e.g., on daemon shutdown).
    pub fn cancel_all(&mut self) {
        for (_, waiter) in self.requests.drain() {
            let _ = waiter.tx.send(RequestAction::Cancel {
                status: 502,
                message: "Breakpoint interrupted".to_string(),
            });
        }
    }
}

/// Pattern matcher for breakpoints.
pub fn matches_pattern(url: &str, pattern: &str) -> bool {
    // Simple wildcard matching
    if pattern == "*" {
        return true;
    }

    // Convert wildcard pattern to regex-like matching
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 1 {
        // No wildcards, exact match
        return url == pattern;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            if i == 0 {
                // Leading wildcard
                continue;
            }
            if i == parts.len() - 1 {
                // Trailing wildcard
                return true;
            }
            continue;
        }

        if let Some(idx) = url[pos..].find(part) {
            pos += idx + part.len();
        } else {
            return false;
        }
    }

    // If there was a trailing wildcard, we already returned true
    // Otherwise, check if we consumed the entire URL
    parts.last().map_or(true, |last| last.is_empty() || pos == url.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern("https://api.example.com/users", "*"));
        assert!(matches_pattern(
            "https://api.example.com/users",
            "https://api.example.com/*"
        ));
        assert!(matches_pattern(
            "https://api.example.com/users/123",
            "https://api.example.com/users/*"
        ));
        assert!(matches_pattern(
            "https://api.example.com/v1/users",
            "https://*/users"
        ));
        assert!(!matches_pattern(
            "https://api.example.com/posts",
            "https://api.example.com/users"
        ));
        assert!(matches_pattern(
            "https://api.example.com/users",
            "https://api.example.com/users"
        ));
    }
}
