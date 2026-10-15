//! M0 throwaway PoC: explicit MITM proxy on hudsucker.
//! Emits JSON-lines flow events on stdout and applies one hard-coded mock.
//! Usage: np-spike-rust [--port 8899] [--ca-out ca.pem]
use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{body::{Body as HttpBody, Bytes, Frame, SizeHint}, header, Method, Request, Response, StatusCode},
    rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose},
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::Message,
    *,
};
use http_body_util::BodyExt;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use bytes::BytesMut;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt, sync::mpsc};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    pin::Pin,
    sync::OnceLock,
    task::{Context, Poll},
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

static FLOW_SEQ: AtomicU64 = AtomicU64::new(1);
static DROPPED: AtomicU64 = AtomicU64::new(0);
static EVENTS: OnceLock<mpsc::Sender<String>> = OnceLock::new();
/// tracing span id of hudsucker's per-connection `process_connect` span -> our connection id.
static SPAN_CONN: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();
const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_millis(250);

/// Shared by every request on one client connection (handler clones share this Arc).
struct Conn {
    id: u64,
    requests: AtomicU64,
}
const PREVIEW_BYTES: usize = 4096;

fn now_us() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()
}

/// Never blocks the data path: a full queue drops the event and counts it.
fn emit(v: serde_json::Value) {
    if let Some(tx) = EVENTS.get() {
        if tx.try_send(v.to_string()).is_err() {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Wraps a body, reporting chunk sizes, total size, sha256 and a bounded preview.
struct Tap {
    inner: Body,
    flow: u64,
    dir: &'static str, // "req" | "res"
    total: u64,
    chunks: u64,
    hasher: Option<Sha256>,
    preview: Vec<u8>,
    finished: bool,
    buf: BytesMut,
    trailers: Option<header::HeaderMap>,
    inner_done: bool,
    last_progress: Option<Instant>,
}

impl Tap {
    fn finish(&mut self) {
        if self.finished { return; }
        self.finished = true;
        let sha = self.hasher.take().map(|h| h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>());
        emit(json!({"ev": "body_end", "flow": self.flow, "dir": self.dir, "t_us": now_us(),
                    "total": self.total, "chunks": self.chunks, "sha256": sha, "preview_len": self.preview.len(),
                    "preview_b64": B64.encode(&self.preview)}));
        if self.dir == "res" {
            emit(json!({"ev": "flow_completed", "flow": self.flow, "t_us": now_us()}));
        }
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        if !self.finished {
            emit(json!({"ev": "body_aborted", "flow": self.flow, "dir": self.dir,
                        "t_us": now_us(), "total": self.total}));
        }
    }
}

impl Tap {
    fn wrap(inner: Body, flow: u64, dir: &'static str) -> Body {
        if inner.is_end_stream() { return inner; } // empty body: hyper never polls it, nothing to tap
        Body::from(
            Tap { inner, flow, dir, total: 0, chunks: 0, hasher: Some(Sha256::new()), preview: Vec::new(), finished: false,
                  buf: BytesMut::new(), trailers: None, inner_done: false, last_progress: None }.boxed(),
        )
    }
}

const FRAME: usize = 16 * 1024;

impl HttpBody for Tap {
    type Data = Bytes;
    type Error = hudsucker::Error;

    /// Coalesces upstream chunks into full 16 KiB frames *without ever waiting*: it buffers only
    /// while upstream data is immediately ready and flushes as soon as upstream would block.
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let this = &mut *self;
        loop {
            if this.buf.len() >= FRAME {
                let out = this.buf.split_to(FRAME).freeze();
                if this.is_end_stream() { this.finish(); } // hyper won't poll again after the last frame
                return Poll::Ready(Some(Ok(Frame::data(out))));
            }
            if this.inner_done {
                if !this.buf.is_empty() {
                    let out = this.buf.split().freeze();
                    if this.is_end_stream() { this.finish(); }
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
                        if let Some(h) = this.hasher.as_mut() { h.update(&d); }
                        let room = PREVIEW_BYTES.saturating_sub(this.preview.len());
                        this.preview.extend_from_slice(&d[..d.len().min(room)]);
                        // Aggregated: first byte, then at most one progress event per 250 ms per body.
                        let due = this.last_progress.map_or(true, |t| t.elapsed() >= PROGRESS_EVERY);
                        if due {
                            let ev = if this.last_progress.is_none() { "body_first_byte" } else { "body_progress" };
                            this.last_progress = Some(Instant::now());
                            emit(json!({"ev": ev, "flow": this.flow, "dir": this.dir,
                                        "t_us": now_us(), "total": this.total}));
                        }
                        this.buf.extend_from_slice(&d);
                        if this.inner.is_end_stream() { this.inner_done = true; }
                    }
                    Err(f) => {
                        this.trailers = f.into_trailers().ok();
                        this.inner_done = true;
                    }
                },
                Poll::Ready(Some(Err(e))) => {
                    this.finished = true; // error is terminal; body_error below accounts for it
                    emit(json!({"ev": "body_error", "flow": this.flow, "dir": this.dir,
                                "t_us": now_us(), "total": this.total, "error": e.to_string()}));
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => this.inner_done = true,
                Poll::Pending => {
                    if this.buf.is_empty() { return Poll::Pending; }
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

/// Coarse failure classes the UI can explain. Based on the rustls/hyper error chain text for now.
fn classify(dbg: &str) -> &'static str {
    if dbg.contains("UnknownIssuer") { "tls_untrusted_issuer" }
    else if dbg.contains("NotValidForName") { "tls_hostname_mismatch" }
    else if dbg.contains("Expired") { "tls_expired" }
    else if dbg.contains("NotValidYet") { "tls_not_yet_valid" }
    else if dbg.contains("BadCertificate") || dbg.contains("InvalidCertificate") { "tls_certificate_other" }
    else if dbg.contains("HandshakeFailure") || dbg.contains("AlertReceived") || dbg.contains("Tls") { "tls_handshake_failed" }
    else if dbg.contains("ConnectionRefused") { "connect_refused" }
    else if dbg.contains("timed out") || dbg.contains("TimedOut") { "connect_timeout" }
    else if dbg.contains("dns") || dbg.contains("lookup") { "dns_failure" }
    else { "other" }
}

#[derive(Clone, Default)]
struct Events {
    // hudsucker clones the handler per request, so this state is exact per flow (h1 and h2).
    flow: u64,
    start: Option<Instant>,
    conn: Option<Arc<Conn>>,
}

impl HttpHandler for Events {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        let id = FLOW_SEQ.fetch_add(1, Ordering::Relaxed);
        self.flow = id;
        self.start = Some(Instant::now());
        let hdrs = |h: &hudsucker::hyper::HeaderMap| {
            h.iter().map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("<bin>").to_owned())).collect::<Vec<_>>()
        };

        if req.method() == Method::CONNECT {
            self.conn = Some(Arc::new(Conn { id, requests: AtomicU64::new(0) }));
            emit(json!({"ev": "connection_opened", "flow": id, "t_us": now_us(),
                        "client": ctx.client_addr.to_string(), "target": req.uri().to_string()}));
            return req.into();
        }

        let host = req.uri().host().map(str::to_owned).or_else(|| {
            req.headers().get(header::HOST).and_then(|h| h.to_str().ok()).map(str::to_owned)
        });
        let conn_id = self.conn.as_ref().map(|c| { c.requests.fetch_add(1, Ordering::Relaxed); c.id });
        emit(json!({
            "ev": "request_headers", "flow": id, "conn": conn_id, "t_us": now_us(),
            "client": ctx.client_addr.to_string(), "version": format!("{:?}", req.version()),
            "method": req.method().as_str(), "uri": req.uri().to_string(), "host": host,
            "headers": hdrs(req.headers()),
        }));

        // The single hard-coded mock (M0 only): GET <any host>/__np_mock
        if req.uri().path() == "/__np_mock" {
            emit(json!({"ev": "mock_hit", "flow": id, "t_us": now_us(), "rule": "poc-1"}));
            let res = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-np-mock", "poc-1")
                .body(Body::from(r#"{"mocked":true}"#))
                .unwrap();
            emit(json!({"ev": "flow_completed", "flow": id, "t_us": now_us(), "status": 200, "mocked": true}));
            return res.into();
        }
        let (parts, body) = req.into_parts();
        Request::from_parts(parts, Tap::wrap(body, id, "req")).into()
    }

    async fn should_intercept_tls(&mut self, _ctx: &HttpContext, hello: hudsucker::rustls::server::ClientHello<'_>) -> bool {
        if let Some(conn) = self.conn.clone() {
            let span = tracing::Span::current().id().map(|i| i.into_u64());
            if let Some(sp) = span {
                SPAN_CONN.get_or_init(Default::default).lock().unwrap().insert(sp, conn.id);
            }
            let alpn: Vec<String> = hello.alpn().into_iter().flatten().map(|a| String::from_utf8_lossy(a).into_owned()).collect();
            emit(json!({"ev": "tls_hello", "conn": conn.id, "t_us": now_us(), "sni": hello.server_name(), "alpn": alpn}));
            // Safety net for clients that vanish without any alert (e.g. some pinning implementations).
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                if conn.requests.load(Ordering::Relaxed) == 0 {
                    emit(json!({"ev": "client_tls_no_request", "conn": conn.id, "t_us": now_us(),
                                "hint": "client sent hello but no HTTP request within 5s: certificate rejected (pinning/untrusted CA) or idle"}));
                }
            });
        }
        true
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if self.flow == 0 || res.headers().contains_key("x-np-mock") { return res; }
        let ttfb = self.start.map(|s| s.elapsed().as_micros() as u64);
        emit(json!({
            "ev": "response_headers", "flow": self.flow, "t_us": now_us(),
            "status": res.status().as_u16(), "ttfb_us": ttfb,
            "headers": res.headers().iter().map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("<bin>").to_owned())).collect::<Vec<_>>(),
        }));
        let (parts, body) = res.into_parts();
        Response::from_parts(parts, Tap::wrap(body, self.flow, "res"))
    }

    async fn handle_error(&mut self, _ctx: &HttpContext, err: hudsucker::hyper_util::client::legacy::Error) -> Response<Body> {
        let dbg = format!("{err:?}");
        emit(json!({"ev": "flow_failed", "flow": self.flow, "t_us": now_us(),
                    "class": classify(&dbg), "error": dbg}));
        Response::builder().status(StatusCode::BAD_GATEWAY).body(Body::empty()).unwrap()
    }
}

impl WebSocketHandler for Events {
    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        let (dir, _) = match ctx {
            WebSocketContext::ClientToServer { src, .. } => ("c2s", src.to_string()),
            WebSocketContext::ServerToClient { dst, .. } => ("s2c", dst.to_string()),
        };
        emit(json!({"ev": "ws_message", "t_us": now_us(), "dir": dir, "bytes": msg.len()}));
        Some(msg)
    }
}

/// Bridges hudsucker's warn/error `tracing` events into the event stream, tagged with our connection id
/// via the per-connection span. Turns client-side TLS rejections into an explicit `client_tls_failed`.
struct LogBridge;

#[derive(Default)]
struct Fields { message: String, error: String }
impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        match f.name() { "message" => self.message = format!("{v:?}"), "error" => self.error = format!("{v:?}"), _ => {} }
    }
    fn record_error(&mut self, f: &tracing::field::Field, v: &(dyn std::error::Error + 'static)) {
        if f.name() == "error" { self.error = format!("{v:?}"); }
    }
}

fn classify_client_tls(detail: &str) -> &'static str {
    if detail.contains("UnknownCA") || detail.contains("UnknownIssuer") { "client_rejected_unknown_ca" }
    else if detail.contains("BadCertificate") || detail.contains("CertificateUnknown") || detail.contains("BadCert") { "client_rejected_certificate" }
    else if detail.contains("UnexpectedEof") || detail.contains("eof") || detail.contains("EOF") { "client_closed_during_handshake" }
    else { "client_tls_other" }
}

impl<S> tracing_subscriber::Layer<S> for LogBridge
where S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> {
    fn on_event(&self, ev: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let lvl = *ev.metadata().level();
        if lvl > tracing::Level::WARN { return; }
        let mut f = Fields::default();
        ev.record(&mut f);
        let conn = ctx.event_scope(ev).and_then(|scope| {
            let map = SPAN_CONN.get()?.lock().ok()?;
            scope.from_root().find_map(|sp| map.get(&sp.id().into_u64()).copied())
        });
        emit(json!({"ev": "proxy_log", "level": lvl.as_str(), "t_us": now_us(), "conn": conn,
                    "msg": f.message, "error": f.error}));
        if f.message.contains("Failed to establish TLS connection") {
            emit(json!({"ev": "client_tls_failed", "conn": conn, "t_us": now_us(),
                        "class": classify_client_tls(&f.error), "detail": f.error}));
        }
    }
}

fn make_ca(out: &str) -> RcgenAuthority {
    let key = KeyPair::generate().expect("keygen");
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    params.distinguished_name.push(DnType::CommonName, "NetProxy M0 Spike CA");
    let cert = params.self_signed(&key).expect("self-sign");
    std::fs::write(out, cert.pem()).expect("write CA");
    RcgenAuthority::new(Issuer::new(params, key), 1_000, aws_lc_rs::default_provider())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| {
        args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or(d.into())
    };
    let port: u16 = arg("--port", "8899").parse().expect("port");
    {
        use tracing_subscriber::layer::SubscriberExt;
        tracing::subscriber::set_global_default(tracing_subscriber::registry().with(LogBridge)).ok();
    }
    let (tx, mut rx) = mpsc::channel::<String>(65_536);
    EVENTS.set(tx).ok();
    let writer = tokio::spawn(async move {
        let mut out = tokio::io::BufWriter::new(tokio::io::stdout());
        while let Some(line) = rx.recv().await {
            let _ = out.write_all(line.as_bytes()).await;
            let _ = out.write_all(b"\n").await;
            if rx.is_empty() { let _ = out.flush().await; }
        }
        let _ = out.flush().await;
    });
    let ca = make_ca(&arg("--ca-out", "ca.pem"));

    // Optional extra upstream trust root (harness CA) on top of the public webpki roots.
    let mut roots = hudsucker::rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let up_ca = arg("--upstream-ca", "");
    if !up_ca.is_empty() {
        let pem = std::fs::read(&up_ca).expect("read upstream CA");
        for c in <hudsucker::rustls::pki_types::CertificateDer as hudsucker::rustls::pki_types::pem::PemObject>::pem_slice_iter(&pem) {
            roots.add(c.expect("parse CA")).expect("add CA");
        }
    }
    let tls = hudsucker::rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions().expect("tls versions")
        .with_root_certificates(roots).with_no_client_auth();
    let mut http = hudsucker::hyper_util::client::legacy::connect::HttpConnector::new();
    http.enforce_http(false);
    http.set_connect_timeout(Some(std::time::Duration::from_secs(10)));
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls.clone()).https_or_http().enable_http1().enable_http2().wrap_connector(http);

    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_ca(ca)
        .with_http_connector(connector)
        .with_websocket_connector(hudsucker::tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(tls)))
        .with_http_handler(Events::default())
        .with_websocket_handler(Events::default())
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; })
        .build()
        .expect("proxy");
    eprintln!("listening on 127.0.0.1:{port}");
    if let Err(e) = proxy.start().await {
        eprintln!("proxy error: {e}");
    }
    eprintln!("events_dropped={}", DROPPED.load(Ordering::Relaxed));
    drop(writer);
}
