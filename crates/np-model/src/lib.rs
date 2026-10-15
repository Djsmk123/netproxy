//! Canonical traffic model shared by the engine, the store, the control API and every client
//! (SwiftUI, CLI, MCP). Wire format: one JSON object per [`Envelope`].
//!
//! Invariants (enforced by the sequencer and checked by `np-engine`'s conformance tests):
//! * `seq` starts at 1 per run and is contiguous. A gap is only ever explained by an
//!   [`Event::EventsDropped`] marker that sits immediately before the first surviving event.
//! * For one flow the events obey the state machine in [`FlowState::apply`].
//! * IDs are unique per run: connections and flows have independent counters.

use serde::{Deserialize, Serialize};

/// Bump on any breaking change to [`Event`] or the control protocol.
pub const SCHEMA_VERSION: u32 = 1;
pub const API_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConnId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowId(pub u64);

pub type Headers = Vec<(String, String)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyDir {
    Req,
    Res,
}

/// Why a client refused our certificate during the TLS handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientTlsClass {
    /// Explicit `unknown_ca` alert: the client does not trust our CA.
    RejectedUnknownCa,
    /// `certificate_unknown` / `bad_certificate` alert (Android apps ignoring user CAs).
    RejectedCertificate,
    /// Client closed without an alert: typical of certificate pinning or an aborted dial.
    ClosedDuringHandshake,
    Other,
}

/// Why an upstream request failed before a response was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowFailClass {
    TlsUntrustedIssuer,
    TlsHostnameMismatch,
    TlsExpired,
    TlsNotYetValid,
    TlsCertificateOther,
    TlsHandshakeFailed,
    ConnectRefused,
    ConnectTimeout,
    DnsFailure,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    EngineStarted { listen: String, engine: String, engine_version: String },
    EngineStopped { reason: String },
    /// `count` events were discarded at the producer edge immediately before this point.
    EventsDropped { count: u64 },

    ConnectionOpened { conn: ConnId, client: String, target: String },
    TlsHello { conn: ConnId, sni: Option<String>, alpn: Vec<String> },
    ClientTlsFailed { conn: Option<ConnId>, class: ClientTlsClass, detail: String },
    ConnectionClosed { conn: ConnId, requests: u64 },

    RequestHead {
        flow: FlowId,
        conn: Option<ConnId>,
        client: String,
        method: String,
        uri: String,
        host: Option<String>,
        http_version: String,
        headers: Headers,
    },
    ResponseHead { flow: FlowId, status: u16, ttfb_us: u64, headers: Headers, mocked: bool },
    BodyFirstByte { flow: FlowId, dir: BodyDir },
    BodyProgress { flow: FlowId, dir: BodyDir, total: u64 },
    BodyEnd {
        flow: FlowId,
        dir: BodyDir,
        total: u64,
        chunks: u64,
        sha256: Option<String>,
        /// First bytes of the body (capped by the engine), base64.
        preview_b64: String,
        /// Bytes persisted to the body store (<= `total`).
        stored_bytes: u64,
        /// True when the stored copy is incomplete (per-body cap, full write queue, or no store).
        truncated: bool,
    },
    BodyError { flow: FlowId, dir: BodyDir, total: u64, error: String },
    BodyAborted { flow: FlowId, dir: BodyDir, total: u64 },

    MockHit { flow: FlowId, rule: String },
    FlowCompleted { flow: FlowId, status: Option<u16>, mocked: bool },
    FlowFailed { flow: FlowId, class: FlowFailClass, detail: String },

    WsMessage { client: String, dir: WsDir, bytes: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsDir {
    ClientToServer,
    ServerToClient,
}

impl Event {
    pub fn flow(&self) -> Option<FlowId> {
        use Event::*;
        match self {
            RequestHead { flow, .. } | ResponseHead { flow, .. } | BodyFirstByte { flow, .. }
            | BodyProgress { flow, .. } | BodyEnd { flow, .. } | BodyError { flow, .. }
            | BodyAborted { flow, .. } | MockHit { flow, .. } | FlowCompleted { flow, .. }
            | FlowFailed { flow, .. } => Some(*flow),
            _ => None,
        }
    }

    pub fn conn(&self) -> Option<ConnId> {
        use Event::*;
        match self {
            ConnectionOpened { conn, .. } | TlsHello { conn, .. } | ConnectionClosed { conn, .. } => Some(*conn),
            ClientTlsFailed { conn, .. } | RequestHead { conn, .. } => *conn,
            _ => None,
        }
    }

    /// Stable snake_case name, identical to the serialized `kind` tag.
    pub fn kind(&self) -> &'static str {
        use Event::*;
        match self {
            EngineStarted { .. } => "engine_started",
            EngineStopped { .. } => "engine_stopped",
            EventsDropped { .. } => "events_dropped",
            ConnectionOpened { .. } => "connection_opened",
            TlsHello { .. } => "tls_hello",
            ClientTlsFailed { .. } => "client_tls_failed",
            ConnectionClosed { .. } => "connection_closed",
            RequestHead { .. } => "request_head",
            ResponseHead { .. } => "response_head",
            BodyFirstByte { .. } => "body_first_byte",
            BodyProgress { .. } => "body_progress",
            BodyEnd { .. } => "body_end",
            BodyError { .. } => "body_error",
            BodyAborted { .. } => "body_aborted",
            MockHit { .. } => "mock_hit",
            FlowCompleted { .. } => "flow_completed",
            FlowFailed { .. } => "flow_failed",
            WsMessage { .. } => "ws_message",
        }
    }
}

/// One sequenced event as stored and sent over the wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Engine run this event belongs to (random per start). `(run, seq)` is globally unique.
    pub run: String,
    pub seq: u64,
    /// Wall-clock microseconds since the Unix epoch at emission.
    pub ts_us: u64,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStateKind {
    /// Request seen, no terminal event yet.
    Open,
    Completed,
    Failed,
    /// Body ended abnormally (client cancelled, upstream reset) without a failure event.
    Aborted,
}

impl FlowStateKind {
    pub fn is_terminal(self) -> bool {
        !matches!(self, FlowStateKind::Open)
    }
}

/// Per-flow state machine used by the store projector and the conformance tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowState {
    pub kind: FlowStateKind,
    pub has_response_head: bool,
    pub req_body_ended: bool,
    pub res_body_ended: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation(pub String);

impl FlowState {
    pub fn new() -> Self {
        Self { kind: FlowStateKind::Open, has_response_head: false, req_body_ended: false, res_body_ended: false }
    }

    /// Applies one flow event; returns a violation if the ordering contract is broken.
    /// `first` is true only for the flow's first event, which must be `RequestHead`.
    pub fn apply(state: Option<Self>, ev: &Event) -> Result<Self, Violation> {
        use Event::*;
        let mut s = match (state, ev) {
            (None, RequestHead { .. }) => return Ok(Self::new()),
            (None, other) => return Err(Violation(format!("first event of a flow was {}", other.kind()))),
            (Some(s), RequestHead { .. }) => return Err(Violation(format!("second request_head for a flow in {:?}", s.kind))),
            (Some(s), _) => s,
        };
        if s.kind.is_terminal() {
            // A body_end / completion may legitimately trail a failure only for the *request* body.
            return match ev {
                BodyEnd { dir: BodyDir::Req, .. } | BodyAborted { dir: BodyDir::Req, .. } => Ok(s),
                other => Err(Violation(format!("{} after terminal state {:?}", other.kind(), s.kind))),
            };
        }
        match ev {
            ResponseHead { .. } => {
                if s.has_response_head {
                    return Err(Violation("duplicate response_head".into()));
                }
                s.has_response_head = true;
            }
            BodyFirstByte { dir, .. } | BodyProgress { dir, .. } => {
                if *dir == BodyDir::Res && !s.has_response_head {
                    return Err(Violation("response body before response_head".into()));
                }
            }
            BodyEnd { dir: BodyDir::Req, .. } => s.req_body_ended = true,
            BodyEnd { dir: BodyDir::Res, .. } => {
                if !s.has_response_head {
                    return Err(Violation("response body_end before response_head".into()));
                }
                s.res_body_ended = true;
            }
            BodyAborted { dir: BodyDir::Req, .. } => {}
            BodyAborted { dir: BodyDir::Res, .. } | BodyError { dir: BodyDir::Res, .. } => s.kind = FlowStateKind::Aborted,
            BodyError { dir: BodyDir::Req, .. } => s.kind = FlowStateKind::Aborted,
            MockHit { .. } => {}
            FlowCompleted { .. } => s.kind = FlowStateKind::Completed,
            FlowFailed { .. } => s.kind = FlowStateKind::Failed,
            other => return Err(Violation(format!("{} is not a flow event", other.kind()))),
        }
        Ok(s)
    }
}

impl Default for FlowState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(f: u64) -> Event {
        Event::RequestHead {
            flow: FlowId(f), conn: None, client: "c".into(), method: "GET".into(), uri: "https://h/p".into(),
            host: None, http_version: "HTTP/1.1".into(), headers: vec![],
        }
    }

    #[test]
    fn envelope_roundtrip_is_flat() {
        let e = Envelope { run: "r".into(), seq: 7, ts_us: 9, event: Event::MockHit { flow: FlowId(3), rule: "x".into() } };
        let j = serde_json::to_string(&e).unwrap();
        assert_eq!(j, r#"{"run":"r","seq":7,"ts_us":9,"kind":"mock_hit","flow":3,"rule":"x"}"#);
        assert_eq!(serde_json::from_str::<Envelope>(&j).unwrap(), e);
    }

    #[test]
    fn kind_matches_serialized_tag() {
        let ev = Event::EventsDropped { count: 2 };
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["kind"], ev.kind());
    }

    #[test]
    fn happy_path_and_violations() {
        let f = FlowId(1);
        let s = FlowState::apply(None, &head(1)).unwrap();
        let s = FlowState::apply(Some(s), &Event::ResponseHead { flow: f, status: 200, ttfb_us: 1, headers: vec![], mocked: false }).unwrap();
        let s = FlowState::apply(Some(s), &Event::FlowCompleted { flow: f, status: Some(200), mocked: false }).unwrap();
        assert_eq!(s.kind, FlowStateKind::Completed);
        assert!(FlowState::apply(Some(s), &Event::ResponseHead { flow: f, status: 200, ttfb_us: 1, headers: vec![], mocked: false }).is_err());
        assert!(FlowState::apply(None, &Event::FlowCompleted { flow: f, status: None, mocked: false }).is_err());
        let open = FlowState::apply(None, &head(2)).unwrap();
        assert!(FlowState::apply(Some(open), &Event::BodyEnd { flow: f, dir: BodyDir::Res, total: 0, chunks: 0, sha256: None, preview_b64: String::new(), stored_bytes: 0, truncated: false }).is_err());
    }
}
