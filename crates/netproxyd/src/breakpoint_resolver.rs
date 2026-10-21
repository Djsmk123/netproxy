//! BreakpointMockResolver that pauses requests matching breakpoint patterns.

use crate::{
    breakpoint_engine::{matches_pattern, PausedRequest, PausedRequestStore, RequestAction},
    daemon::{Breakpoint, BreakpointStore},
};
use np_engine::{MockDecision, MockResolver, RequestView};
use np_model::Headers;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

/// Resolver that checks breakpoints before delegating to the inner mock resolver.
pub struct BreakpointMockResolver {
    inner: Arc<dyn MockResolver>,
    breakpoints: Arc<Mutex<BreakpointStore>>,
    paused_requests: Arc<Mutex<PausedRequestStore>>,
    flow_counter: AtomicU64,
}

impl BreakpointMockResolver {
    pub fn new(
        inner: Arc<dyn MockResolver>,
        breakpoints: Arc<Mutex<BreakpointStore>>,
        paused_requests: Arc<Mutex<PausedRequestStore>>,
    ) -> Self {
        Self {
            inner,
            breakpoints,
            paused_requests,
            flow_counter: AtomicU64::new(1),
        }
    }

    fn check_breakpoints(&self, req: &RequestView<'_>) -> Option<(Breakpoint, u64)> {
        let breakpoints = self.breakpoints.lock().unwrap();
        let flow_id = self.flow_counter.fetch_add(1, Ordering::Relaxed);

        for bp in breakpoints.list() {
            if !bp.enabled {
                continue;
            }

            if matches_pattern(req.uri, &bp.pattern) {
                return Some((bp, flow_id));
            }
        }

        None
    }
}

impl MockResolver for BreakpointMockResolver {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
        // Check if this request matches any enabled breakpoint
        if let Some((bp, flow_id)) = self.check_breakpoints(req) {
            tracing::info!(
                breakpoint_id = %bp.id,
                pattern = %bp.pattern,
                uri = %req.uri,
                "Request hit breakpoint, pausing"
            );

            // Create paused request
            let paused = PausedRequest {
                id: format!("pause-{}", uuid::Uuid::new_v4()),
                flow_id,
                method: req.method.to_string(),
                uri: req.uri.to_string(),
                host: req.host.map(|s| s.to_string()),
                headers: req.headers.clone(),
                body_preview: None, // TODO: Capture body preview
                breakpoint_id: bp.id.clone(),
                paused_at: std::time::SystemTime::now(),
            };

            let id = paused.id.clone();

            // Pause the request and wait for action
            let rx = {
                let mut store = self.paused_requests.lock().unwrap();
                store.pause_request(paused)
            };

            // Block waiting for the action (this is sync but will block the request)
            match rx.blocking_recv() {
                Ok(RequestAction::Continue) => {
                    tracing::info!(id = %id, "Continuing request");
                    // Let it pass through to the inner resolver
                    self.inner.resolve(req)
                }
                Ok(RequestAction::Modify {
                    method: _,
                    uri: _,
                    headers: _,
                }) => {
                    // TODO: Apply modifications
                    tracing::warn!(id = %id, "Request modifications not yet implemented");
                    self.inner.resolve(req)
                }
                Ok(RequestAction::Cancel { status, message }) => {
                    tracing::info!(id = %id, "Cancelling request");
                    MockDecision::Respond(np_engine::MockResponse {
                        rule: format!("breakpoint-cancel:{}", bp.id),
                        status,
                        headers: Headers::new(),
                        body: message.into_bytes().into(),
                    })
                }
                Err(_) => {
                    // Channel closed, probably daemon shutting down
                    tracing::warn!(id = %id, "Breakpoint interrupted");
                    MockDecision::Respond(np_engine::MockResponse {
                        rule: format!("breakpoint-interrupted:{}", bp.id),
                        status: 502,
                        headers: Headers::new(),
                        body: b"Request interrupted by breakpoint".to_vec().into(),
                    })
                }
            }
        } else {
            // No breakpoint matched, delegate to inner resolver
            self.inner.resolve(req)
        }
    }
}
