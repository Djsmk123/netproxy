//! `ProxyEngine` implementation on hudsucker 0.25 (hyper + rustls).
//!
//! hudsucker clones the HTTP handler **per request** and reuses that clone for request, response
//! and error, so a flow id stored on the handler is exact for HTTP/1.1 and HTTP/2. Handler clones
//! made for requests on the same client connection share one `Arc<Conn>`.

use crate::{
    body::{BodySink, BodyWriter},
    bus::EventSink,
    engine::*,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use bytes::{Bytes, BytesMut};
use futures::future::BoxFuture;
use http_body_util::BodyExt;
use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{
        body::{Body as HttpBody, Frame, SizeHint},
        header, Method, Request, Response, StatusCode,
    },
    rcgen::{Issuer, KeyPair},
    rustls::{self, crypto::aws_lc_rs},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use np_model::*;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::SocketAddr,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

const FRAME: usize = 16 * 1024;
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

#[derive(Default)]
struct Ids {
    conn: AtomicU64,
    flow: AtomicU64,
}

struct Shared {
    sink: EventSink,
    ids: Ids,
    mock: Arc<dyn MockResolver>,
    bodies: Arc<dyn BodySink>,
    preview: usize,
}

/// One client connection (CONNECT tunnel). Dropped when the last request handler is gone.
struct Conn {
    id: ConnId,
    requests: AtomicU64,
    span: Mutex<Option<u64>>,
    sink: EventSink,
}

impl Drop for Conn {
    fn drop(&mut self) {
        if let Some(sp) = self.span.lock().ok().and_then(|g| *g) {
            if let Ok(mut b) = BRIDGE.lock() {
                b.spans().remove(&sp);
            }
        }
        self.sink.emit(Event::ConnectionClosed {
            conn: self.id,
            requests: self.requests.load(Ordering::Relaxed),
        });
    }
}

// ---------------------------------------------------------------- body tap

/// Wraps a body: reports first byte / throttled progress / end, hashes, keeps a bounded preview,
/// and coalesces chunks into full 16 KiB frames **without ever waiting** (buffers only while the
/// upstream is immediately ready). Strict HTTP/2 peers drop connections that receive too many
/// sub-threshold DATA frames, which uncoalesced proxying provokes.
struct Tap {
    inner: Body,
    sink: EventSink,
    flow: FlowId,
    dir: BodyDir,
    /// For response bodies: reported with `FlowCompleted`.
    status: Option<u16>,
    preview_cap: usize,
    total: u64,
    chunks: u64,
    hasher: Option<Sha256>,
    preview: Vec<u8>,
    finished: bool,
    buf: BytesMut,
    trailers: Option<header::HeaderMap>,
    inner_done: bool,
    last_progress: Option<Instant>,
    writer: Option<Box<dyn BodyWriter>>,
}

impl Tap {
    fn wrap(inner: Body, sh: &Shared, flow: FlowId, dir: BodyDir, status: Option<u16>) -> Body {
        let (sink, preview_cap) = (&sh.sink, sh.preview);
        let writer = sh.bodies.open(flow, dir);
        Body::from(
            Tap {
                inner,
                sink: sink.clone(),
                flow,
                dir,
                status,
                preview_cap,
                total: 0,
                chunks: 0,
                hasher: Some(Sha256::new()),
                preview: Vec::new(),
                finished: false,
                buf: BytesMut::new(),
                trailers: None,
                inner_done: false,
                last_progress: None,
                writer,
            }
            .boxed(),
        )
    }

    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let sha = self.hasher.take().map(|h| hex(&h.finalize()));
        let cap = self.writer.take().map_or(
            crate::body::BodyCapture {
                stored_bytes: 0,
                truncated: self.total > 0,
            },
            |w| w.finish(self.total),
        );
        self.sink.emit(Event::BodyEnd {
            flow: self.flow,
            dir: self.dir,
            total: self.total,
            chunks: self.chunks,
            sha256: sha,
            preview_b64: B64.encode(&self.preview),
            stored_bytes: cap.stored_bytes,
            truncated: cap.truncated,
        });
        if self.dir == BodyDir::Res {
            self.sink.emit(Event::FlowCompleted {
                flow: self.flow,
                status: self.status,
                mocked: false,
            });
        }
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        if !self.finished {
            if let Some(w) = self.writer.take() {
                w.finish(self.total); // release the partial file handle
            }
            self.sink.emit(Event::BodyAborted {
                flow: self.flow,
                dir: self.dir,
                total: self.total,
            });
        }
    }
}

impl HttpBody for Tap {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let this = &mut *self;
        loop {
            if this.buf.len() >= FRAME {
                let out = this.buf.split_to(FRAME).freeze();
                if this.is_end_stream() {
                    this.finish(); // hyper does not poll again after the last frame
                }
                return Poll::Ready(Some(Ok(Frame::data(out))));
            }
            if this.inner_done {
                if !this.buf.is_empty() {
                    let out = this.buf.split().freeze();
                    if this.is_end_stream() {
                        this.finish();
                    }
                    return Poll::Ready(Some(Ok(Frame::data(out))));
                }
                if let Some(t) = this.trailers.take() {
                    this.finish();
                    return Poll::Ready(Some(Ok(Frame::trailers(t))));
                }
                this.finish();
                return Poll::Ready(None);
            }
            match Pin::new(&mut this.inner).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    Ok(d) => {
                        this.total += d.len() as u64;
                        this.chunks += 1;
                        if let Some(h) = this.hasher.as_mut() {
                            h.update(&d);
                        }
                        if let Some(w) = this.writer.as_mut() {
                            w.write(&d);
                        }
                        let room = this.preview_cap.saturating_sub(this.preview.len());
                        this.preview.extend_from_slice(&d[..d.len().min(room)]);
                        match this.last_progress {
                            None => {
                                this.last_progress = Some(Instant::now());
                                this.sink.emit(Event::BodyFirstByte {
                                    flow: this.flow,
                                    dir: this.dir,
                                });
                            }
                            Some(t) if t.elapsed() >= PROGRESS_EVERY => {
                                this.last_progress = Some(Instant::now());
                                this.sink.emit(Event::BodyProgress {
                                    flow: this.flow,
                                    dir: this.dir,
                                    total: this.total,
                                });
                            }
                            _ => {}
                        }
                        this.buf.extend_from_slice(&d);
                        if this.inner.is_end_stream() {
                            this.inner_done = true;
                        }
                    }
                    Err(f) => {
                        this.trailers = f.into_trailers().ok();
                        this.inner_done = true;
                    }
                },
                Poll::Ready(Some(Err(e))) => {
                    this.finished = true; // terminal; BodyError accounts for it
                    this.sink.emit(Event::BodyError {
                        flow: this.flow,
                        dir: this.dir,
                        total: this.total,
                        error: e.to_string(),
                    });
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => this.inner_done = true,
                Poll::Pending => {
                    if this.buf.is_empty() {
                        return Poll::Pending;
                    }
                    return Poll::Ready(Some(Ok(Frame::data(this.buf.split().freeze()))));
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner_done && self.buf.is_empty() && self.trailers.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------- handler

#[derive(Clone)]
struct Handler {
    shared: Arc<Shared>,
    conn: Option<Arc<Conn>>,
    flow: Option<FlowId>,
    started: Option<Instant>,
}

fn header_list(h: &header::HeaderMap) -> Headers {
    h.iter()
        .map(|(k, v)| {
            (
                k.as_str().to_owned(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect()
}

fn is_ws_upgrade(h: &header::HeaderMap) -> bool {
    h.get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
}

impl HttpHandler for Handler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        let sh = self.shared.clone();

        if req.method() == Method::CONNECT {
            let id = ConnId(sh.ids.conn.fetch_add(1, Ordering::Relaxed) + 1);
            self.conn = Some(Arc::new(Conn {
                id,
                requests: AtomicU64::new(0),
                span: Mutex::new(None),
                sink: sh.sink.clone(),
            }));
            sh.sink.emit(Event::ConnectionOpened {
                conn: id,
                client: ctx.client_addr.to_string(),
                target: req.uri().to_string(),
            });
            return req.into();
        }

        let flow = FlowId(sh.ids.flow.fetch_add(1, Ordering::Relaxed) + 1);
        self.flow = Some(flow);
        self.started = Some(Instant::now());
        let conn = self.conn.as_ref().map(|c| {
            c.requests.fetch_add(1, Ordering::Relaxed);
            c.id
        });
        let host = req.uri().host().map(str::to_owned).or_else(|| {
            req.headers()
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .map(str::to_owned)
        });
        let headers = header_list(req.headers());
        sh.sink.emit(Event::RequestHead {
            flow,
            conn,
            client: ctx.client_addr.to_string(),
            method: req.method().as_str().to_owned(),
            uri: req.uri().to_string(),
            host: host.clone(),
            http_version: format!("{:?}", req.version()),
            headers: headers.clone(),
        });

        // A WebSocket handshake is its own (short) flow; frames are reported as `WsMessage`.
        if is_ws_upgrade(req.headers()) {
            sh.sink.emit(Event::FlowCompleted {
                flow,
                status: Some(101),
                mocked: false,
            });
            self.flow = None;
            return req.into();
        }

        let view = RequestView {
            method: req.method().as_str(),
            uri: &req.uri().to_string(),
            host: host.as_deref(),
            path: req.uri().path(),
            headers: &headers,
        };
        if let MockDecision::Respond(m) = sh.mock.resolve(&view) {
            sh.sink.emit(Event::MockHit {
                flow,
                rule: m.rule.clone(),
            });
            sh.sink.emit(Event::ResponseHead {
                flow,
                status: m.status,
                ttfb_us: 0,
                headers: m.headers.clone(),
                mocked: true,
            });
            sh.sink.emit(Event::BodyEnd {
                flow,
                dir: BodyDir::Res,
                total: m.body.len() as u64,
                chunks: 1,
                sha256: Some(hex(&Sha256::digest(&m.body))),
                preview_b64: B64.encode(&m.body[..m.body.len().min(sh.preview)]),
                stored_bytes: 0,
                truncated: false,
            });
            sh.sink.emit(Event::FlowCompleted {
                flow,
                status: Some(m.status),
                mocked: true,
            });
            let mut b = Response::builder()
                .status(StatusCode::from_u16(m.status).unwrap_or(StatusCode::OK));
            for (k, v) in &m.headers {
                b = b.header(k.as_str(), v.as_str());
            }
            self.flow = None;
            return b
                .header("x-np-mock", m.rule.as_str())
                .body(Body::from(m.body))
                .unwrap()
                .into();
        }

        let (parts, body) = req.into_parts();
        let body = if body.is_end_stream() {
            body
        } else {
            Tap::wrap(body, &sh, flow, BodyDir::Req, None)
        };
        Request::from_parts(parts, body).into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let (Some(flow), sh) = (self.flow, self.shared.clone()) else {
            return res;
        };
        let status = res.status().as_u16();
        sh.sink.emit(Event::ResponseHead {
            flow,
            status,
            ttfb_us: self.started.map_or(0, |s| s.elapsed().as_micros() as u64),
            headers: header_list(res.headers()),
            mocked: false,
        });
        let (parts, body) = res.into_parts();
        if body.is_end_stream() {
            // hyper never polls an empty body, so report its end (and the flow's) here.
            sh.sink.emit(Event::BodyEnd {
                flow,
                dir: BodyDir::Res,
                total: 0,
                chunks: 0,
                sha256: Some(hex(&Sha256::digest(b""))),
                preview_b64: String::new(),
                stored_bytes: 0,
                truncated: false,
            });
            sh.sink.emit(Event::FlowCompleted {
                flow,
                status: Some(status),
                mocked: false,
            });
            return Response::from_parts(parts, body);
        }
        Response::from_parts(
            parts,
            Tap::wrap(body, &sh, flow, BodyDir::Res, Some(status)),
        )
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hudsucker::hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        if let Some(flow) = self.flow {
            let detail = format!("{err:?}");
            self.shared.sink.emit(Event::FlowFailed {
                flow,
                class: classify_flow_failure(&detail),
                detail,
            });
        }
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap()
    }

    async fn should_intercept_tls(
        &mut self,
        _ctx: &HttpContext,
        hello: rustls::server::ClientHello<'_>,
    ) -> bool {
        if let Some(conn) = self.conn.clone() {
            if let Some(sp) = tracing::Span::current().id().map(|i| i.into_u64()) {
                *conn.span.lock().unwrap() = Some(sp);
                BRIDGE.lock().unwrap().spans().insert(sp, conn.id);
            }
            let alpn = hello
                .alpn()
                .into_iter()
                .flatten()
                .map(|a| String::from_utf8_lossy(a).into_owned())
                .collect();
            self.shared.sink.emit(Event::TlsHello {
                conn: conn.id,
                sni: hello.server_name().map(str::to_owned),
                alpn,
            });
        }
        true
    }
}

impl WebSocketHandler for Handler {
    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        let (client, dir) = match ctx {
            WebSocketContext::ClientToServer { src, .. } => {
                (src.to_string(), WsDir::ClientToServer)
            }
            WebSocketContext::ServerToClient { dst, .. } => {
                (dst.to_string(), WsDir::ServerToClient)
            }
        };
        self.shared.sink.emit(Event::WsMessage {
            client,
            dir,
            bytes: msg.len() as u64,
        });
        Some(msg)
    }
}

pub fn classify_flow_failure(dbg: &str) -> FlowFailClass {
    use FlowFailClass::*;
    if dbg.contains("UnknownIssuer") {
        TlsUntrustedIssuer
    } else if dbg.contains("NotValidForName") {
        TlsHostnameMismatch
    } else if dbg.contains("Expired") {
        TlsExpired
    } else if dbg.contains("NotValidYet") {
        TlsNotYetValid
    } else if dbg.contains("BadCertificate") || dbg.contains("InvalidCertificate") {
        TlsCertificateOther
    } else if dbg.contains("HandshakeFailure")
        || dbg.contains("AlertReceived")
        || dbg.contains("Tls")
    {
        TlsHandshakeFailed
    } else if dbg.contains("ConnectionRefused") {
        ConnectRefused
    } else if dbg.contains("timed out") || dbg.contains("TimedOut") {
        ConnectTimeout
    } else if dbg.contains("dns") || dbg.contains("lookup") {
        DnsFailure
    } else {
        Other
    }
}

pub fn classify_client_tls(detail: &str) -> ClientTlsClass {
    use ClientTlsClass::*;
    if detail.contains("UnknownCA") || detail.contains("UnknownIssuer") {
        RejectedUnknownCa
    } else if detail.contains("BadCertificate") || detail.contains("CertificateUnknown") {
        RejectedCertificate
    } else if detail.contains("UnexpectedEof") || detail.contains("eof") || detail.contains("EOF") {
        ClosedDuringHandshake
    } else {
        Other
    }
}

// ---------------------------------------------------------------- tracing bridge

struct Bridge {
    sink: Option<EventSink>,
    spans: Option<HashMap<u64, ConnId>>,
}

impl Bridge {
    fn spans(&mut self) -> &mut HashMap<u64, ConnId> {
        self.spans.get_or_insert_with(HashMap::new)
    }
}

static BRIDGE: Mutex<Bridge> = Mutex::new(Bridge {
    sink: None,
    spans: None,
});

#[derive(Default)]
struct Fields {
    message: String,
    error: String,
}

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        match f.name() {
            "message" => self.message = format!("{v:?}"),
            "error" => self.error = format!("{v:?}"),
            _ => {}
        }
    }
    fn record_error(&mut self, f: &tracing::field::Field, v: &(dyn std::error::Error + 'static)) {
        if f.name() == "error" {
            self.error = format!("{v:?}");
        }
    }
}

/// hudsucker reports client-side TLS handshake failures only through `tracing`. This layer turns
/// them into `ClientTlsFailed` events, tagged with the connection via hudsucker's per-connection
/// span. Compose it into the process-wide subscriber (`registry().with(tracing_layer())`).
pub struct TracingBridge;

pub fn tracing_layer() -> TracingBridge {
    TracingBridge
}

impl<S> tracing_subscriber::Layer<S> for TracingBridge
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, ev: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if *ev.metadata().level() > tracing::Level::WARN {
            return;
        }
        let mut f = Fields::default();
        ev.record(&mut f);
        if !f.message.contains("Failed to establish TLS connection") {
            return;
        }
        let Ok(mut b) = BRIDGE.lock() else { return };
        let Some(sink) = b.sink.clone() else { return };
        let spans = b.spans();
        let conn = ctx.event_scope(ev).and_then(|scope| {
            scope
                .from_root()
                .find_map(|sp| spans.get(&sp.id().into_u64()).copied())
        });
        sink.emit(Event::ClientTlsFailed {
            conn,
            class: classify_client_tls(&f.error),
            detail: f.error,
        });
    }
}

// ---------------------------------------------------------------- engine

pub struct HudsuckerEngine;

struct Handle {
    addr: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
    sink: EventSink,
}

impl EngineHandle for Handle {
    fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    fn shutdown(mut self: Box<Self>) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            if let Some(tx) = self.stop.take() {
                let _ = tx.send(());
            }
            // Graceful drain can wait on keep-alive/WebSocket connections: bound it.
            if tokio::time::timeout(Duration::from_secs(3), &mut self.task)
                .await
                .is_err()
            {
                self.task.abort();
            }
            if let Ok(mut b) = BRIDGE.lock() {
                b.sink = None;
                b.spans().clear();
            }
            self.sink.emit(Event::EngineStopped {
                reason: "shutdown".into(),
            });
        })
    }
}

impl ProxyEngine for HudsuckerEngine {
    fn name(&self) -> &'static str {
        "hudsucker"
    }

    fn start(
        &self,
        cfg: EngineConfig,
        sink: EventSink,
    ) -> BoxFuture<'static, Result<Box<dyn EngineHandle>, EngineError>> {
        Box::pin(async move {
            let provider = aws_lc_rs::default_provider();
            let key =
                KeyPair::from_pem(&cfg.ca.key_pem).map_err(|e| EngineError::Ca(e.to_string()))?;
            let issuer = Issuer::from_ca_cert_pem(&cfg.ca.cert_pem, key)
                .map_err(|e| EngineError::Ca(e.to_string()))?;
            let authority = RcgenAuthority::new(issuer, 1_000, provider.clone());

            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            if let Some(pem) = &cfg.upstream_roots_pem {
                for c in <rustls::pki_types::CertificateDer as rustls::pki_types::pem::PemObject>::pem_slice_iter(pem) {
                    roots.add(c.map_err(|e| EngineError::Tls(e.to_string()))?).map_err(|e| EngineError::Tls(e.to_string()))?;
                }
            }
            let tls = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
                .with_safe_default_protocol_versions()
                .map_err(|e| EngineError::Tls(e.to_string()))?
                .with_root_certificates(roots)
                .with_no_client_auth();
            let mut http = hudsucker::hyper_util::client::legacy::connect::HttpConnector::new();
            http.enforce_http(false);
            http.set_connect_timeout(Some(cfg.connect_timeout));
            let connector = hyper_rustls::HttpsConnectorBuilder::new()
                .with_tls_config(tls.clone())
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .wrap_connector(http);

            let listener = TcpListener::bind(cfg.listen)
                .await
                .map_err(|e| EngineError::Bind(cfg.listen, e))?;
            let addr = listener
                .local_addr()
                .map_err(|e| EngineError::Bind(cfg.listen, e))?;

            BRIDGE.lock().unwrap().sink = Some(sink.clone());
            let shared = Arc::new(Shared {
                sink: sink.clone(),
                ids: Ids::default(),
                mock: cfg.mock.clone(),
                bodies: cfg.bodies.clone(),
                preview: cfg.preview_bytes,
            });
            let handler = Handler {
                shared,
                conn: None,
                flow: None,
                started: None,
            };

            let (stop_tx, stop_rx) = oneshot::channel::<()>();
            let proxy = Proxy::builder()
                .with_listener(listener)
                .with_ca(authority)
                .with_http_connector(connector)
                .with_websocket_connector(hudsucker::tokio_tungstenite::Connector::Rustls(
                    Arc::new(tls),
                ))
                .with_http_handler(handler.clone())
                .with_websocket_handler(handler)
                .with_graceful_shutdown(async move {
                    let _ = stop_rx.await;
                })
                .build()
                .map_err(|e| EngineError::Other(e.to_string()))?;

            sink.emit(Event::EngineStarted {
                listen: addr.to_string(),
                engine: "hudsucker".into(),
                engine_version: "0.25".into(),
            });
            let task = tokio::spawn(async move {
                if let Err(e) = proxy.start().await {
                    eprintln!("proxy terminated: {e}");
                }
            });
            Ok(Box::new(Handle {
                addr,
                stop: Some(stop_tx),
                task,
                sink,
            }) as Box<dyn EngineHandle>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_classes() {
        assert_eq!(
            classify_flow_failure("InvalidCertificate(UnknownIssuer)"),
            FlowFailClass::TlsUntrustedIssuer
        );
        assert_eq!(
            classify_flow_failure("InvalidCertificate(NotValidForName)"),
            FlowFailClass::TlsHostnameMismatch
        );
        assert_eq!(
            classify_flow_failure("InvalidCertificate(Expired)"),
            FlowFailClass::TlsExpired
        );
        assert_eq!(
            classify_flow_failure("ConnectionRefused"),
            FlowFailClass::ConnectRefused
        );
    }

    #[test]
    fn client_tls_classes() {
        assert_eq!(
            classify_client_tls("AlertReceived(UnknownCA)"),
            ClientTlsClass::RejectedUnknownCa
        );
        assert_eq!(
            classify_client_tls("AlertReceived(CertificateUnknown)"),
            ClientTlsClass::RejectedCertificate
        );
        assert_eq!(
            classify_client_tls("tls handshake eof"),
            ClientTlsClass::ClosedDuringHandshake
        );
    }
}
