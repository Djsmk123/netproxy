//! Built-in device-setup pages served by the proxy itself at `http://np.pro/`.
//!
//! A phone that is already pointed at the proxy opens `http://np.pro/` and can download the CA as
//! PEM, DER (`.crt`, for Android) or an iOS configuration profile. Only plain HTTP works: the
//! device does not trust the CA yet, so HTTPS to this name would fail by construction.

use bytes::Bytes;
use np_engine::{CaMaterial, EngineError, MockDecision, MockResolver, MockResponse, RequestView};
use std::sync::Arc;

pub const SETUP_HOST: &str = "np.pro";
pub const SETUP_RULE: &str = "np-setup";

pub struct DeviceSetup {
    pem: Bytes,
    der: Bytes,
    profile: Bytes,
    inner: Arc<dyn MockResolver>,
}

impl DeviceSetup {
    pub fn new(ca: &CaMaterial, inner: Arc<dyn MockResolver>) -> Result<Self, EngineError> {
        Ok(Self {
            pem: Bytes::from(ca.cert_pem.clone()),
            der: Bytes::from(ca.cert_der()?),
            profile: Bytes::from(ca.mobileconfig("NetProxy Root CA")?),
            inner,
        })
    }

    fn page() -> Bytes {
        Bytes::from_static(
            b"<!doctype html><meta name=viewport content='width=device-width,initial-scale=1'><title>NetProxy setup</title>\
<body style='font:16px -apple-system,sans-serif;max-width:32em;margin:2em auto;padding:0 1em'>\
<h1>NetProxy certificate</h1>\
<p>Install the root certificate so NetProxy can show HTTPS traffic from this device.</p>\
<ul><li><a href='/ca.mobileconfig'>iOS / iPadOS profile</a> (then enable it under Settings &rsaquo; General &rsaquo; About &rsaquo; Certificate Trust Settings)</li>\
<li><a href='/ca.crt'>Android certificate (.crt)</a> &mdash; apps ignore user certificates unless they opt in</li>\
<li><a href='/ca.pem'>PEM</a></li></ul></body>",
        )
    }
}

fn resp(content_type: &str, body: Bytes, filename: Option<&str>) -> MockDecision {
    let mut headers = vec![
        ("content-type".to_owned(), content_type.to_owned()),
        ("cache-control".to_owned(), "no-store".to_owned()),
    ];
    if let Some(f) = filename {
        headers.push((
            "content-disposition".to_owned(),
            format!("attachment; filename=\"{f}\""),
        ));
    }
    MockDecision::Respond(MockResponse {
        rule: SETUP_RULE.into(),
        status: 200,
        headers,
        body,
    })
}

impl MockResolver for DeviceSetup {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
        let host = req
            .host
            .map(|h| h.split(':').next().unwrap_or(h).to_ascii_lowercase());
        if host.as_deref() == Some(SETUP_HOST) && req.method == "GET" {
            return match req.path {
                "/" | "/index.html" => resp("text/html; charset=utf-8", Self::page(), None),
                "/ca.pem" => resp(
                    "application/x-pem-file",
                    self.pem.clone(),
                    Some("netproxy-ca.pem"),
                ),
                "/ca.crt" | "/ca.cer" => resp(
                    "application/x-x509-ca-cert",
                    self.der.clone(),
                    Some("netproxy-ca.crt"),
                ),
                "/ca.mobileconfig" => resp(
                    "application/x-apple-aspen-config",
                    self.profile.clone(),
                    Some("netproxy-ca.mobileconfig"),
                ),
                _ => MockDecision::Respond(MockResponse {
                    rule: SETUP_RULE.into(),
                    status: 404,
                    headers: vec![],
                    body: Bytes::from_static(b"not found"),
                }),
            };
        }
        self.inner.resolve(req)
    }
}
