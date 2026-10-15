//! M0 shared upstream: HTTPS (h1 + h2 via ALPN) with deterministic endpoints so any
//! engine can be checked against ground truth.
//!   /small         small JSON
//!   /big?mb=N      N MiB streamed, bytes = repeating pattern (sha256 in x-np-sha256)
//!   /slow?ms=N     body dripped over N ms
//!   /reset         connection dropped mid-body
//!   /redirect      302 -> /small
//!   /echo          echoes request body (POST)
//! Writes its CA to --ca-out so clients can trust it. Usage: np-harness [--port 9443] [--ca-out upstream-ca.pem]
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{body::{Frame, Incoming}, service::service_fn, Request, Response, StatusCode};
use hyper_util::{rt::{TokioExecutor, TokioIo}, server::conn::auto::Builder};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType};
use sha2::{Digest, Sha256};
use std::{convert::Infallible, io, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio_rustls::{rustls::{self, pki_types::{CertificateDer, PrivateKeyDer}}, TlsAcceptor};

type Body = BoxBody<Bytes, io::Error>;
const CHUNK: usize = 64 * 1024;

fn full(b: impl Into<Bytes>) -> Body {
    Full::new(b.into()).map_err(|n| match n {}).boxed()
}

fn pattern(n: usize, off: usize) -> Bytes {
    Bytes::from((0..n).map(|i| ((off + i) % 251) as u8).collect::<Vec<u8>>())
}

fn q(req: &Request<Incoming>, k: &str, d: u64) -> u64 {
    req.uri().query().unwrap_or("").split('&')
        .find_map(|p| p.strip_prefix(&format!("{k}=")).and_then(|v| v.parse().ok())).unwrap_or(d)
}

async fn route(req: Request<Incoming>) -> Result<Response<Body>, Infallible> {
    let res = match req.uri().path() {
        "/small" => Response::builder().header("content-type", "application/json")
            .body(full(r#"{"ok":true,"n":42}"#)).unwrap(),
        "/big" => {
            let total = q(&req, "mb", 100) as usize * 1024 * 1024;
            let mut h = Sha256::new();
            let mut off = 0;
            while off < total { let n = CHUNK.min(total - off); h.update(pattern(n, off)); off += n; }
            let digest: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
            let stream = futures_util::stream::unfold(0usize, move |off| async move {
                if off >= total { return None; }
                let n = CHUNK.min(total - off);
                Some((Ok::<_, io::Error>(Frame::data(pattern(n, off))), off + n))
            });
            Response::builder().header("x-np-sha256", digest).header("content-length", total)
                .body(StreamBody::new(stream).boxed()).unwrap()
        }
        "/slow" => {
            let ms = q(&req, "ms", 2000);
            let stream = futures_util::stream::unfold(0u32, move |i| async move {
                if i >= 10 { return None; }
                tokio::time::sleep(Duration::from_millis(ms / 10)).await;
                Some((Ok::<_, io::Error>(Frame::data(Bytes::from_static(b"0123456789"))), i + 1))
            });
            Response::new(StreamBody::new(stream).boxed())
        }
        "/reset" => {
            let stream = futures_util::stream::unfold(0u32, |i| async move {
                match i {
                    0 => Some((Ok(Frame::data(Bytes::from_static(b"partial"))), 1)),
                    1 => { tokio::time::sleep(Duration::from_millis(50)).await;
                           Some((Err(io::Error::new(io::ErrorKind::ConnectionReset, "reset")), 2)) }
                    _ => None,
                }
            });
            Response::builder().header("content-length", "1000").body(StreamBody::new(stream).boxed()).unwrap()
        }
        "/ws" => {
            let key = req.headers().get("sec-websocket-key").and_then(|v| v.to_str().ok()).unwrap_or("").to_owned();
            let accept = tokio_tungstenite::tungstenite::handshake::derive_accept_key(key.as_bytes());
            tokio::spawn(async move {
                use futures_util::{SinkExt, StreamExt};
                if let Ok(up) = hyper::upgrade::on(req).await {
                    let ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
                        TokioIo::new(up), tokio_tungstenite::tungstenite::protocol::Role::Server, None).await;
                    let (mut tx, mut rx) = ws.split();
                    while let Some(Ok(m)) = rx.next().await {
                        if m.is_text() || m.is_binary() { if tx.send(m).await.is_err() { break; } }
                    }
                }
            });
            Response::builder().status(101).header("upgrade", "websocket").header("connection", "Upgrade")
                .header("sec-websocket-accept", accept).body(full("")).unwrap()
        }
        "/redirect" => Response::builder().status(StatusCode::FOUND).header("location", "/small").body(full("")).unwrap(),
        "/echo" => {
            let body = req.into_body().collect().await.map(|b| b.to_bytes()).unwrap_or_default();
            Response::new(full(body))
        }
        _ => Response::builder().status(404).body(full("not found")).unwrap(),
    };
    Ok(res)
}

/// Handle to the running upstream: ports `base_port..base_port+4` are
/// good / untrusted issuer / wrong hostname / expired.
pub struct Upstream {
    pub ca_pem: String,
    pub base_port: u16,
}

/// Finds 4 consecutive free loopback ports. Blocks are handed out from a process-wide counter so
/// parallel tests in one process never pick the same block; each is also checked against the OS.
pub fn find_free_base() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static NEXT: AtomicU16 = AtomicU16::new(21000);
    loop {
        let base = NEXT.fetch_add(8, Ordering::SeqCst);
        assert!(base < 60000, "no free port block");
        if (0..4).all(|o| std::net::TcpListener::bind(("127.0.0.1", base + o)).is_ok()) {
            return base;
        }
    }
}

pub async fn start(base: u16) -> Upstream {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::default();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    ca_params.distinguished_name.push(DnType::CommonName, "NetProxy Harness CA");
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let ca_pem = ca_cert.pem();
    let issuer = Issuer::new(ca_params, ca_key);

    let cfg_for = |cert: &rcgen::Certificate, key: &KeyPair| {
        let mut cfg = rustls::ServerConfig::builder().with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(cert.der().to_vec())],
                PrivateKeyDer::try_from(key.serialize_der()).unwrap()).unwrap();
        cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        TlsAcceptor::from(Arc::new(cfg))
    };
    let leaf = |names: Vec<String>, issuer: &Issuer<'_, KeyPair>, expired: bool| {
        let k = KeyPair::generate().unwrap();
        let mut p = CertificateParams::new(names).unwrap();
        p.subject_alt_names.push(SanType::IpAddress("127.0.0.1".parse().unwrap()));
        if expired {
            p.not_before = rcgen::date_time_ymd(2020, 1, 1);
            p.not_after = rcgen::date_time_ymd(2021, 1, 1);
        }
        let c = p.signed_by(&k, issuer).unwrap();
        (c, k)
    };
    // 9443 good | 9444 untrusted issuer | 9445 wrong hostname | 9446 expired
    let other_key = KeyPair::generate().unwrap();
    let mut other_params = CertificateParams::default();
    other_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    other_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    other_params.distinguished_name.push(DnType::CommonName, "Untrusted CA");
    let other_issuer = Issuer::new(other_params, other_key);

        let (c, k) = leaf(vec!["localhost".into()], &issuer, false);
    let good = cfg_for(&c, &k);
    let (c, k) = leaf(vec!["localhost".into()], &other_issuer, false);
    let untrusted = cfg_for(&c, &k);
    let (c, k) = leaf(vec!["wrong.example".into()], &issuer, false);
    // wrong-host leaf must not carry the 127.0.0.1 SAN
    let _ = (c, k);
    let wrong = {
        let k = KeyPair::generate().unwrap();
        let p = CertificateParams::new(vec!["wrong.example".to_string()]).unwrap();
        let c = p.signed_by(&k, &issuer).unwrap();
        cfg_for(&c, &k)
    };
    let (c, k) = leaf(vec!["localhost".into()], &issuer, true);
    let expired = cfg_for(&c, &k);

    for (off, acc) in [(0u16, good), (1, untrusted), (2, wrong), (3, expired)] {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", base + off)).await.unwrap();
        eprintln!("harness upstream on https://localhost:{}", base + off);
        tokio::spawn(async move {
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                let acceptor = acc.clone();
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else { return };
                    let _ = Builder::new(TokioExecutor::new())
                        .serve_connection_with_upgrades(TokioIo::new(tls), service_fn(route)).await;
                });
            }
        });
    }
    Upstream { ca_pem, base_port: base }
}
