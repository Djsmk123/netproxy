//! Control API: JSON-RPC 2.0, one JSON object per line, over a Unix domain socket.
//!
//! Security model (there is no unauthenticated control endpoint):
//! * the socket lives in a 0700 directory and is itself 0600;
//! * the peer's uid must equal the daemon's effective uid;
//! * the first message must be `hello` carrying the per-launch token (read from a 0600 file);
//!   anything else closes the connection without a reply;
//! * requests are size-limited and the handshake has a deadline.
//!
//! Live events are *not* pushed from memory: a subscriber reads the durable log in order and is
//! woken by store commits, so a slow client is paced by its own socket and can never lose,
//! duplicate or reorder events, nor slow capture. A reconnecting client resumes with `since`.

use crate::daemon::{Daemon, DaemonError};
use crate::pattern_detector::{PatternDetector, Specificity};
use futures::{SinkExt, StreamExt};
use np_model::{API_VERSION, SCHEMA_VERSION};
use np_store::{Cursor, FlowFilter};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::mpsc,
    task::JoinHandle,
};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};

/// sockaddr_un.sun_path is 104 bytes on macOS (108 on Linux), including the NUL.
const MAX_SOCKET_PATH: usize = 100;
const MAX_LINE: usize = 1 << 20;
const HELLO_DEADLINE: Duration = Duration::from_secs(5);

pub struct ControlInfo {
    pub socket: PathBuf,
    /// `<run>/control.endpoint`: one line holding the real socket path (it may differ from `<run>/control.sock`).
    pub endpoint_file: PathBuf,
    pub token_file: PathBuf,
    pub token: String,
}

pub struct ControlServer {
    pub info: ControlInfo,
    cancel: CancellationToken,
    task: JoinHandle<()>,
    /// Held for the daemon's lifetime: an exclusive flock proving this is the only daemon here.
    _lock: std::fs::File,
}

impl ControlServer {
    pub async fn stop(self) {
        self.cancel.cancel();
        let _ = self.task.await;
        let _ = std::fs::remove_file(&self.info.socket);
        let _ = std::fs::remove_file(&self.info.endpoint_file);
        let _ = std::fs::remove_file(&self.info.token_file);
    }
}

fn euid() -> u32 {
    unsafe { libc::geteuid() }
}

fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let md = std::fs::metadata(dir)?;
    use std::os::unix::fs::MetadataExt;
    if md.uid() != euid() || md.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} must be owned by the current user with mode 0700",
                dir.display()
            ),
        ));
    }
    Ok(())
}

fn random_token() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("os randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn format_datetime_iso8601(us: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let secs = (us / 1_000_000) as u64;
    let nanos = ((us % 1_000_000) * 1000) as u32;
    let datetime = UNIX_EPOCH + Duration::new(secs, nanos);
    // Format as ISO8601: YYYY-MM-DDTHH:MM:SS.sssZ
    let since_epoch = datetime.duration_since(UNIX_EPOCH).unwrap();
    let secs = since_epoch.as_secs();
    let millis = since_epoch.subsec_millis();

    // Simple conversion (not handling timezones properly, but good enough for HAR)
    let days_since_epoch = secs / 86400;
    let year = 1970 + (days_since_epoch / 365); // Approximation
    let remaining_days = days_since_epoch % 365;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;

    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, minute, second, millis
    )
}

/// The error kind returned when another daemon already owns this data directory.
pub const ALREADY_RUNNING: &str = "another netproxyd is already running for this data directory";

pub async fn serve(daemon: Arc<Daemon>, run_dir: &Path) -> io::Result<ControlServer> {
    ensure_private_dir(run_dir)?;
    // Single-instance guard: never clobber a live daemon's socket/token.
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(run_dir.join("daemon.lock"))?;
    if unsafe {
        libc::flock(
            std::os::fd::AsRawFd::as_raw_fd(&lock),
            libc::LOCK_EX | libc::LOCK_NB,
        )
    } != 0
    {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, ALREADY_RUNNING));
    }
    // macOS limits sockaddr_un paths to ~104 bytes. If `<data>/run/control.sock` does not fit, bind a
    // short private path instead; clients always learn the real location from `control.endpoint`.
    let socket = {
        let preferred = run_dir.join("control.sock");
        if preferred.as_os_str().len() <= MAX_SOCKET_PATH {
            preferred
        } else {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            run_dir.hash(&mut h);
            let short_dir = PathBuf::from(format!("/tmp/np-{}", euid()));
            ensure_private_dir(&short_dir)?;
            short_dir.join(format!("{:016x}.sock", h.finish()))
        }
    };
    let endpoint_file = run_dir.join("control.endpoint");
    let token_file = run_dir.join("control.token");
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    std::fs::write(&endpoint_file, socket.to_string_lossy().as_bytes())?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    let token = random_token();
    let _ = std::fs::remove_file(&token_file);
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&token_file)?;
        f.write_all(token.as_bytes())?;
    }
    let cancel = CancellationToken::new();
    let (c2, t2) = (cancel.clone(), token.clone());
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = c2.cancelled() => break,
                acc = listener.accept() => {
                    let Ok((stream, _)) = acc else { continue };
                    match stream.peer_cred() {
                        Ok(c) if c.uid() == euid() => {}
                        _ => continue, // foreign uid: drop silently
                    }
                    let (d, t, c) = (daemon.clone(), t2.clone(), c2.child_token());
                    tokio::spawn(async move { let _ = connection(d, stream, t, c).await; });
                }
            }
        }
    });
    Ok(ControlServer {
        info: ControlInfo {
            socket,
            endpoint_file,
            token_file,
            token,
        },
        cancel,
        task,
        _lock: lock,
    })
}

fn ok(id: &Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

fn err(id: &Value, code: i32, msg: impl Into<String>) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg.into()}}).to_string()
}

const E_PARSE: i32 = -32700;
const E_REQUEST: i32 = -32600;
const E_METHOD: i32 = -32601;
const E_PARAMS: i32 = -32602;
const E_APP: i32 = -32000;
const E_VERSION: i32 = -32002;

#[derive(Deserialize)]
struct Hello {
    token: String,
    api_version: u32,
    #[serde(default)]
    client: Value,
}

async fn connection(
    daemon: Arc<Daemon>,
    stream: UnixStream,
    token: String,
    cancel: CancellationToken,
) -> io::Result<()> {
    let framed = Framed::new(stream, LinesCodec::new_with_max_length(MAX_LINE));
    let (mut sink, mut lines) = framed.split();

    // --- handshake: first line must be a valid hello, else close silently.
    let first = match tokio::time::timeout(HELLO_DEADLINE, lines.next()).await {
        Ok(Some(Ok(l))) => l,
        _ => return Ok(()),
    };
    let req: Value = serde_json::from_str(&first).unwrap_or(Value::Null);
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let hello: Option<Hello> = (req.get("method") == Some(&json!("hello")))
        .then(|| serde_json::from_value(req["params"].clone()).ok())
        .flatten();
    let Some(h) = hello.filter(|h| ct_eq(&h.token, &token)) else {
        return Ok(());
    };
    if h.api_version != API_VERSION {
        let _ = sink
            .send(err(
                &id,
                E_VERSION,
                format!(
                    "unsupported api_version {}; server speaks {API_VERSION}",
                    h.api_version
                ),
            ))
            .await;
        return Ok(());
    }
    let client_name = h
        .client
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    sink.send(ok(
        &id,
        json!({
            "api_version": API_VERSION, "schema_version": SCHEMA_VERSION,
            "daemon_version": env!("CARGO_PKG_VERSION"), "status": daemon.status(),
        }),
    ))
    .await
    .map_err(io::Error::other)?;

    // --- single writer task; everything else sends lines through a bounded channel (backpressure).
    let (out_tx, mut out_rx) = mpsc::channel::<String>(256);
    let wcancel = cancel.clone();
    let writer = tokio::spawn(async move {
        while let Some(l) = out_rx.recv().await {
            if sink.send(l).await.is_err() {
                break;
            }
        }
        wcancel.cancel();
    });

    let mut subs: std::collections::HashMap<u64, CancellationToken> = Default::default();
    let mut next_sub = 0u64;
    loop {
        let line = tokio::select! {
            _ = cancel.cancelled() => break,
            l = lines.next() => match l { Some(Ok(l)) => l, _ => break },
        };
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = out_tx.send(err(&Value::Null, E_PARSE, e.to_string())).await;
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let Some(method) = req.get("method").and_then(Value::as_str) else {
            let _ = out_tx.send(err(&id, E_REQUEST, "missing method")).await;
            continue;
        };
        let params = req.get("params").cloned().unwrap_or(Value::Null);
        let reply = dispatch(
            &daemon,
            &client_name,
            method,
            params,
            &out_tx,
            &cancel,
            &mut subs,
            &mut next_sub,
        )
        .await;
        let line = match reply {
            Ok(v) => ok(&id, v),
            Err((code, msg)) => err(&id, code, msg),
        };
        if out_tx.send(line).await.is_err() {
            break;
        }
    }
    cancel.cancel();
    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

type Reply = Result<Value, (i32, String)>;

fn p<T: for<'de> Deserialize<'de>>(params: Value) -> Result<T, (i32, String)> {
    serde_json::from_value(params).map_err(|e| (E_PARAMS, format!("invalid params: {e}")))
}

fn app<E: std::fmt::Display>(e: E) -> (i32, String) {
    (E_APP, e.to_string())
}

#[allow(clippy::too_many_arguments)]
async fn dispatch(
    d: &Arc<Daemon>,
    client: &str,
    method: &str,
    params: Value,
    out: &mpsc::Sender<String>,
    conn_cancel: &CancellationToken,
    subs: &mut std::collections::HashMap<u64, CancellationToken>,
    next_sub: &mut u64,
) -> Reply {
    let resolve_run = |run: Option<String>| -> Result<String, (i32, String)> {
        match run.as_deref() {
            None | Some("current") => d
                .current_or_latest_run()
                .ok_or_else(|| app("no capture runs yet")),
            Some(r) => Ok(r.to_owned()),
        }
    };
    match method {
        "ping" => Ok(json!({"pong": true})),
        "engine.status" => Ok(json!(d.status())),
        "engine.start" => {
            #[derive(Deserialize, Default)]
            struct P {
                listen: Option<std::net::SocketAddr>,
            }
            let P { listen } = if params.is_null() {
                P::default()
            } else {
                p(params)?
            };
            tracing::info!(client, "engine.start");
            d.start(listen).await.map(|s| json!(s)).map_err(app)
        }
        "engine.stop" => {
            tracing::info!(client, "engine.stop");
            d.stop().await.map(|s| json!(s)).map_err(|e| match e {
                DaemonError::NotRunning => app("engine is not running"),
                o => app(o),
            })
        }
        "bodies.get" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                flow: u64,
                dir: np_model::BodyDir,
                offset: Option<u64>,
                length: Option<usize>,
            }
            let P {
                run,
                flow,
                dir,
                offset,
                length,
            } = p(params)?;
            let run = resolve_run(run)?;
            let run_again = run.clone();
            let (root, off, len) = (
                d.bodies_dir(),
                offset.unwrap_or(0),
                length.unwrap_or(256 * 1024).clamp(1, 1 << 20),
            );
            match tokio::task::spawn_blocking(move || {
                crate::bodies::read_range(&root, &run, flow, dir, off, len)
            })
            .await
            {
                Ok(Ok(Some((bytes, size)))) => {
                    use base64::Engine as _;
                    Ok(
                        json!({"data_b64": base64::engine::general_purpose::STANDARD.encode(&bytes), "offset": off, "size": size,
                              "eof": off + bytes.len() as u64 >= size, "evicted": false}),
                    )
                }
                Ok(Ok(None)) => {
                    // No file: either a small body held inline in its BodyEnd preview, or evicted.
                    let store = d.store.clone();
                    let (run2, dir2) = (run_again, dir);
                    let inline = typed(move || {
                        Ok(store
                            .reader()?
                            .flow_events(&run2, flow)?
                            .into_iter()
                            .find_map(|e| match e.event {
                                np_model::Event::BodyEnd {
                                    dir: bd,
                                    total,
                                    preview_b64,
                                    stored_bytes,
                                    ..
                                } if bd == dir2 && stored_bytes == total => Some(preview_b64),
                                _ => None,
                            }))
                    })
                    .await?;
                    match inline {
                        Some(b64) => {
                            use base64::Engine as _;
                            let all = base64::engine::general_purpose::STANDARD
                                .decode(&b64)
                                .unwrap_or_default();
                            let start = (off as usize).min(all.len());
                            let end = (start + len).min(all.len());
                            Ok(
                                json!({"data_b64": base64::engine::general_purpose::STANDARD.encode(&all[start..end]), "offset": off,
                                      "size": all.len(), "eof": end >= all.len(), "evicted": false, "inline": true}),
                            )
                        }
                        None => Ok(
                            json!({"data_b64": "", "offset": off, "size": 0, "eof": true, "evicted": true}),
                        ),
                    }
                }
                Ok(Err(e)) => Err(app(e)),
                Err(e) => Err(app(e)),
            }
        }
        "storage.status" => {
            let store = d.store.clone();
            let used = typed(move || store.used_bytes()).await?;
            Ok(json!({"used_bytes": used, "max_bytes": d.max_db_bytes()}))
        }
        "storage.prune" => {
            #[derive(Deserialize)]
            struct P {
                max_bytes: u64,
            }
            let P { max_bytes } = p(params)?;
            tracing::info!(client, max_bytes, "storage.prune");
            d.prune_now(max_bytes).await.map(|r| json!(r)).map_err(app)
        }
        "ca.get" => Ok(json!({"cert_pem": d.ca_cert_pem()})),
        "runs.list" => {
            let store = d.store.clone();
            blocking(move || store.reader()?.runs()).await
        }
        "runs.delete" => {
            #[derive(Deserialize)]
            struct P {
                run: String,
            }
            let P { run } = p(params)?;
            tracing::info!(client, %run, "runs.delete");
            let store = d.store.clone();
            let run_id = run.clone();
            blocking(move || store.delete_run(&run_id)).await?;
            Ok(json!({"deleted": run}))
        }
        "runs.export" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                format: String,
                host: Option<String>,
            }
            let P { run, format, host } = p(params)?;
            let run = resolve_run(run)?;
            tracing::info!(client, %run, %format, "runs.export");

            match format.as_str() {
                "json" => {
                    let store = d.store.clone();
                    let mut filter = FlowFilter {
                        run: Some(run),
                        ..Default::default()
                    };
                    if let Some(h) = host {
                        filter.host = Some(h);
                    }
                    let (flows, _) =
                        typed(move || store.reader()?.list_flows(&filter, 10000, None)).await?;
                    Ok(json!({"flows": flows, "format": "json", "version": 1}))
                }
                "csv" => {
                    let store = d.store.clone();
                    let mut filter = FlowFilter {
                        run: Some(run.clone()),
                        ..Default::default()
                    };
                    if let Some(h) = host {
                        filter.host = Some(h);
                    }
                    let (flows, _) =
                        typed(move || store.reader()?.list_flows(&filter, 10000, None)).await?;

                    // Build CSV
                    let mut csv = String::from("flow,method,host,path,status,duration_ms,req_bytes,res_bytes,started_at,state,mocked\n");
                    for f in &flows {
                        let method = f.method.as_deref().unwrap_or("");
                        let host = f.host.as_deref().unwrap_or("");
                        let path = f.path.as_deref().unwrap_or("");
                        let status = f.status.map(|s| s.to_string()).unwrap_or_default();
                        let duration_ms = f
                            .ended_us
                            .map(|e| ((e - f.started_us) as f64 / 1000.0).to_string())
                            .unwrap_or_default();
                        let started = f.started_us.to_string();
                        csv.push_str(&format!(
                            "{},{},{},{},{},{},{},{},{},{},{}\n",
                            f.flow,
                            escape_csv(method),
                            escape_csv(host),
                            escape_csv(path),
                            status,
                            duration_ms,
                            f.req_bytes,
                            f.res_bytes,
                            started,
                            f.state,
                            f.mocked
                        ));
                    }

                    Ok(json!({"csv": csv, "format": "csv", "rows": flows.len()}))
                }
                "har" => {
                    let store = d.store.clone();
                    let mut filter = FlowFilter {
                        run: Some(run.clone()),
                        ..Default::default()
                    };
                    if let Some(h) = host {
                        filter.host = Some(h);
                    }
                    let (flows, _) =
                        typed(move || store.reader()?.list_flows(&filter, 10000, None)).await?;

                    // Build HAR 1.2 format
                    let entries: Vec<Value> = flows.iter().map(|f| {
                        let started_datetime = format_datetime_iso8601(f.started_us);
                        let duration_ms = f.ended_us.map(|e| (e - f.started_us) as f64 / 1000.0).unwrap_or(0.0);

                        json!({
                            "startedDateTime": started_datetime,
                            "time": duration_ms,
                            "request": {
                                "method": f.method.as_deref().unwrap_or("GET"),
                                "url": f.uri.as_deref().unwrap_or(""),
                                "httpVersion": f.http_version.as_deref().unwrap_or("HTTP/1.1"),
                                "headers": [],
                                "queryString": [],
                                "cookies": [],
                                "headersSize": -1,
                                "bodySize": f.req_bytes
                            },
                            "response": {
                                "status": f.status.unwrap_or(0),
                                "statusText": "",
                                "httpVersion": f.http_version.as_deref().unwrap_or("HTTP/1.1"),
                                "headers": [],
                                "cookies": [],
                                "content": {
                                    "size": f.res_bytes,
                                    "mimeType": "application/octet-stream"
                                },
                                "redirectURL": "",
                                "headersSize": -1,
                                "bodySize": f.res_bytes
                            },
                            "cache": {},
                            "timings": {
                                "send": 0,
                                "wait": f.ttfb_us.map(|t| t as f64 / 1000.0).unwrap_or(0.0),
                                "receive": duration_ms - f.ttfb_us.map(|t| t as f64 / 1000.0).unwrap_or(0.0)
                            },
                            "_mocked": f.mocked,
                            "_rule": f.rule
                        })
                    }).collect();

                    let har = json!({
                        "log": {
                            "version": "1.2",
                            "creator": {
                                "name": "NetProxy",
                                "version": env!("CARGO_PKG_VERSION")
                            },
                            "entries": entries
                        }
                    });

                    Ok(har)
                }
                _ => Err((E_PARAMS, format!("unsupported format: {}", format))),
            }
        }
        "runs.import" => {
            #[derive(Deserialize)]
            struct P {
                har_json: String,
                name: Option<String>,
            }
            let P { har_json, name } = p(params)?;
            tracing::info!(client, "runs.import");

            // Parse HAR JSON
            let har_value: Value = serde_json::from_str(&har_json)
                .map_err(|e| (E_PARAMS, format!("invalid HAR JSON: {}", e)))?;

            let entries = har_value
                .get("log")
                .and_then(|log| log.get("entries"))
                .and_then(Value::as_array)
                .ok_or_else(|| (E_PARAMS, "HAR must have log.entries array".to_string()))?;

            let run_name = name.unwrap_or_else(|| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                format!("imported-{}", now)
            });

            // For now, return import stats without actually inserting
            // Full implementation would create flows in the database
            Ok(json!({
                "imported": run_name,
                "entries": entries.len(),
                "message": "HAR import not yet fully implemented in database"
            }))
        }
        "breakpoints.add" => {
            #[derive(Deserialize)]
            struct P {
                pattern: String,
                enabled: Option<bool>,
            }
            let P { pattern, enabled } = p(params)?;
            tracing::info!(client, %pattern, "breakpoints.add");
            // TODO: Store breakpoints in daemon state
            Ok(
                json!({"id": format!("bp-{}", pattern.replace("/", "-")), "pattern": pattern, "enabled": enabled.unwrap_or(true)}),
            )
        }
        "breakpoints.list" => {
            tracing::info!(client, "breakpoints.list");
            // TODO: Return actual breakpoints from daemon state
            Ok(json!({"breakpoints": []}))
        }
        "breakpoints.remove" => {
            #[derive(Deserialize)]
            struct P {
                id: String,
            }
            let P { id } = p(params)?;
            tracing::info!(client, %id, "breakpoints.remove");
            // TODO: Remove from daemon state
            Ok(json!({"removed": id}))
        }
        "breakpoints.toggle" => {
            #[derive(Deserialize)]
            struct P {
                id: String,
                enabled: bool,
            }
            let P { id, enabled } = p(params)?;
            tracing::info!(client, %id, %enabled, "breakpoints.toggle");
            // TODO: Update daemon state
            Ok(json!({"id": id, "enabled": enabled}))
        }
        "flows.list" => {
            #[derive(Deserialize, Default)]
            struct P {
                #[serde(default)]
                filter: FlowFilter,
                limit: Option<usize>,
                cursor: Option<Cursor>,
            }
            let P {
                mut filter,
                limit,
                cursor,
            } = if params.is_null() {
                P::default()
            } else {
                p(params)?
            };
            if filter.run.as_deref() == Some("current") {
                filter.run = Some(resolve_run(None)?);
            }
            let limit = limit.unwrap_or(100).clamp(1, 500);
            let store = d.store.clone();
            let (rows, next) =
                typed(move || store.reader()?.list_flows(&filter, limit, cursor)).await?;
            Ok(json!({"flows": rows, "next": next}))
        }
        "flows.get_many" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                flows: Vec<u64>,
            }
            let P { run, flows } = p(params)?;
            if flows.len() > 500 {
                return Err((E_PARAMS, "at most 500 flows per call".into()));
            }
            let run = resolve_run(run)?;
            let store = d.store.clone();
            blocking(move || store.reader()?.get_flows(&run, &flows))
                .await
                .map(|rows| json!({"flows": rows}))
        }
        "flows.get" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                flow: u64,
            }
            let P { run, flow } = p(params)?;
            let run = resolve_run(run)?;
            let store = d.store.clone();
            let (row, events) = typed(move || {
                let r = store.reader()?;
                Ok((r.get_flow(&run, flow)?, r.flow_events(&run, flow)?))
            })
            .await?;
            Ok(json!({"flow": row, "events": events}))
        }
        "events.diagnostics" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                limit: Option<usize>,
            }
            let P { run, limit } = if params.is_null() {
                P {
                    run: None,
                    limit: None,
                }
            } else {
                p(params)?
            };
            let run = resolve_run(run)?;
            let limit = limit.unwrap_or(2000).clamp(1, 5000);
            let store = d.store.clone();
            // oldest-first so clients can fold them in order
            let mut evs = typed(move || store.reader()?.diagnostic_events(&run, limit)).await?;
            evs.reverse();
            Ok(json!({"events": evs}))
        }
        "events.since" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                seq: u64,
                limit: Option<usize>,
            }
            let P { run, seq, limit } = p(params)?;
            let run = resolve_run(run)?;
            let limit = limit.unwrap_or(500).clamp(1, 1000);
            let store = d.store.clone();
            let events = blocking(move || store.reader()?.events_since(&run, seq, limit)).await?;
            Ok(json!({"events": events}))
        }
        "events.subscribe" => {
            #[derive(Deserialize, Default)]
            struct P {
                run: Option<String>,
                since: Option<u64>,
            }
            let P { run, since } = if params.is_null() {
                P::default()
            } else {
                p(params)?
            };
            let run = resolve_run(run)?;
            let store = d.store.clone();
            let r2 = run.clone();
            let start = match since {
                Some(s) => s,
                None => typed(move || store.reader()?.last_seq(&r2)).await?, // tail
            };
            *next_sub += 1;
            let id = *next_sub;
            let tok = conn_cancel.child_token();
            subs.insert(id, tok.clone());
            tokio::spawn(follow(d.clone(), run.clone(), start, id, out.clone(), tok));
            Ok(json!({"subscription": id, "run": run, "after_seq": start}))
        }
        "events.unsubscribe" => {
            #[derive(Deserialize)]
            struct P {
                subscription: u64,
            }
            let P { subscription } = p(params)?;
            let found = subs.remove(&subscription).map(|t| t.cancel()).is_some();
            Ok(json!({"unsubscribed": found}))
        }
        "mocks.list" => {
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let rules = registry.list_rules();
            Ok(json!({"rules": rules}))
        }
        "mocks.list_dynamic" => {
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let rules = registry.list_dynamic_rules();
            Ok(json!({"rules": rules}))
        }
        "mocks.list_static" => {
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let rules = registry.list_static_rules();
            Ok(json!({"rules": rules}))
        }
        "mocks.get" => {
            #[derive(Deserialize)]
            struct P {
                id: String,
            }
            let P { id } = p(params)?;
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let rule = registry
                .get_rule(&id)
                .ok_or_else(|| app(format!("rule not found: {}", id)))?;
            Ok(json!({"rule": rule}))
        }
        "mocks.add" => {
            #[derive(Deserialize)]
            struct P {
                rule: np_mock::Rule,
            }
            let P { rule } = p(params)?;
            tracing::info!(client, rule_id = %rule.id, "mocks.add");
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;

            // Validate before adding
            let issues = registry.validate_rule(&rule);
            let errors: Vec<_> = issues
                .iter()
                .filter(|i| matches!(i.severity, np_mock::Severity::Error))
                .collect();

            if !errors.is_empty() {
                return Err(app(format!("validation errors: {:?}", errors)));
            }

            registry.add_rule(rule).map_err(app)?;
            Ok(json!({"success": true}))
        }
        "mocks.update" => {
            #[derive(Deserialize)]
            struct P {
                rule: np_mock::Rule,
            }
            let P { rule } = p(params)?;
            tracing::info!(client, rule_id = %rule.id, "mocks.update");
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;

            // Validate before updating
            let issues = registry.validate_rule(&rule);
            let errors: Vec<_> = issues
                .iter()
                .filter(|i| matches!(i.severity, np_mock::Severity::Error))
                .collect();

            if !errors.is_empty() {
                return Err(app(format!("validation errors: {:?}", errors)));
            }

            let updated = registry.update_rule(rule).map_err(app)?;
            Ok(json!({"updated": updated}))
        }
        "mocks.remove" => {
            #[derive(Deserialize)]
            struct P {
                id: String,
            }
            let P { id } = p(params)?;
            tracing::info!(client, rule_id = %id, "mocks.remove");
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let removed = registry.remove_rule(&id).map_err(app)?;
            Ok(json!({"removed": removed}))
        }
        "mocks.clear_dynamic" => {
            tracing::info!(client, "mocks.clear_dynamic");
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            registry.clear_dynamic_rules();
            Ok(json!({"success": true}))
        }
        "mocks.reload" => {
            tracing::info!(client, "mocks.reload");
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            registry.reload().map_err(app)?;

            // Return validation issues
            let issues = registry.validate_all();
            Ok(json!({"success": true, "issues": issues}))
        }
        "mocks.validate" => {
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;
            let issues = registry.validate_all();
            Ok(json!({"issues": issues}))
        }
        "mocks.analyze_path" => {
            #[derive(Deserialize)]
            struct P {
                path: String,
            }
            let P { path } = p(params)?;
            tracing::info!(client, path = %path, "mocks.analyze_path");

            let analysis = PatternDetector::analyze(&path);
            Ok(serde_json::to_value(analysis).map_err(app)?)
        }
        "mocks.from_flow" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                flow: u64,
                #[serde(default)]
                priority: Option<i32>,
                #[serde(default)]
                enabled: Option<bool>,
                #[serde(default)]
                inline_threshold: Option<usize>,
                #[serde(default)]
                match_query: Option<bool>,
                #[serde(default)]
                match_headers: Option<Vec<String>>,
                #[serde(default)]
                write_fixtures: bool,
                #[serde(default)]
                pattern_analysis: Option<bool>,
                #[serde(default)]
                pattern_specificity: Option<String>,
            }
            let P {
                run,
                flow,
                priority,
                enabled,
                inline_threshold,
                match_query,
                match_headers,
                write_fixtures,
                pattern_analysis,
                pattern_specificity,
            } = p(params)?;

            let run = resolve_run(run)?;

            tracing::info!(client, run = %run, flow, "mocks.from_flow");

            // Get fixtures directory
            let fixtures_dir = d
                .mock_config_path()
                .and_then(|p| p.parent())
                .map(|p| p.join("fixtures"))
                .unwrap_or_else(|| d.data_dir().join("fixtures"));

            // Parse pattern specificity
            let parsed_specificity = pattern_specificity.as_deref().and_then(|s| match s {
                "exact" => Some(Specificity::Exact),
                "moderate" => Some(Specificity::Moderate),
                "broad" => Some(Specificity::Broad),
                _ => None,
            });

            let options = crate::flow_converter::ConvertOptions {
                fixtures_dir: fixtures_dir.clone(),
                inline_threshold: inline_threshold.unwrap_or(1024),
                priority: priority.unwrap_or(100),
                enabled: enabled.unwrap_or(true),
                match_query: match_query.unwrap_or(false),
                match_headers: match_headers.unwrap_or_default(),
                pattern_analysis: pattern_analysis.unwrap_or(false),
                pattern_specificity: parsed_specificity,
            };

            // Convert flow to mock
            let store = d.store.clone();
            let result = tokio::task::spawn_blocking(move || {
                let reader = store.reader()?;
                crate::flow_converter::convert_flow(&reader, &run, flow, &options)
            })
            .await
            .map_err(app)?
            .map_err(app)?;

            // Write fixtures if requested
            if write_fixtures && !result.fixtures.is_empty() {
                std::fs::create_dir_all(&fixtures_dir)
                    .map_err(|e| app(format!("failed to create fixtures directory: {}", e)))?;

                for fixture in &result.fixtures {
                    let path = fixtures_dir.join(&fixture.path);
                    std::fs::write(&path, &fixture.content).map_err(|e| {
                        app(format!("failed to write fixture {}: {}", fixture.path, e))
                    })?;
                }
            }

            let mut response = json!({
                "rule": result.rule,
                "fixtures": result.fixtures.iter().map(|f| json!({
                    "path": f.path,
                    "size": f.size,
                    "content_type": f.content_type,
                })).collect::<Vec<_>>(),
                "warnings": result.warnings,
                "fixtures_written": write_fixtures && !result.fixtures.is_empty(),
            });

            if let Some(analysis) = result.pattern_analysis {
                response["pattern_analysis"] = serde_json::to_value(analysis).map_err(app)?;
            }

            Ok(response)
        }
        "mocks.from_flow_and_add" => {
            #[derive(Deserialize)]
            struct P {
                run: Option<String>,
                flow: u64,
                #[serde(default)]
                priority: Option<i32>,
                #[serde(default)]
                enabled: Option<bool>,
                #[serde(default)]
                inline_threshold: Option<usize>,
                #[serde(default)]
                match_query: Option<bool>,
                #[serde(default)]
                match_headers: Option<Vec<String>>,
                #[serde(default)]
                pattern_analysis: Option<bool>,
                #[serde(default)]
                pattern_specificity: Option<String>,
            }
            let P {
                run,
                flow,
                priority,
                enabled,
                inline_threshold,
                match_query,
                match_headers,
                pattern_analysis,
                pattern_specificity,
            } = p(params)?;

            let run_resolved = resolve_run(run)?;

            tracing::info!(client, run = %run_resolved, flow, "mocks.from_flow_and_add");

            // Get fixtures directory
            let fixtures_dir = d
                .mock_config_path()
                .and_then(|p| p.parent())
                .map(|p| p.join("fixtures"))
                .unwrap_or_else(|| d.data_dir().join("fixtures"));

            // Parse pattern specificity
            let parsed_specificity = pattern_specificity.as_deref().and_then(|s| match s {
                "exact" => Some(Specificity::Exact),
                "moderate" => Some(Specificity::Moderate),
                "broad" => Some(Specificity::Broad),
                _ => None,
            });

            let options = crate::flow_converter::ConvertOptions {
                fixtures_dir: fixtures_dir.clone(),
                inline_threshold: inline_threshold.unwrap_or(1024),
                priority: priority.unwrap_or(100),
                enabled: enabled.unwrap_or(true),
                match_query: match_query.unwrap_or(false),
                match_headers: match_headers.unwrap_or_default(),
                pattern_analysis: pattern_analysis.unwrap_or(false),
                pattern_specificity: parsed_specificity,
            };

            // Convert flow to mock
            let store = d.store.clone();
            let result = tokio::task::spawn_blocking(move || {
                let reader = store.reader()?;
                crate::flow_converter::convert_flow(&reader, &run_resolved, flow, &options)
            })
            .await
            .map_err(app)?
            .map_err(app)?;

            // Write fixtures
            if !result.fixtures.is_empty() {
                std::fs::create_dir_all(&fixtures_dir)
                    .map_err(|e| app(format!("failed to create fixtures directory: {}", e)))?;

                for fixture in &result.fixtures {
                    let path = fixtures_dir.join(&fixture.path);
                    std::fs::write(&path, &fixture.content).map_err(|e| {
                        app(format!("failed to write fixture {}: {}", fixture.path, e))
                    })?;
                }
            }

            // Add the rule to the registry
            let registry = d
                .mock_registry()
                .ok_or_else(|| app("mock registry not configured"))?;

            // Validate before adding
            let issues = registry.validate_rule(&result.rule);
            let errors: Vec<_> = issues
                .iter()
                .filter(|i| matches!(i.severity, np_mock::Severity::Error))
                .collect();

            if !errors.is_empty() {
                return Err(app(format!("validation errors: {:?}", errors)));
            }

            registry.add_rule(result.rule.clone()).map_err(app)?;

            let mut response = json!({
                "success": true,
                "rule": result.rule,
                "fixtures_written": !result.fixtures.is_empty(),
                "warnings": result.warnings,
            });

            if let Some(analysis) = result.pattern_analysis {
                response["pattern_analysis"] = serde_json::to_value(analysis).map_err(app)?;
            }

            Ok(response)
        }
        other => Err((E_METHOD, format!("unknown method {other}"))),
    }
}

async fn typed<T, F>(f: F) -> Result<T, (i32, String)>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, np_store::StoreError> + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(app(e)),
        Err(e) => Err(app(e)),
    }
}

async fn blocking<T, F>(f: F) -> Result<Value, (i32, String)>
where
    T: serde::Serialize + Send + 'static,
    F: FnOnce() -> Result<T, np_store::StoreError> + Send + 'static,
{
    typed(f).await.map(|v| json!(v))
}

/// Streams events with `seq > after` in order, forever, until cancelled or the client goes away.
async fn follow(
    d: Arc<Daemon>,
    run: String,
    mut after: u64,
    sub: u64,
    out: mpsc::Sender<String>,
    cancel: CancellationToken,
) {
    let mut commits = d.commits();
    let Ok(mut reader) = d.store.reader() else {
        return;
    };
    loop {
        let r = run.clone();
        let (rd, chunk) = match tokio::task::spawn_blocking(move || {
            let res = reader.events_since(&r, after, 500);
            (reader, res)
        })
        .await
        {
            Ok((rd, Ok(c))) => (rd, c),
            _ => return,
        };
        reader = rd;
        if chunk.is_empty() {
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = commits.changed() => {}
                _ = tokio::time::sleep(Duration::from_millis(500)) => {}
            }
            continue;
        }
        for env in chunk {
            after = env.seq;
            let line = json!({"jsonrpc": "2.0", "method": "event", "params": {"subscription": sub, "event": env}}).to_string();
            tokio::select! {
                _ = cancel.cancelled() => return,
                r = out.send(line) => if r.is_err() { return },
            }
        }
    }
}
