//! SQLite persistence: an append-only event log (source of truth) plus a `flows` projection kept
//! in the same transaction, so queries never see a flow that is ahead of or behind its events.
//!
//! * One writer thread consumes the sequenced envelopes from the bus in batches.
//! * Readers open their own connections (WAL), so UI/MCP queries never block capture.
//! * Opening the store repairs sessions that died mid-capture (see [`Store::open`]).

use np_model::*;
use rusqlite::{params, params_from_iter, types::Value, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, sync::Arc, thread::JoinHandle};
use tokio::sync::mpsc;

pub const STORE_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store schema {found} is newer than this build supports ({supported})")]
    TooNew { found: i64, supported: i64 },
}

pub type Result<T> = std::result::Result<T, StoreError>;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS runs (
  run TEXT PRIMARY KEY, started_us INTEGER NOT NULL, ended_us INTEGER,
  engine TEXT, listen TEXT, status TEXT NOT NULL DEFAULT 'running',
  flow_count INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS events (
  run TEXT NOT NULL, seq INTEGER NOT NULL, ts_us INTEGER NOT NULL, kind TEXT NOT NULL,
  flow INTEGER, conn INTEGER, data TEXT NOT NULL,
  PRIMARY KEY (run, seq)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS events_flow ON events (run, flow, seq) WHERE flow IS NOT NULL;
-- Rare, diagnostic-only kinds: lets the UI fetch history without scanning every body/progress event.
CREATE INDEX IF NOT EXISTS events_diag ON events (run, seq) WHERE kind IN ('client_tls_failed','tls_hello','connection_opened','connection_closed','events_dropped');
CREATE TABLE IF NOT EXISTS flows (
  run TEXT NOT NULL, flow INTEGER NOT NULL, conn INTEGER, state TEXT NOT NULL,
  client TEXT, method TEXT, uri TEXT, host TEXT, path TEXT, http_version TEXT,
  started_us INTEGER NOT NULL, ended_us INTEGER, status INTEGER, ttfb_us INTEGER,
  req_bytes INTEGER NOT NULL DEFAULT 0, res_bytes INTEGER NOT NULL DEFAULT 0,
  req_sha256 TEXT, res_sha256 TEXT, mocked INTEGER NOT NULL DEFAULT 0, rule TEXT,
  fail_class TEXT, fail_detail TEXT, first_seq INTEGER NOT NULL, last_seq INTEGER NOT NULL,
  PRIMARY KEY (run, flow)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS flows_time ON flows (run, started_us DESC, flow DESC);
CREATE INDEX IF NOT EXISTS flows_time_all ON flows (started_us DESC, flow DESC);
CREATE INDEX IF NOT EXISTS flows_host ON flows (host, started_us DESC);
CREATE INDEX IF NOT EXISTS flows_status ON flows (status);
";

#[derive(Clone)]
pub struct Store {
    path: Arc<PathBuf>,
}

fn configure(c: &Connection) -> rusqlite::Result<()> {
    c.pragma_update(None, "journal_mode", "WAL")?;
    c.pragma_update(None, "synchronous", "NORMAL")?;
    c.busy_timeout(std::time::Duration::from_secs(5))
}

impl Store {
    /// Creates/migrates the database and repairs sessions interrupted by a crash or power loss:
    /// runs with no `ended_us` become `interrupted` and their still-open flows `aborted`.
    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        let path = path.as_ref().to_path_buf();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let c = Connection::open(&path)?;
        configure(&c)?;
        let found: Option<i64> = c
            .query_row("SELECT value FROM meta WHERE key='schema_version'", [], |r| r.get::<_, String>(0))
            .optional()
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok());
        if let Some(found) = found {
            if found > STORE_SCHEMA_VERSION {
                return Err(StoreError::TooNew { found, supported: STORE_SCHEMA_VERSION });
            }
        }
        c.execute_batch(SCHEMA)?;
        c.execute("INSERT OR IGNORE INTO meta VALUES ('schema_version', ?1)", [STORE_SCHEMA_VERSION.to_string()])?;
        c.execute_batch(
            "UPDATE flows SET state='aborted', fail_detail='session interrupted',
                    ended_us=COALESCE(ended_us,(SELECT MAX(ts_us) FROM events e WHERE e.run=flows.run))
              WHERE state='open' AND run IN (SELECT run FROM runs WHERE ended_us IS NULL);
             UPDATE runs SET status='interrupted',
                    ended_us=COALESCE((SELECT MAX(ts_us) FROM events e WHERE e.run=runs.run), started_us)
              WHERE ended_us IS NULL;",
        )?;
        Ok(Store { path: Arc::new(path) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reader(&self) -> Result<Reader> {
        let c = Connection::open_with_flags(&*self.path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Reader { c })
    }

    /// Spawns the single writer thread. It ends when `rx` is closed and drained.
    pub fn spawn_writer(&self, rx: mpsc::Receiver<Arc<Envelope>>) -> Result<WriterHandle> {
        self.spawn_writer_with(rx, || {})
    }

    /// Like [`spawn_writer`](Self::spawn_writer); `on_commit` runs after every committed batch
    /// (used to wake live subscribers).
    pub fn spawn_writer_with(&self, rx: mpsc::Receiver<Arc<Envelope>>, on_commit: impl Fn() + Send + 'static) -> Result<WriterHandle> {
        self.spawn_writer_retaining(rx, on_commit, None)
    }

    /// Writer that also enforces `retention` itself, in small time-boxed steps between batches.
    pub fn spawn_writer_retaining(&self, mut rx: mpsc::Receiver<Arc<Envelope>>, on_commit: impl Fn() + Send + 'static, retention: Option<PrunePolicy>) -> Result<WriterHandle> {
        let mut c = Connection::open(&*self.path)?;
        configure(&c)?;
        let join = std::thread::Builder::new().name("np-store-writer".into()).spawn(move || {
            let mut batch: Vec<Arc<Envelope>> = Vec::with_capacity(512);
            let mut last_check = std::time::Instant::now();
            while let Some(first) = rx.blocking_recv() {
                batch.push(first);
                while batch.len() < 2048 {
                    match rx.try_recv() {
                        Ok(e) => batch.push(e),
                        Err(_) => break,
                    }
                }
                match write_batch(&mut c, &batch) {
                    Ok(()) => on_commit(),
                    Err(e) => eprintln!("np-store: batch of {} events failed: {e}", batch.len()),
                }
                if let (Some(pol), Some(last)) = (retention, batch.last()) {
                    if last_check.elapsed() >= std::time::Duration::from_millis(250) {
                        retain_step(&c, &pol, &last.run);
                        last_check = std::time::Instant::now();
                    }
                }
                batch.clear();
            }
        })
        .expect("spawn writer");
        Ok(WriterHandle { join: Some(join) })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PrunePolicy {
    /// Upper bound for live database content (pages in use, excluding free pages).
    pub max_bytes: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PruneReport {
    pub runs_deleted: u64,
    pub events_deleted: u64,
    pub flows_deleted: u64,
    pub bytes_before: u64,
    pub bytes_after: u64,
}

const PRUNE_BATCH: i64 = 20_000;

impl Store {
    /// Bytes of live content (allocated pages minus the free list). The file may stay larger; freed
    /// pages are reused by new writes, so this is the number retention bounds.
    pub fn used_bytes(&self) -> Result<u64> {
        let c = Connection::open_with_flags(&*self.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        used_bytes(&c)
    }

    /// Enforces `policy` synchronously (used when no capture is running; while capturing, the
    /// writer thread prunes in small time-boxed steps itself so the two never fight for SQLite's
    /// single write lock). Deletes whole old sessions first, then the oldest part of the active one.
    pub fn prune(&self, policy: &PrunePolicy, active_run: Option<&str>) -> Result<PruneReport> {
        let c = Connection::open(&*self.path)?;
        configure(&c)?;
        let mut rep = PruneReport { bytes_before: used_bytes(&c)?, ..Default::default() };
        while used_bytes(&c)? > policy.max_bytes {
            if !prune_step(&c, active_run, PRUNE_BATCH, &mut rep)? {
                break;
            }
        }
        let _ = c.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        rep.bytes_after = used_bytes(&c)?;
        Ok(rep)
    }
}

/// One bounded deletion step, oldest data first. Returns false when there is nothing left to delete
/// (or, for the active run, too little to be worth trimming).
fn prune_step(c: &Connection, active: Option<&str>, n: i64, rep: &mut PruneReport) -> Result<bool> {
    let Some(victim): Option<String> = c.query_row("SELECT run FROM runs ORDER BY started_us ASC LIMIT 1", [], |r| r.get(0)).optional()? else {
        return Ok(false);
    };
    let is_active = active == Some(victim.as_str());
    if is_active {
        // Cheap "at least 1000 events" probe (a COUNT(*) here is O(n) on every step).
        let enough: Option<i64> = c
            .query_row("SELECT seq FROM events WHERE run=?1 ORDER BY seq LIMIT 1 OFFSET 999", [&victim], |r| r.get(0))
            .optional()?;
        if enough.is_none() {
            return Ok(false);
        }
    }
    // Highest flow id touched by the chunk we are about to delete. Flow ids grow with time, so every
    // flow fully covered by the deleted events has an id <= this; bounding the flows delete by it
    // keeps the scan to the chunk's own span instead of the whole table.
    let max_flow: Option<i64> = c.query_row(
        "SELECT MAX(flow) FROM (SELECT flow FROM events WHERE run=?1 ORDER BY seq LIMIT ?2)",
        params![victim, n],
        |r| r.get(0),
    )?;
    let ev = c.execute(
        "DELETE FROM events WHERE run=?1 AND seq IN (SELECT seq FROM events WHERE run=?1 ORDER BY seq LIMIT ?2)",
        params![victim, n],
    )? as u64;
    rep.events_deleted += ev;
    let oldest: Option<i64> = c.query_row("SELECT MIN(seq) FROM events WHERE run=?1", [&victim], |r| r.get(0))?;
    let fl = match oldest {
        Some(min_seq) => match max_flow {
            Some(mf) => c.execute(
                "DELETE FROM flows WHERE run=?1 AND flow<=?2 AND last_seq<?3 AND state!='open'",
                params![victim, mf, min_seq],
            )?,
            None => 0,
        },
        // Run drained of events: remove its remaining flows in bounded chunks.
        None => c.execute(
            "DELETE FROM flows WHERE run=?1 AND flow IN (SELECT flow FROM flows WHERE run=?1 LIMIT ?2)",
            params![victim, n],
        )?,
    } as u64;
    rep.flows_deleted += fl;
    if fl > 0 {
        c.execute("UPDATE runs SET flow_count = MAX(flow_count - ?2, 0) WHERE run=?1", params![victim, fl as i64])?;
    }
    if oldest.is_none() && fl == 0 && !is_active {
        c.execute("DELETE FROM runs WHERE run=?1", [&victim])?;
        rep.runs_deleted += 1;
        return Ok(true);
    }
    Ok(ev + fl > 0)
}

fn used_bytes(c: &Connection) -> Result<u64> {
    let pages: i64 = c.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let free: i64 = c.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
    let size: i64 = c.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    Ok(((pages - free).max(0) * size) as u64)
}

/// Deletes oldest data for at most ~100 ms when over the cap, aiming slightly below it so checks
/// are not needed on every batch.
fn retain_step(c: &Connection, pol: &PrunePolicy, active: &str) {
    let Ok(mut used) = used_bytes(c) else { return };
    if used <= pol.max_bytes {
        return;
    }
    let target = pol.max_bytes / 100 * 97;
    let t = std::time::Instant::now();
    let mut rep = PruneReport::default();
    while used > target && t.elapsed() < std::time::Duration::from_millis(100) {
        match prune_step(c, Some(active), 5_000, &mut rep) {
            Ok(true) => used = used_bytes(c).unwrap_or(0),
            _ => break,
        }
    }
}

pub struct WriterHandle {
    join: Option<JoinHandle<()>>,
}

impl WriterHandle {
    /// Blocks until the channel feeding the writer has been closed and fully flushed.
    pub fn join(mut self) {
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn path_of(uri: &str) -> String {
    let rest = uri.split_once("://").map_or(uri, |(_, r)| r);
    let p = rest.find('/').map_or("/", |i| &rest[i..]);
    p.split('?').next().unwrap_or("/").to_owned()
}

fn write_batch(c: &mut Connection, batch: &[Arc<Envelope>]) -> Result<()> {
    let tx = c.transaction()?;
    {
        let mut ins = tx.prepare_cached("INSERT OR IGNORE INTO events VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
        for e in batch {
            let data = serde_json::to_string(&e.event)?;
            ins.execute(params![e.run, e.seq as i64, e.ts_us as i64, e.event.kind(), e.event.flow().map(|f| f.0 as i64), e.event.conn().map(|c| c.0 as i64), data])?;
            project(&tx, e)?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn project(tx: &rusqlite::Transaction<'_>, e: &Envelope) -> Result<()> {
    use Event::*;
    let (run, seq, ts) = (e.run.as_str(), e.seq as i64, e.ts_us as i64);
    match &e.event {
        EngineStarted { listen, engine, .. } => {
            tx.execute("INSERT OR IGNORE INTO runs (run, started_us, engine, listen) VALUES (?1,?2,?3,?4)", params![run, ts, engine, listen])?;
        }
        EngineStopped { .. } => {
            tx.execute("UPDATE runs SET ended_us=?2, status='stopped' WHERE run=?1", params![run, ts])?;
        }
        RequestHead { flow, conn, client, method, uri, host, http_version, .. } => {
            tx.prepare_cached(
                "INSERT OR IGNORE INTO flows (run, flow, conn, state, client, method, uri, host, path, http_version, started_us, first_seq, last_seq)
                 VALUES (?1,?2,?3,'open',?4,?5,?6,?7,?8,?9,?10,?11,?11)",
            )?
            .execute(params![run, flow.0 as i64, conn.map(|c| c.0 as i64), client, method, uri, host, path_of(uri), http_version, ts, seq])
            .and_then(|inserted| {
                if inserted > 0 {
                    tx.prepare_cached("UPDATE runs SET flow_count = flow_count + 1 WHERE run=?1")?.execute([run])?;
                }
                Ok(())
            })?;
        }
        ResponseHead { flow, status, ttfb_us, mocked, .. } => {
            upd(tx, run, *flow, seq, "status=?4, ttfb_us=?5, mocked=(mocked OR ?6)", &[&(*status as i64), &(*ttfb_us as i64), &(*mocked as i64)])?
        }
        BodyProgress { flow, dir, total } => {
            upd(tx, run, *flow, seq, if *dir == BodyDir::Req { "req_bytes=?4" } else { "res_bytes=?4" }, &[&(*total as i64)])?
        }
        BodyError { flow, dir, total, .. } | BodyAborted { flow, dir, total } => {
            upd(tx, run, *flow, seq, if *dir == BodyDir::Req { "req_bytes=?4" } else { "res_bytes=?4" }, &[&(*total as i64)])?;
            if *dir == BodyDir::Res {
                terminal(tx, run, *flow, ts, "aborted", None, None)?;
            }
        }
        BodyEnd { flow, dir, total, sha256, .. } => {
            let set = if *dir == BodyDir::Req { "req_bytes=?4, req_sha256=?5" } else { "res_bytes=?4, res_sha256=?5" };
            upd(tx, run, *flow, seq, set, &[&(*total as i64), sha256])?
        }
        MockHit { flow, rule } => upd(tx, run, *flow, seq, "mocked=1, rule=?4", &[rule])?,
        FlowCompleted { flow, status, mocked } => {
            upd(tx, run, *flow, seq, "status=COALESCE(?4,status), mocked=(mocked OR ?5)", &[&status.map(|s| s as i64), &(*mocked as i64)])?;
            terminal(tx, run, *flow, ts, "completed", None, None)?;
        }
        FlowFailed { flow, class, detail } => {
            let class = serde_json::to_value(class)?.as_str().unwrap_or("other").to_owned();
            terminal(tx, run, *flow, ts, "failed", Some(&class), Some(detail))?;
        }
        BodyFirstByte { flow, .. } => upd(tx, run, *flow, seq, "last_seq=last_seq", &[])?,
        _ => {}
    }
    Ok(())
}

/// `set` may use ?4.. for `vals`; ?1=run ?2=flow ?3=seq are fixed.
fn upd(tx: &rusqlite::Transaction<'_>, run: &str, flow: FlowId, seq: i64, set: &str, vals: &[&dyn rusqlite::ToSql]) -> Result<()> {
    let sql = format!("UPDATE flows SET {set}, last_seq=MAX(last_seq, ?3) WHERE run=?1 AND flow=?2");
    let flow_i = flow.0 as i64;
    let mut p: Vec<&dyn rusqlite::ToSql> = vec![&run, &flow_i, &seq];
    p.extend_from_slice(vals);
    tx.prepare_cached(&sql)?.execute(params_from_iter(p))?;
    Ok(())
}

/// Moves an *open* flow to a terminal state (first terminal event wins).
fn terminal(tx: &rusqlite::Transaction<'_>, run: &str, flow: FlowId, ts: i64, state: &str, class: Option<&str>, detail: Option<&str>) -> Result<()> {
    tx.prepare_cached(
        "UPDATE flows SET state=?3, ended_us=?4, fail_class=COALESCE(?5,fail_class), fail_detail=COALESCE(?6,fail_detail)
          WHERE run=?1 AND flow=?2 AND state='open'",
    )?
    .execute(params![run, flow.0 as i64, state, ts, class, detail])?;
    Ok(())
}

// ------------------------------------------------------------------ reads

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowRow {
    pub run: String,
    pub flow: u64,
    pub conn: Option<u64>,
    pub state: String,
    pub client: Option<String>,
    pub method: Option<String>,
    pub uri: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
    pub http_version: Option<String>,
    pub started_us: i64,
    pub ended_us: Option<i64>,
    pub status: Option<u16>,
    pub ttfb_us: Option<i64>,
    pub req_bytes: i64,
    pub res_bytes: i64,
    pub req_sha256: Option<String>,
    pub res_sha256: Option<String>,
    pub mocked: bool,
    pub rule: Option<String>,
    pub fail_class: Option<String>,
    pub fail_detail: Option<String>,
    pub first_seq: u64,
    pub last_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunRow {
    pub run: String,
    pub started_us: i64,
    pub ended_us: Option<i64>,
    pub engine: Option<String>,
    pub listen: Option<String>,
    /// `running` | `stopped` | `interrupted`
    pub status: String,
    pub events: i64,
    pub flows: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FlowFilter {
    pub run: Option<String>,
    pub host: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    /// 2, 3, 4, 5 => 2xx..5xx
    pub status_class: Option<u16>,
    pub state: Option<String>,
    pub mocked: Option<bool>,
    /// Substring of the full URI (case-insensitive).
    pub text: Option<String>,
    pub since_us: Option<i64>,
    pub until_us: Option<i64>,
}

/// Keyset cursor (newest first): pass the previous page's `next` to continue.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub started_us: i64,
    pub flow: u64,
}

pub struct Reader {
    c: Connection,
}

const FLOW_COLS: &str = "run, flow, conn, state, client, method, uri, host, path, http_version, started_us, ended_us, status, ttfb_us,
    req_bytes, res_bytes, req_sha256, res_sha256, mocked, rule, fail_class, fail_detail, first_seq, last_seq";

fn flow_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<FlowRow> {
    Ok(FlowRow {
        run: r.get(0)?, flow: r.get::<_, i64>(1)? as u64, conn: r.get::<_, Option<i64>>(2)?.map(|v| v as u64),
        state: r.get(3)?, client: r.get(4)?, method: r.get(5)?, uri: r.get(6)?, host: r.get(7)?, path: r.get(8)?,
        http_version: r.get(9)?, started_us: r.get(10)?, ended_us: r.get(11)?,
        status: r.get::<_, Option<i64>>(12)?.map(|v| v as u16), ttfb_us: r.get(13)?, req_bytes: r.get(14)?, res_bytes: r.get(15)?,
        req_sha256: r.get(16)?, res_sha256: r.get(17)?, mocked: r.get::<_, i64>(18)? != 0, rule: r.get(19)?,
        fail_class: r.get(20)?, fail_detail: r.get(21)?, first_seq: r.get::<_, i64>(22)? as u64, last_seq: r.get::<_, i64>(23)? as u64,
    })
}

impl Reader {
    pub fn runs(&self) -> Result<Vec<RunRow>> {
        let mut s = self.c.prepare(
            "SELECT r.run, r.started_us, r.ended_us, r.engine, r.listen, r.status,
                    -- seq is contiguous (retention only trims the head), so the span is the count; both are O(log n)
                    COALESCE((SELECT MAX(seq) FROM events e WHERE e.run=r.run) - (SELECT MIN(seq) FROM events e WHERE e.run=r.run) + 1, 0),
                    r.flow_count
               FROM runs r ORDER BY r.started_us DESC",
        )?;
        let rows = s.query_map([], |r| {
            Ok(RunRow { run: r.get(0)?, started_us: r.get(1)?, ended_us: r.get(2)?, engine: r.get(3)?, listen: r.get(4)?, status: r.get(5)?, events: r.get(6)?, flows: r.get(7)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn latest_run(&self) -> Result<Option<String>> {
        Ok(self.c.query_row("SELECT run FROM runs ORDER BY started_us DESC LIMIT 1", [], |r| r.get(0)).optional()?)
    }

    /// Newest-first page of flows. Returns the cursor for the next page when more rows exist.
    pub fn list_flows(&self, f: &FlowFilter, limit: usize, after: Option<Cursor>) -> Result<(Vec<FlowRow>, Option<Cursor>)> {
        let mut w: Vec<String> = vec![];
        let mut p: Vec<Value> = vec![];
        let mut add = |cond: &str, v: Value| {
            p.push(v);
            w.push(cond.replace('?', &format!("?{}", p.len())));
        };
        if let Some(v) = &f.run { add("run = ?", v.clone().into()); }
        if let Some(v) = &f.host { add("host = ?", v.clone().into()); }
        if let Some(v) = &f.method { add("method = ?", v.to_uppercase().into()); }
        if let Some(v) = f.status { add("status = ?", (v as i64).into()); }
        if let Some(v) = f.status_class { add("status >= ?", ((v as i64) * 100).into()); add("status < ?", ((v as i64 + 1) * 100).into()); }
        if let Some(v) = &f.state { add("state = ?", v.clone().into()); }
        if let Some(v) = f.mocked { add("mocked = ?", (v as i64).into()); }
        if let Some(v) = &f.text { add("uri LIKE ? ESCAPE '\\'", format!("%{}%", v.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")).into()); }
        if let Some(v) = f.since_us { add("started_us >= ?", v.into()); }
        if let Some(v) = f.until_us { add("started_us <= ?", v.into()); }
        if let Some(c) = after {
            p.push((c.started_us).into());
            let a = p.len();
            p.push((c.flow as i64).into());
            let b = p.len();
            w.push(format!("(started_us < ?{a} OR (started_us = ?{a} AND flow < ?{b}))"));
        }
        let where_sql = if w.is_empty() { String::new() } else { format!("WHERE {}", w.join(" AND ")) };
        let sql = format!("SELECT {FLOW_COLS} FROM flows {where_sql} ORDER BY started_us DESC, flow DESC LIMIT {}", limit + 1);
        let mut s = self.c.prepare(&sql)?;
        let mut rows: Vec<FlowRow> = s.query_map(params_from_iter(p), flow_row)?.collect::<rusqlite::Result<_>>()?;
        let next = if rows.len() > limit {
            rows.truncate(limit);
            rows.last().map(|r| Cursor { started_us: r.started_us, flow: r.flow })
        } else {
            None
        };
        Ok((rows, next))
    }

    pub fn get_flow(&self, run: &str, flow: u64) -> Result<Option<FlowRow>> {
        Ok(self.c.query_row(&format!("SELECT {FLOW_COLS} FROM flows WHERE run=?1 AND flow=?2"), params![run, flow as i64], flow_row).optional()?)
    }

    /// Rows for the given flow ids (any order); unknown ids are skipped. Used to refresh visible rows.
    pub fn get_flows(&self, run: &str, flows: &[u64]) -> Result<Vec<FlowRow>> {
        if flows.is_empty() {
            return Ok(vec![]);
        }
        let marks = (0..flows.len()).map(|i| format!("?{}", i + 2)).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT {FLOW_COLS} FROM flows WHERE run=?1 AND flow IN ({marks})");
        let mut args: Vec<Value> = vec![run.to_owned().into()];
        args.extend(flows.iter().map(|f| Value::from(*f as i64)));
        let mut s = self.c.prepare(&sql)?;
        let rows = s.query_map(params_from_iter(args), flow_row)?.collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    pub fn flow_events(&self, run: &str, flow: u64) -> Result<Vec<Envelope>> {
        let mut s = self.c.prepare("SELECT seq, ts_us, data FROM events WHERE run=?1 AND flow=?2 ORDER BY seq")?;
        let rows = s.query_map(params![run, flow as i64], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
        rows.map(|r| { let (seq, ts, data) = r?; Ok(Envelope { run: run.to_owned(), seq: seq as u64, ts_us: ts as u64, event: serde_json::from_str(&data)? }) }).collect()
    }

    /// Events with `seq > after`, in order. This is how a client resyncs after a lagged live stream.
    pub fn events_since(&self, run: &str, after: u64, limit: usize) -> Result<Vec<Envelope>> {
        let mut s = self.c.prepare("SELECT seq, ts_us, data FROM events WHERE run=?1 AND seq>?2 ORDER BY seq LIMIT ?3")?;
        let rows = s.query_map(params![run, after as i64, limit as i64], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
        rows.map(|r| { let (seq, ts, data) = r?; Ok(Envelope { run: run.to_owned(), seq: seq as u64, ts_us: ts as u64, event: serde_json::from_str(&data)? }) }).collect()
    }

    /// Newest-first diagnostic events (`client_tls_failed`, `tls_hello`, `connection_opened`,
    /// `connection_closed`, `events_dropped`), served from a partial index.
    pub fn diagnostic_events(&self, run: &str, limit: usize) -> Result<Vec<Envelope>> {
        let mut s = self.c.prepare(
            "SELECT seq, ts_us, data FROM events
              WHERE run=?1 AND kind IN ('client_tls_failed','tls_hello','connection_opened','connection_closed','events_dropped')
              ORDER BY seq DESC LIMIT ?2",
        )?;
        let rows = s.query_map(params![run, limit as i64], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
        rows.map(|r| { let (seq, ts, data) = r?; Ok(Envelope { run: run.to_owned(), seq: seq as u64, ts_us: ts as u64, event: serde_json::from_str(&data)? }) }).collect()
    }

    /// Oldest retained `seq` of a run (greater than 1 after retention trimmed its head).
    pub fn first_seq(&self, run: &str) -> Result<u64> {
        Ok(self.c.query_row("SELECT COALESCE(MIN(seq),0) FROM events WHERE run=?1", [run], |r| r.get::<_, i64>(0))? as u64)
    }

    pub fn last_seq(&self, run: &str) -> Result<u64> {
        Ok(self.c.query_row("SELECT COALESCE(MAX(seq),0) FROM events WHERE run=?1", [run], |r| r.get::<_, i64>(0))? as u64)
    }
}
