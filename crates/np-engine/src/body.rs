//! Body capture contract. The engine hands body bytes to a [`BodySink`] on the data path, so a sink
//! **must never block**: it queues the bytes (cheap `Bytes` clones) and reports truncation instead.

use bytes::Bytes;
use np_model::{BodyDir, FlowId};

/// Final state of one captured body, reported by [`BodyWriter::finish`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyCapture {
    pub stored_bytes: u64,
    pub truncated: bool,
}

pub trait BodyWriter: Send + Sync {
    /// Append wire bytes. Non-blocking.
    fn write(&mut self, data: &Bytes);
    /// The body ended (or was aborted); `total` is the number of bytes that crossed the proxy.
    fn finish(self: Box<Self>, total: u64) -> BodyCapture;
}

pub trait BodySink: Send + Sync {
    /// `None` = do not capture this body.
    fn open(&self, flow: FlowId, dir: BodyDir) -> Option<Box<dyn BodyWriter>>;
}

pub struct NoBodies;

impl BodySink for NoBodies {
    fn open(&self, _: FlowId, _: BodyDir) -> Option<Box<dyn BodyWriter>> {
        None
    }
}
