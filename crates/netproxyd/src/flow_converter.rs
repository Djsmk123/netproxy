//! Convert captured HTTP flows into mock rules.

use crate::pattern_detector::{PathAnalysis, PatternDetector, Specificity};
use np_mock::{Action, BodySource, MatchSpec, Rule};
use np_model::{BodyDir, Envelope, Event};
use np_store::{FlowRow, Reader};
use std::collections::HashMap;
use std::path::PathBuf;

/// Options for flow-to-mock conversion.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub fixtures_dir: PathBuf,
    pub inline_threshold: usize,
    pub priority: i32,
    pub enabled: bool,
    pub match_query: bool,
    pub match_headers: Vec<String>,
    pub pattern_analysis: bool,
    pub pattern_specificity: Option<Specificity>,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            fixtures_dir: PathBuf::from("./fixtures"),
            inline_threshold: 1024,
            priority: 100,
            enabled: true,
            match_query: false,
            match_headers: Vec::new(),
            pattern_analysis: false,
            pattern_specificity: None,
        }
    }
}

/// Result of converting a flow to a mock rule.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversionResult {
    pub rule: Rule,
    pub fixtures: Vec<Fixture>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_analysis: Option<PathAnalysis>,
}

/// A fixture file to be written.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Fixture {
    pub path: String,
    #[serde(skip)]
    pub content: Vec<u8>,
    pub content_type: Option<String>,
    pub size: usize,
}

/// Convert a captured flow into a mock rule.
pub fn convert_flow(
    reader: &Reader,
    run: &str,
    flow_id: u64,
    options: &ConvertOptions,
) -> Result<ConversionResult, ConvertError> {
    // Get flow metadata
    let flow = reader
        .get_flow(run, flow_id)?
        .ok_or_else(|| ConvertError::FlowNotFound(flow_id))?;

    // Get all events for this flow
    let events = reader.flow_events(run, flow_id)?;

    let mut warnings = Vec::new();

    // Generate rule ID
    let rule_id = format!("flow-{}-{}", &run[..8.min(run.len())], flow_id);

    // Extract request/response info from events
    let (req_headers, resp_headers, resp_body_preview) = extract_from_events(&events);

    // Analyze path patterns if requested
    let pattern_analysis = if options.pattern_analysis {
        Some(PatternDetector::analyze(
            flow.path.as_deref().unwrap_or("/"),
        ))
    } else {
        None
    };

    // Determine final path to use in match spec
    let final_path = if let Some(specificity) = options.pattern_specificity {
        // Apply pattern specificity if specified
        pattern_analysis
            .as_ref()
            .and_then(|pa| {
                pa.suggestions
                    .iter()
                    .find(|s| s.specificity == specificity)
                    .map(|s| s.pattern.clone())
            })
            .or_else(|| flow.path.clone())
    } else {
        flow.path.clone()
    };

    // Build match spec
    let match_spec = build_match_spec_with_path(&flow, &req_headers, final_path, options);

    // Build response action
    let (action, fixtures) = build_response_action(
        &flow,
        &resp_headers,
        &resp_body_preview,
        &rule_id,
        options,
        &mut warnings,
    )?;

    let rule = Rule {
        id: rule_id,
        priority: options.priority,
        enabled: options.enabled,
        description: Some(format!(
            "Converted from flow: {} {} {}",
            flow.method.as_deref().unwrap_or("?"),
            flow.host.as_deref().unwrap_or("?"),
            flow.path.as_deref().unwrap_or("?")
        )),
        match_spec,
        action,
    };

    Ok(ConversionResult {
        rule,
        fixtures,
        warnings,
        pattern_analysis,
    })
}

type ExtractedData = (
    HashMap<String, String>,
    HashMap<String, String>,
    Option<Vec<u8>>,
);

fn extract_from_events(
    events: &[Envelope],
) -> ExtractedData {
    let mut req_headers = HashMap::new();
    let mut resp_headers = HashMap::new();
    let mut resp_body_preview = None;

    for envelope in events {
        match &envelope.event {
            Event::RequestHead { headers, .. } => {
                for (k, v) in headers {
                    req_headers.insert(k.to_lowercase(), v.clone());
                }
            }
            Event::ResponseHead { headers, .. } => {
                for (k, v) in headers {
                    resp_headers.insert(k.to_lowercase(), v.clone());
                }
            }
            Event::BodyEnd {
                dir: BodyDir::Res,
                preview_b64,
                stored_bytes,
                total,
                ..
            } => {
                // Use preview if body was small enough to be fully captured
                if *stored_bytes == *total && !preview_b64.is_empty() {
                    if let Ok(decoded) = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        preview_b64,
                    ) {
                        resp_body_preview = Some(decoded);
                    }
                }
            }
            _ => {}
        }
    }

    (req_headers, resp_headers, resp_body_preview)
}

fn build_match_spec_with_path(
    flow: &FlowRow,
    req_headers: &HashMap<String, String>,
    path: Option<String>,
    options: &ConvertOptions,
) -> MatchSpec {
    let mut spec = MatchSpec {
        method: flow.method.clone(),
        host: flow.host.clone(),
        path,
        ..Default::default()
    };

    // Add query parameter matching if requested
    if options.match_query {
        if let Some(ref uri) = flow.uri {
            if let Some(q_start) = uri.find('?') {
                let query = &uri[q_start + 1..];
                let query_params: HashMap<String, String> = query
                    .split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        match (parts.next(), parts.next()) {
                            (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                            _ => None,
                        }
                    })
                    .collect();

                if !query_params.is_empty() {
                    spec.query = Some(query_params);
                }
            }
        }
    }

    // Add header matching if requested
    if !options.match_headers.is_empty() {
        let mut headers = HashMap::new();
        for header_name in &options.match_headers {
            if let Some(value) = req_headers.get(&header_name.to_lowercase()) {
                headers.insert(header_name.clone(), value.clone());
            }
        }
        if !headers.is_empty() {
            spec.headers = Some(headers);
        }
    }

    spec
}

fn build_response_action(
    flow: &FlowRow,
    resp_headers: &HashMap<String, String>,
    resp_body_preview: &Option<Vec<u8>>,
    rule_id: &str,
    options: &ConvertOptions,
    warnings: &mut Vec<String>,
) -> Result<(Action, Vec<Fixture>), ConvertError> {
    let mut fixtures = Vec::new();
    let mut response_headers = HashMap::new();

    // Copy important response headers
    for (k, v) in resp_headers {
        if should_include_header(k) {
            response_headers.insert(k.clone(), v.clone());
        }
    }

    // Handle response body
    let body_source = if let Some(body_bytes) = resp_body_preview {
        if body_bytes.is_empty() {
            BodySource::Inline {
                inline: String::new(),
            }
        } else if body_bytes.len() <= options.inline_threshold {
            // Small body - try to inline it
            let content_type = resp_headers.get("content-type");
            if is_text_content(&content_type) {
                match String::from_utf8(body_bytes.clone()) {
                    Ok(text) => BodySource::Inline { inline: text },
                    Err(_) => {
                        warnings.push("Body contains invalid UTF-8, saving as file".to_string());
                        let ct = content_type.cloned();
                        create_fixture_file(rule_id, body_bytes, &ct, &mut fixtures)
                    }
                }
            } else {
                warnings.push("Binary content, saving as file".to_string());
                let ct = content_type.cloned();
                create_fixture_file(rule_id, body_bytes, &ct, &mut fixtures)
            }
        } else {
            // Large body - save as file
            let content_type = resp_headers.get("content-type").cloned();
            create_fixture_file(rule_id, body_bytes, &content_type, &mut fixtures)
        }
    } else {
        // No body preview available
        if flow.res_bytes > 0 {
            warnings.push(format!(
                "Response body ({} bytes) not available in preview - body was too large or stored separately",
                flow.res_bytes
            ));
        }
        BodySource::Inline {
            inline: String::new(),
        }
    };

    let status = flow.status.unwrap_or(200);

    let action = Action::Respond {
        status,
        headers: response_headers,
        body: body_source,
        delay_ms: None,
    };

    Ok((action, fixtures))
}

fn create_fixture_file(
    rule_id: &str,
    content: &[u8],
    content_type: &Option<String>,
    fixtures: &mut Vec<Fixture>,
) -> BodySource {
    // Determine file extension from content type
    let ext = content_type
        .as_ref()
        .and_then(|ct| match ct.split(';').next().unwrap().trim() {
            "application/json" => Some("json"),
            "text/html" => Some("html"),
            "text/plain" => Some("txt"),
            "application/xml" | "text/xml" => Some("xml"),
            "image/png" => Some("png"),
            "image/jpeg" | "image/jpg" => Some("jpg"),
            "image/gif" => Some("gif"),
            "image/svg+xml" => Some("svg"),
            _ => None,
        })
        .unwrap_or("bin");

    let filename = format!("{}.{}", rule_id, ext);

    fixtures.push(Fixture {
        path: filename.clone(),
        content: content.to_vec(),
        content_type: content_type.clone(),
        size: content.len(),
    });

    BodySource::File { file: filename }
}

fn should_include_header(name: &str) -> bool {
    // Skip hop-by-hop headers and headers that vary
    !matches!(
        name,
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "date"
            | "age"
            | "expires"
            | "etag"
            | "last-modified"
            | "vary"
            | "content-encoding" // Skip compression headers
    )
}

fn is_text_content(content_type: &Option<&String>) -> bool {
    content_type
        .map(|ct| {
            let ct = ct.split(';').next().unwrap().trim();
            ct.starts_with("text/")
                || ct == "application/json"
                || ct == "application/xml"
                || ct == "application/javascript"
                || ct == "application/x-javascript"
        })
        .unwrap_or(false)
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("store error: {0}")]
    Store(#[from] np_store::StoreError),

    #[error("flow {0} not found")]
    FlowNotFound(u64),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_include_header_filters_correctly() {
        assert!(should_include_header("content-type"));
        assert!(should_include_header("cache-control"));
        assert!(!should_include_header("connection"));
        assert!(!should_include_header("transfer-encoding"));
        assert!(!should_include_header("date"));
        assert!(!should_include_header("content-encoding"));
    }

    #[test]
    fn is_text_content_detects_text() {
        let text = "text/plain".to_string();
        let json = "application/json".to_string();
        let html = "text/html; charset=utf-8".to_string();
        let png = "image/png".to_string();

        assert!(is_text_content(&Some(&text)));
        assert!(is_text_content(&Some(&json)));
        assert!(is_text_content(&Some(&html)));
        assert!(!is_text_content(&Some(&png)));
        assert!(!is_text_content(&None));
    }
}
