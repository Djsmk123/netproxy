//! Engine lifecycle owner: wires CA, store writer, bus and engine into numbered capture runs.

use np_engine::{Bus, BusConfig, CaMaterial, EngineConfig, KeyStorage, EngineError, EngineHandle, MockResolver, NoMocks, ProxyEngine};
use np_store::{Store, StoreError, WriterHandle};
use serde::Serialize;
use std::{net::SocketAddr, path::PathBuf, sync::{atomic::Ordering, Arc}};
use tokio::sync::{mpsc, watch, Mutex};

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
}

pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub default_listen: SocketAddr,
    pub upstream_roots_pem: Option<Vec<u8>>,
    pub mock: Arc<dyn MockResolver>,
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
        Self { data_dir, default_listen: "127.0.0.1:8899".parse().unwrap(), upstream_roots_pem: None, mock: Arc::new(NoMocks), max_db_bytes: Some(2 << 30), bodies: Some(BodyConfig { per_body_cap: 4 << 20, total_cap: 1 << 30 }), ca_storage: KeyStorage::File }
    }

    /// Load mock rules from a YAML config file and wrap the existing mock resolver.
    pub fn with_mock_config(mut self, config_path: PathBuf) -> Result<Self, DaemonError> {
        let yaml = std::fs::read_to_string(&config_path)?;
        let mock_config: np_mock::MockConfig = serde_yaml::from_str(&yaml)
            .map_err(|e| DaemonError::Io(std::io::Error::other(format!("invalid mocks.yaml: {}", e))))?;

        // Resolve fixtures_dir relative to the config file location
        let config_dir = config_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let mut mock_config = mock_config;
        if !mock_config.fixtures_dir.is_absolute() {
            mock_config.fixtures_dir = config_dir.join(&mock_config.fixtures_dir);
        }

        self.mock = Arc::new(np_mock::RuleMockResolver::new(mock_config, self.mock.clone())
            .map_err(|e| DaemonError::Io(std::io::Error::other(format!("mock config error: {}", e))))?);

        Ok(self)
    }
}

struct RunState {
    run: String,
    bus: Bus,
    handle: Box<dyn EngineHandle>,
    writer: WriterHandle,
    listen: SocketAddr,
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
    inner: Mutex<Option<RunState>>,
    committed: Arc<watch::Sender<u64>>,
    status: std::sync::Mutex<Status>,
    stats: std::sync::Mutex<Option<Arc<np_engine::bus::BusStats>>>,
}

fn run_id() -> String {
    let mut b = [0u8; 6];
    getrandom::fill(&mut b).expect("os randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Daemon {
    pub fn new(cfg: DaemonConfig, engine: Arc<dyn ProxyEngine>) -> Result<Arc<Daemon>, DaemonError> {
        std::fs::create_dir_all(&cfg.data_dir)?;
        let store = Store::open(cfg.data_dir.join("netproxy.db"))?;
        let ca = CaMaterial::load_or_create_with(&cfg.data_dir.join("ca"), "NetProxy Local CA", &cfg.ca_storage)?;
        Ok(Arc::new(Daemon {
            cfg, engine, store, ca,
            inner: Mutex::new(None),
            committed: Arc::new(watch::channel(0u64).0),
            status: std::sync::Mutex::new(Status { state: "stopped", run: None, listen: None, sequenced: 0, dropped: 0 }),
            stats: std::sync::Mutex::new(None),
        }))
    }

    /// Background housekeeping: DB retention while idle (the store writer prunes itself while
    /// capturing) every `every`, and the body-store cap every second so it cannot overshoot far.
    pub fn spawn_maintenance(self: &Arc<Self>, every: std::time::Duration) -> tokio::task::JoinHandle<()> {
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
        let rep = tokio::task::spawn_blocking(move || store.prune(&np_store::PrunePolicy { max_bytes }, active.as_deref()))
            .await
            .map_err(|e| DaemonError::Io(std::io::Error::other(e)))??;
        if rep.runs_deleted + rep.events_deleted > 0 {
            tracing::info!(runs = rep.runs_deleted, events = rep.events_deleted, flows = rep.flows_deleted, before = rep.bytes_before, after = rep.bytes_after, "retention pruned");
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
            let known: std::collections::HashSet<String> = store.reader().and_then(|r| r.runs()).map(|v| v.into_iter().map(|x| x.run).collect()).unwrap_or_default();
            crate::bodies::enforce_cap(&dir, b.total_cap, &|run| known.contains(run) || active.as_deref() == Some(run))
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
        let retention = self.cfg.max_db_bytes.map(|max_bytes| np_store::PrunePolicy { max_bytes });
        let writer = self.store.spawn_writer_retaining(drx, move || {
            commit.send_modify(|v| *v += 1);
        }, retention)?;
        let bus = Bus::spawn(run.clone(), BusConfig::default(), Some(dtx));
        let bodies_dir = self.cfg.data_dir.join("bodies");
        let mut ecfg = EngineConfig::new(listen.unwrap_or(self.cfg.default_listen), self.ca.clone());
        ecfg.upstream_roots_pem = self.cfg.upstream_roots_pem.clone();
        ecfg.mock = Arc::new(crate::setup::DeviceSetup::new(&self.ca, self.cfg.mock.clone())?);
        if let Some(b) = self.cfg.bodies {
            ecfg.bodies = Arc::new(crate::bodies::FileBodySink::new(&bodies_dir, &run, b.per_body_cap, 16_384)?);
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
        *self.stats.lock().unwrap() = Some(bus.stats());
        *self.status.lock().unwrap() = Status { state: "running", run: Some(run.clone()), listen: Some(addr.to_string()), sequenced: 0, dropped: 0 };
        *g = Some(RunState { run, bus, handle, writer, listen: addr });
        drop(g);
        Ok(self.status())
    }

    pub async fn stop(&self) -> Result<Status, DaemonError> {
        let mut g = self.inner.lock().await;
        let Some(rs) = g.take() else { return Err(DaemonError::NotRunning) };
        let RunState { run, bus, handle, writer, listen } = rs;
        handle.shutdown().await;
        bus.close().await;
        drop(bus);
        let _ = tokio::task::spawn_blocking(move || writer.join()).await;
        *self.status.lock().unwrap() = Status { state: "stopped", run: None, listen: None, sequenced: 0, dropped: 0 };
        let mut out = Status { state: "stopped", run: Some(run), listen: Some(listen.to_string()), sequenced: 0, dropped: 0 };
        if let Some(st) = self.stats.lock().unwrap().take() {
            out.sequenced = st.sequenced.load(Ordering::Relaxed);
            out.dropped = st.dropped.load(Ordering::Relaxed);
        }
        Ok(out)
    }

    /// Latest run id known to the store (current run if running).
    pub fn current_or_latest_run(&self) -> Option<String> {
        if let Some(r) = self.status.lock().unwrap().run.clone() {
            return Some(r);
        }
        self.store.reader().ok()?.latest_run().ok().flatten()
    }
}
