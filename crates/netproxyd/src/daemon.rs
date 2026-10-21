//! Engine lifecycle owner: wires CA, store writer, bus and engine into numbered capture runs.

use crate::{breakpoint_engine::PausedRequestStore, watcher::{MockConfigWatcher, WatcherError}};
use np_engine::{
    Bus, BusConfig, CaMaterial, EngineConfig, EngineError, EngineHandle, KeyStorage, MockResolver,
    NoMocks, ProxyEngine,
};
use np_mock::DynamicMockRegistry;
use np_store::{Store, StoreError, WriterHandle};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tokio::sync::{mpsc, watch, Mutex};

/// Breakpoint configuration for request interception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: String,
    pub pattern: String,
    pub enabled: bool,
}

/// In-memory breakpoint storage.
#[derive(Debug, Default)]
pub struct BreakpointStore {
    breakpoints: HashMap<String, Breakpoint>,
}

impl BreakpointStore {
    pub fn add(&mut self, pattern: String, enabled: bool) -> Breakpoint {
        let id = format!("bp-{}", uuid::Uuid::new_v4());
        let bp = Breakpoint {
            id: id.clone(),
            pattern,
            enabled,
        };
        self.breakpoints.insert(id, bp.clone());
        bp
    }

    pub fn list(&self) -> Vec<Breakpoint> {
        self.breakpoints.values().cloned().collect()
    }

    pub fn remove(&mut self, id: &str) -> Option<Breakpoint> {
        self.breakpoints.remove(id)
    }

    pub fn toggle(&mut self, id: &str, enabled: bool) -> Option<Breakpoint> {
        self.breakpoints.get_mut(id).map(|bp| {
            bp.enabled = enabled;
            bp.clone()
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("engine is already running")]
    AlreadyRunning,
    #[error("engine is not running")]
    NotRunning,
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Watcher(#[from] WatcherError),
    #[error(transparent)]
    MockRegistry(#[from] np_mock::RegistryError),
}

pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub default_listen: SocketAddr,
    pub upstream_roots_pem: Option<Vec<u8>>,
    pub mock: Arc<dyn MockResolver>,
    pub mock_registry: Option<Arc<DynamicMockRegistry>>,
    pub mock_config_path: Option<PathBuf>,
    /// Retention cap for live database content. `None` disables pruning.
    pub max_db_bytes: Option<u64>,
    /// Body capture: `None` disables it. Per-body cap and total cap in bytes.
    pub bodies: Option<BodyConfig>,
    /// Where the CA private key lives. Tests use `File`; the shipped daemon uses the Keychain.
    pub ca_storage: KeyStorage,
}

#[derive(Clone, Copy)]
pub struct BodyConfig {
    pub per_body_cap: u64,
    pub total_cap: u64,
}

impl DaemonConfig {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            default_listen: "127.0.0.1:8899".parse().unwrap(),
            upstream_roots_pem: None,
            mock: Arc::new(NoMocks),
            mock_registry: None,
            mock_config_path: None,
            max_db_bytes: Some(2 << 30),
            bodies: Some(BodyConfig {
                per_body_cap: 4 << 20,
                total_cap: 1 << 30,
            }),
            ca_storage: KeyStorage::File,
        }
    }

    /// Load mock rules from a YAML config file using DynamicMockRegistry.
    ///
    /// This enables hot-reload and runtime rule management.
    pub fn with_mock_config(mut self, config_path: PathBuf) -> Result<Self, DaemonError> {
        let registry = Arc::new(DynamicMockRegistry::new(
            config_path.clone(),
            self.mock.clone(),
        )?);

        // Validate rules on startup
        let issues = registry.validate_all();
        for issue in &issues {
            match issue.severity {
                np_mock::Severity::Error => {
                    tracing::error!("mock config error [{}]: {}", issue.rule_id, issue.message);
                }
                np_mock::Severity::Warning => {
                    tracing::warn!("mock config warning [{}]: {}", issue.rule_id, issue.message);
                }
                np_mock::Severity::Info => {
                    tracing::info!("mock config info [{}]: {}", issue.rule_id, issue.message);
                }
            }
        }

        // Fail if there are errors
        let has_errors = issues
            .iter()
            .any(|i| matches!(i.severity, np_mock::Severity::Error));
        if has_errors {
            return Err(DaemonError::Io(std::io::Error::other(
                "mock configuration has errors (see logs above)",
            )));
        }

        self.mock = registry.clone();
        self.mock_registry = Some(registry);
        self.mock_config_path = Some(config_path);

        Ok(self)
    }
}

struct RunState {
    run: String,
    bus: Bus,
    handle: Box<dyn EngineHandle>,
    writer: WriterHandle,
    listen: SocketAddr,
    watcher: Option<MockConfigWatcher>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub state: &'static str, // "stopped" | "running"
    pub run: Option<String>,
    pub listen: Option<String>,
    pub sequenced: u64,
    pub dropped: u64,
}

pub struct Daemon {
    cfg: DaemonConfig,
    engine: Arc<dyn ProxyEngine>,
    pub store: Store,
    ca: CaMaterial,
    mock_registry: Option<Arc<DynamicMockRegistry>>,
    inner: Mutex<Option<RunState>>,
    committed: Arc<watch::Sender<u64>>,
    status: std::sync::Mutex<Status>,
    stats: std::sync::Mutex<Option<Arc<np_engine::bus::BusStats>>>,
    pub breakpoints: Arc<std::sync::Mutex<BreakpointStore>>,
    pub paused_requests: Arc<std::sync::Mutex<PausedRequestStore>>,
}

fn run_id() -> String {
    let mut b = [0u8; 6];
    getrandom::fill(&mut b).expect("os randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Daemon {
    pub fn new(
        cfg: DaemonConfig,
        engine: Arc<dyn ProxyEngine>,
    ) -> Result<Arc<Daemon>, DaemonError> {
        std::fs::create_dir_all(&cfg.data_dir)?;
        let store = Store::open(cfg.data_dir.join("netproxy.db"))?;
        let ca = CaMaterial::load_or_create_with(
            &cfg.data_dir.join("ca"),
            "NetProxy Local CA",
            &cfg.ca_storage,
        )?;
        let mock_registry = cfg.mock_registry.clone();
        Ok(Arc::new(Daemon {
            cfg,
            engine,
            store,
            ca,
            mock_registry,
            inner: Mutex::new(None),
            committed: Arc::new(watch::channel(0u64).0),
            status: std::sync::Mutex::new(Status {
                state: "stopped",
                run: None,
                listen: None,
                sequenced: 0,
                dropped: 0,
            }),
            stats: std::sync::Mutex::new(None),
            breakpoints: Arc::new(std::sync::Mutex::new(BreakpointStore::default())),
            paused_requests: Arc::new(std::sync::Mutex::new(PausedRequestStore::new())),
        }))
    }

    /// Background housekeeping: DB retention while idle (the store writer prunes itself while
    /// capturing) every `every`, and the body-store cap every second so it cannot overshoot far.
    pub fn spawn_maintenance(
        self: &Arc<Self>,
        every: std::time::Duration,
    ) -> tokio::task::JoinHandle<()> {
        let (a, b) = (self.clone(), self.clone());
        tokio::spawn(async move {
            let db = async move {
                loop {
                    tokio::time::sleep(every).await;
                    if let Some(max) = a.cfg.max_db_bytes {
                        if a.status().state == "stopped" {
                            let _ = a.prune_now(max).await;
                        }
                    }
                }
            };
            let bodies = async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    b.enforce_body_cap().await;
                }
            };
            tokio::join!(db, bodies);
        })
    }

    pub async fn prune_now(&self, max_bytes: u64) -> Result<np_store::PruneReport, DaemonError> {
        let store = self.store.clone();
        let active = self.status.lock().unwrap().run.clone();
        let rep = tokio::task::spawn_blocking(move || {
            store.prune(&np_store::PrunePolicy { max_bytes }, active.as_deref())
        })
        .await
        .map_err(|e| DaemonError::Io(std::io::Error::other(e)))??;
        if rep.runs_deleted + rep.events_deleted > 0 {
            tracing::info!(
                runs = rep.runs_deleted,
                events = rep.events_deleted,
                flows = rep.flows_deleted,
                before = rep.bytes_before,
                after = rep.bytes_after,
                "retention pruned"
            );
        }
        Ok(rep)
    }

    pub fn bodies_dir(&self) -> PathBuf {
        self.cfg.data_dir.join("bodies")
    }

    /// Evicts oldest body files over the cap and drops bodies of runs no longer in the database.
    pub async fn enforce_body_cap(&self) {
        let Some(b) = self.cfg.bodies else { return };
        let (dir, store) = (self.bodies_dir(), self.store.clone());
        let active = self.status.lock().unwrap().run.clone(); // its row may not be committed yet
        let _ = tokio::task::spawn_blocking(move || {
            let known: std::collections::HashSet<String> = store
                .reader()
                .and_then(|r| r.runs())
                .map(|v| v.into_iter().map(|x| x.run).collect())
                .unwrap_or_default();
            crate::bodies::enforce_cap(&dir, b.total_cap, &|run| {
                known.contains(run) || active.as_deref() == Some(run)
            })
        })
        .await;
    }

    pub fn max_db_bytes(&self) -> Option<u64> {
        self.cfg.max_db_bytes
    }

    pub fn ca_cert_pem(&self) -> &str {
        &self.ca.cert_pem
    }

    /// Notified after every committed store batch; value is a monotonically increasing counter.
    pub fn commits(&self) -> watch::Receiver<u64> {
        self.committed.subscribe()
    }

    pub fn status(&self) -> Status {
        let mut s = self.status.lock().unwrap().clone();
        if let Some(st) = self.stats.lock().unwrap().as_ref() {
            s.sequenced = st.sequenced.load(Ordering::Relaxed);
            s.dropped = st.dropped.load(Ordering::Relaxed);
        }
        s
    }

    pub async fn start(&self, listen: Option<SocketAddr>) -> Result<Status, DaemonError> {
        let mut g = self.inner.lock().await;
        if g.is_some() {
            return Err(DaemonError::AlreadyRunning);
        }
        let run = run_id();
        let (dtx, drx) = mpsc::channel(8192);
        let commit = self.committed.clone();
        let retention = self
            .cfg
            .max_db_bytes
            .map(|max_bytes| np_store::PrunePolicy { max_bytes });
        let writer = self.store.spawn_writer_retaining(
            drx,
            move || {
                commit.send_modify(|v| *v += 1);
            },
            retention,
        )?;
        let bus = Bus::spawn(run.clone(), BusConfig::default(), Some(dtx));
        let bodies_dir = self.cfg.data_dir.join("bodies");
        let mut ecfg =
            EngineConfig::new(listen.unwrap_or(self.cfg.default_listen), self.ca.clone());
        ecfg.upstream_roots_pem = self.cfg.upstream_roots_pem.clone();

        // Wrap the mock resolver with breakpoint checking
        let breakpoint_resolver = Arc::new(crate::breakpoint_resolver::BreakpointMockResolver::new(
            self.cfg.mock.clone(),
            Arc::new(std::sync::Mutex::new(BreakpointStore::default())), // TODO: share with self.breakpoints
            Arc::new(std::sync::Mutex::new(crate::breakpoint_engine::PausedRequestStore::new())), // TODO: share with self.paused_requests
        ));
        ecfg.mock = Arc::new(crate::setup::DeviceSetup::new(
            &self.ca,
            breakpoint_resolver,
        )?);
        if let Some(b) = self.cfg.bodies {
            ecfg.bodies = Arc::new(crate::bodies::FileBodySink::new(
                &bodies_dir,
                &run,
                b.per_body_cap,
                16_384,
            )?);
        }
        let handle = match self.engine.start(ecfg, bus.sink()).await {
            Ok(h) => h,
            Err(e) => {
                bus.close().await;
                drop(bus);
                let w = writer;
                let _ = tokio::task::spawn_blocking(move || w.join()).await;
                return Err(e.into());
            }
        };
        let addr = handle.local_addr();

        // Start file watcher for hot reload if we have a mock registry
        let watcher = if let (Some(registry), Some(config_path)) = (
            self.mock_registry.as_ref(),
            self.cfg.mock_config_path.as_ref(),
        ) {
            match MockConfigWatcher::watch(config_path.clone(), registry.clone()) {
                Ok(w) => {
                    tracing::info!("mock config watcher started");
                    Some(w)
                }
                Err(e) => {
                    tracing::warn!("failed to start mock config watcher: {}", e);
                    None
                }
            }
        } else {
            None
        };

        *self.stats.lock().unwrap() = Some(bus.stats());
        *self.status.lock().unwrap() = Status {
            state: "running",
            run: Some(run.clone()),
            listen: Some(addr.to_string()),
            sequenced: 0,
            dropped: 0,
        };
        *g = Some(RunState {
            run,
            bus,
            handle,
            writer,
            listen: addr,
            watcher,
        });
        drop(g);
        Ok(self.status())
    }

    pub async fn stop(&self) -> Result<Status, DaemonError> {
        let mut g = self.inner.lock().await;
        let Some(rs) = g.take() else {
            return Err(DaemonError::NotRunning);
        };
        let RunState {
            run,
            bus,
            handle,
            writer,
            listen,
            watcher,
        } = rs;

        // Stop watcher first
        if let Some(w) = watcher {
            w.stop().await;
            tracing::info!("mock config watcher stopped");
        }

        handle.shutdown().await;
        bus.close().await;
        drop(bus);
        let _ = tokio::task::spawn_blocking(move || writer.join()).await;
        *self.status.lock().unwrap() = Status {
            state: "stopped",
            run: None,
            listen: None,
            sequenced: 0,
            dropped: 0,
        };
        let mut out = Status {
            state: "stopped",
            run: Some(run),
            listen: Some(listen.to_string()),
            sequenced: 0,
            dropped: 0,
        };
        if let Some(st) = self.stats.lock().unwrap().take() {
            out.sequenced = st.sequenced.load(Ordering::Relaxed);
            out.dropped = st.dropped.load(Ordering::Relaxed);
        }
        Ok(out)
    }

    /// Get the mock registry if configured.
    pub fn mock_registry(&self) -> Option<Arc<DynamicMockRegistry>> {
        self.mock_registry.clone()
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &std::path::Path {
        &self.cfg.data_dir
    }

    /// Get the mock config path if configured.
    pub fn mock_config_path(&self) -> Option<&std::path::Path> {
        self.cfg.mock_config_path.as_deref()
    }

    /// Latest run id known to the store (current run if running).
    pub fn current_or_latest_run(&self) -> Option<String> {
        if let Some(r) = self.status.lock().unwrap().run.clone() {
            return Some(r);
        }
        self.store.reader().ok()?.latest_run().ok().flatten()
    }
}
