use netproxyd::{
    control,
    daemon::{Daemon, DaemonConfig},
};
use np_engine::HudsuckerEngine;
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

fn arg(k: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == k)
        .and_then(|i| a.get(i + 1))
        .cloned()
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Application Support/NetProxy")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(np_engine::tracing_layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new("warn,netproxyd=info")),
                ),
        )
        .init();

    let data_dir = arg("--data-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_data_dir);
    let mut cfg = DaemonConfig::new(data_dir.clone());

    // Load mock rules from mocks.yaml if it exists
    let mock_config = data_dir.join("mocks.yaml");
    if mock_config.exists() {
        cfg = cfg.with_mock_config(mock_config).expect("load mocks.yaml");
        tracing::info!("loaded mock rules from mocks.yaml");
    }

    if let Some(l) = arg("--listen") {
        cfg.default_listen = l.parse().expect("--listen ip:port");
    }
    // Keychain by default on macOS; `--ca-in-file` keeps the key in a 0600 file (headless/CI).
    if cfg!(target_os = "macos") && !std::env::args().any(|a| a == "--ca-in-file") {
        cfg.ca_storage = np_engine::KeyStorage::Keychain {
            service: "dev.netproxy.ca-key".into(),
        };
    }
    if let Some(mb) = arg("--max-db-mb") {
        cfg.max_db_bytes = match mb.parse::<u64>().expect("--max-db-mb N") {
            0 => None,
            n => Some(n << 20),
        };
    }
    if let Some(p) = arg("--upstream-ca") {
        cfg.upstream_roots_pem = Some(std::fs::read(p).expect("read --upstream-ca"));
    }
    let daemon = Daemon::new(cfg, Arc::new(HudsuckerEngine)).expect("init daemon");
    let _maintenance = daemon.spawn_maintenance(std::time::Duration::from_secs(5));
    let ctl = match control::serve(daemon.clone(), &data_dir.join("run")).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("{}", control::ALREADY_RUNNING);
            std::process::exit(75); // EX_TEMPFAIL: supervisors attach to the running daemon instead
        }
        Err(e) => panic!("control socket: {e}"),
    };
    eprintln!(
        "netproxyd ready\n  control socket: {}\n  endpoint file:  {}\n  token file:     {}",
        ctl.info.socket.display(),
        ctl.info.endpoint_file.display(),
        ctl.info.token_file.display()
    );
    if std::env::args().any(|a| a == "--autostart") {
        match daemon.start(None).await {
            Ok(s) => eprintln!("  capturing on {}", s.listen.unwrap_or_default()),
            Err(e) => eprintln!("  autostart failed: {e}"),
        }
    }
    // Exit on Ctrl-C, SIGTERM, or when the supervising app disappears (so a crashed UI can never
    // leave a proxy running with nobody watching it).
    let parent: Option<i32> = arg("--parent-pid").and_then(|p| p.parse().ok());
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler");
    let parent_gone = async {
        match parent {
            Some(pid) => loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if unsafe { libc::kill(pid, 0) } != 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    break;
                }
            },
            None => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
        _ = parent_gone => eprintln!("parent process exited"),
    }
    eprintln!("shutting down");
    let _ = daemon.stop().await;
    ctl.stop().await;
}
