//! The engine contract. Everything above this line (store, control API, UI, MCP) talks to
//! `ProxyEngine`, never to a concrete proxy library.

use crate::{
    body::{BodySink, NoBodies},
    bus::EventSink,
    ca::CaMaterial,
};
use bytes::Bytes;
use futures::future::BoxFuture;
use np_model::Headers;
use std::{net::SocketAddr, sync::Arc, time::Duration};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("cannot bind {0}: {1}")]
    Bind(SocketAddr, std::io::Error),
    #[error("CA error: {0}")]
    Ca(String),
    #[error("TLS configuration error: {0}")]
    Tls(String),
    #[error("{0}")]
    Other(String),
}

/// Read-only view of a request handed to the mock runtime. The resolver is called **synchronously
/// on the data path**, so it must not perform I/O (fixtures are pre-resolved in M3).
pub struct RequestView<'a> {
    pub method: &'a str,
    pub uri: &'a str,
    pub host: Option<&'a str>,
    pub path: &'a str,
    pub headers: &'a Headers,
}

pub struct MockResponse {
    pub rule: String,
    pub status: u16,
    pub headers: Headers,
    pub body: Bytes,
}

pub enum MockDecision {
    Pass,
    Respond(MockResponse),
}

pub trait MockResolver: Send + Sync {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision;
}

pub struct NoMocks;
impl MockResolver for NoMocks {
    fn resolve(&self, _: &RequestView<'_>) -> MockDecision {
        MockDecision::Pass
    }
}

pub struct EngineConfig {
    pub listen: SocketAddr,
    pub ca: CaMaterial,
    /// Extra upstream trust roots (PEM), added to the public roots. Used by tests/enterprise CAs.
    pub upstream_roots_pem: Option<Vec<u8>>,
    pub connect_timeout: Duration,
    /// Bytes of each body kept as a preview in `BodyEnd`.
    pub preview_bytes: usize,
    pub mock: Arc<dyn MockResolver>,
    pub bodies: Arc<dyn BodySink>,
}

impl EngineConfig {
    pub fn new(listen: SocketAddr, ca: CaMaterial) -> Self {
        Self {
            listen,
            ca,
            upstream_roots_pem: None,
            connect_timeout: Duration::from_secs(10),
            preview_bytes: 4096,
            mock: Arc::new(NoMocks),
            bodies: Arc::new(NoBodies),
        }
    }
}

pub trait EngineHandle: Send {
    fn local_addr(&self) -> SocketAddr;
    /// Stops accepting, drains briefly, then aborts remaining connections. Emits `EngineStopped`.
    fn shutdown(self: Box<Self>) -> BoxFuture<'static, ()>;
}

pub trait ProxyEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(
        &self,
        cfg: EngineConfig,
        sink: EventSink,
    ) -> BoxFuture<'static, Result<Box<dyn EngineHandle>, EngineError>>;
}
