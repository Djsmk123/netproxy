//! File-backed body store: `<dir>/<run>/<flow>.<req|res>` holds the raw wire bytes of a body.
//!
//! Capture must never slow traffic, so `write` only clones a `Bytes` handle into a bounded queue
//! drained by one writer thread. If the queue is full or a body exceeds the per-body cap the bytes
//! are dropped and the body is reported `truncated`. A separate cap bounds the whole directory;
//! the oldest files are evicted first and readers see `evicted` instead of an error.

use bytes::Bytes;
use np_engine::{BodyCapture, BodySink, BodyWriter};
use np_model::{BodyDir, FlowId};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{sync_channel, SyncSender, TrySendError},
        Arc,
    },
};

/// Upper bound on body bytes queued for the writer thread. Beyond it new bytes are dropped (and the
/// body marked truncated) instead of growing memory or blocking the data path.
pub const QUEUE_BYTES_BUDGET: u64 = 32 << 20;

/// Bodies up to this size are not written to a file: the engine's event preview already holds
/// them completely, and readers fall back to it (`bodies.get`).
pub const INLINE_LIMIT: usize = 4096;

enum Op {
    Data {
        key: u64,
        path: Arc<PathBuf>,
        data: Bytes,
        budget: Arc<AtomicU64>,
    },
    Close {
        key: u64,
    },
}

pub struct FileBodySink {
    root: PathBuf,
    run: String,
    tx: SyncSender<Op>,
    per_body_cap: u64,
    next_key: AtomicU64,
    queued: Arc<AtomicU64>,
    pub dropped_bytes: Arc<AtomicU64>,
}

impl FileBodySink {
    pub fn new(
        root: &Path,
        run: &str,
        per_body_cap: u64,
        queue: usize,
    ) -> std::io::Result<FileBodySink> {
        let dir = root.join(run);
        fs::create_dir_all(&dir)?;
        let (tx, rx) = sync_channel::<Op>(queue);
        std::thread::Builder::new()
            .name("np-body-writer".into())
            .spawn(move || {
                let mut open: HashMap<u64, File> = HashMap::new();
                while let Ok(op) = rx.recv() {
                    match op {
                        Op::Data {
                            key,
                            path,
                            data,
                            budget,
                        } => {
                            let f = open
                                .entry(key)
                                .or_insert_with(|| File::create(&*path).expect("create body file"));
                            let _ = f.write_all(&data);
                            budget.fetch_sub(data.len() as u64, Ordering::Relaxed);
                        }
                        Op::Close { key } => {
                            open.remove(&key);
                        }
                    }
                }
            })?;
        Ok(FileBodySink {
            root: root.to_path_buf(),
            run: run.to_owned(),
            tx,
            per_body_cap,
            next_key: AtomicU64::new(1),
            queued: Arc::new(AtomicU64::new(0)),
            dropped_bytes: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn path_for(root: &Path, run: &str, flow: u64, dir: BodyDir) -> PathBuf {
        root.join(run).join(format!(
            "{flow}.{}",
            if dir == BodyDir::Req { "req" } else { "res" }
        ))
    }
}

struct FileWriter {
    tx: SyncSender<Op>,
    key: u64,
    path: Arc<PathBuf>,
    cap: u64,
    stored: u64,
    truncated: bool,
    dropped: Arc<AtomicU64>,
    queued: Arc<AtomicU64>,
    closed: AtomicBool,
    /// Bytes held back until we know whether the body fits inline.
    inline: Vec<u8>,
    spilled: bool,
}

impl FileWriter {
    fn enqueue(&mut self, data: Bytes) {
        let n = data.len() as u64;
        if self.queued.fetch_add(n, Ordering::Relaxed) + n > QUEUE_BYTES_BUDGET {
            self.queued.fetch_sub(n, Ordering::Relaxed);
            self.truncated = true;
            self.dropped.fetch_add(n, Ordering::Relaxed);
            return;
        }
        match self.tx.try_send(Op::Data {
            key: self.key,
            path: self.path.clone(),
            data,
            budget: self.queued.clone(),
        }) {
            Ok(()) => self.stored += n,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.queued.fetch_sub(n, Ordering::Relaxed);
                // Never skip bytes in the middle of a body: once one is lost, stop storing it.
                self.truncated = true;
                self.dropped.fetch_add(n, Ordering::Relaxed);
            }
        }
    }
}

impl BodyWriter for FileWriter {
    fn write(&mut self, data: &Bytes) {
        if self.truncated {
            self.dropped.fetch_add(data.len() as u64, Ordering::Relaxed);
            return;
        }
        let room = self
            .cap
            .saturating_sub(self.stored + self.inline.len() as u64) as usize;
        let take = data.len().min(room);
        if take < data.len() {
            self.truncated = true;
            self.dropped
                .fetch_add((data.len() - take) as u64, Ordering::Relaxed);
        }
        if take == 0 {
            return;
        }
        if !self.spilled {
            if self.inline.len() + take <= INLINE_LIMIT {
                self.inline.extend_from_slice(&data[..take]);
                return;
            }
            self.spilled = true;
            let held = Bytes::from(std::mem::take(&mut self.inline));
            if !held.is_empty() {
                self.enqueue(held);
            }
        }
        if !self.truncated {
            self.enqueue(data.slice(..take));
        }
    }

    fn finish(self: Box<Self>, total: u64) -> BodyCapture {
        if !self.spilled {
            // Entire body (<= INLINE_LIMIT) lives in the event preview; no file was ever created.
            let stored = self.inline.len() as u64;
            return BodyCapture {
                stored_bytes: stored,
                truncated: self.truncated || stored < total,
            };
        }
        if !self.closed.swap(true, Ordering::Relaxed) {
            // Close must not be lost, or the file handle would leak in the writer thread.
            let _ = self.tx.send(Op::Close { key: self.key });
        }
        BodyCapture {
            stored_bytes: self.stored,
            truncated: self.truncated || self.stored < total,
        }
    }
}

impl BodySink for FileBodySink {
    fn open(&self, flow: FlowId, dir: BodyDir) -> Option<Box<dyn BodyWriter>> {
        Some(Box::new(FileWriter {
            tx: self.tx.clone(),
            key: self.next_key.fetch_add(1, Ordering::Relaxed),
            path: Arc::new(Self::path_for(&self.root, &self.run, flow.0, dir)),
            cap: self.per_body_cap,
            stored: 0,
            truncated: false,
            dropped: self.dropped_bytes.clone(),
            queued: self.queued.clone(),
            closed: AtomicBool::new(false),
            inline: Vec::new(),
            spilled: false,
        }))
    }
}

/// Reads up to `len` bytes at `offset`. `Ok(None)` => the body was never stored or has been evicted.
pub fn read_range(
    root: &Path,
    run: &str,
    flow: u64,
    dir: BodyDir,
    offset: u64,
    len: usize,
) -> std::io::Result<Option<(Vec<u8>, u64)>> {
    let p = FileBodySink::path_for(root, run, flow, dir);
    let mut f = match File::open(&p) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let size = f.metadata()?.len();
    f.seek(SeekFrom::Start(offset.min(size)))?;
    let mut buf = vec![0u8; len.min((size.saturating_sub(offset)) as usize)];
    f.read_exact(&mut buf)?;
    Ok(Some((buf, size)))
}

/// Deletes oldest files (by mtime) until the directory is at most `max_bytes`, and drops run
/// directories that no longer exist in the database. Returns bytes freed.
pub fn enforce_cap(
    root: &Path,
    max_bytes: u64,
    keep_runs: &dyn Fn(&str) -> bool,
) -> std::io::Result<u64> {
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = vec![];
    let mut freed = 0u64;
    let Ok(rd) = fs::read_dir(root) else {
        return Ok(0);
    };
    for run in rd.flatten() {
        let name = run.file_name().to_string_lossy().into_owned();
        if !run.path().is_dir() {
            continue;
        }
        if !keep_runs(&name) {
            freed += dir_size(&run.path());
            let _ = fs::remove_dir_all(run.path());
            continue;
        }
        for f in fs::read_dir(run.path())?.flatten() {
            if let Ok(md) = f.metadata() {
                files.push((
                    md.modified().unwrap_or(std::time::UNIX_EPOCH),
                    md.len(),
                    f.path(),
                ));
            }
        }
    }
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total > max_bytes {
        files.sort();
        let target = max_bytes / 100 * 90;
        for (_, size, path) in files {
            if total <= target {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total -= size;
                freed += size;
            }
        }
    }
    Ok(freed)
}

fn dir_size(p: &Path) -> u64 {
    fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}
