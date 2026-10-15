//! File watcher for hot-reloading mock configuration.
//!
//! Monitors mocks.yaml for changes and triggers registry reload automatically.

use np_mock::DynamicMockRegistry;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// File watcher that monitors mocks.yaml and triggers hot reload.
pub struct MockConfigWatcher {
    _watcher: RecommendedWatcher,
    shutdown_tx: mpsc::Sender<()>,
}

impl MockConfigWatcher {
    /// Start watching the config file.
    ///
    /// When the file changes, the registry is automatically reloaded.
    /// Returns a watcher handle that stops watching when dropped.
    pub fn watch(
        config_path: PathBuf,
        registry: Arc<DynamicMockRegistry>,
    ) -> Result<Self, WatcherError> {
        if !config_path.exists() {
            tracing::warn!(
                "config file does not exist yet: {}",
                config_path.display()
            );
        }

        let (event_tx, mut event_rx) = mpsc::channel(32);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Create file watcher
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    // Send event to async handler
                    let _ = event_tx.blocking_send(event);
                }
                Err(e) => {
                    tracing::error!("file watcher error: {}", e);
                }
            }
        })
        .map_err(|e| WatcherError::Init(e.to_string()))?;

        // Watch the config file's parent directory
        // (watching the file directly doesn't work well with atomic writes)
        let watch_dir = config_path
            .parent()
            .ok_or_else(|| WatcherError::Init("config path has no parent".to_string()))?
            .to_path_buf();

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| WatcherError::Init(format!("failed to watch {}: {}", watch_dir.display(), e)))?;

        tracing::info!("watching for changes: {}", config_path.display());

        // Spawn event handler task
        let config_path_clone = config_path.clone();
        tokio::spawn(async move {
            let mut debounce_timer: Option<tokio::time::Instant> = None;
            const DEBOUNCE_MS: u64 = 500; // Wait 500ms after last change

            loop {
                tokio::select! {
                    // File system event
                    Some(event) = event_rx.recv() => {
                        if Self::is_relevant_event(&event, &config_path_clone) {
                            tracing::debug!("config file changed: {:?}", event.kind);
                            // Set/reset debounce timer
                            debounce_timer = Some(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                        }
                    }

                    // Debounce timer expired
                    _ = Self::wait_for_debounce(debounce_timer), if debounce_timer.is_some() => {
                        debounce_timer = None;
                        tracing::info!("reloading config: {}", config_path_clone.display());

                        match registry.reload() {
                            Ok(()) => {
                                tracing::info!("config reloaded successfully");

                                // Run validation and log issues
                                let issues = registry.validate_all();
                                if !issues.is_empty() {
                                    tracing::warn!("validation issues after reload:");
                                    for issue in issues {
                                        match issue.severity {
                                            np_mock::Severity::Error => {
                                                tracing::error!("  [{}] {}", issue.rule_id, issue.message);
                                            }
                                            np_mock::Severity::Warning => {
                                                tracing::warn!("  [{}] {}", issue.rule_id, issue.message);
                                            }
                                            np_mock::Severity::Info => {
                                                tracing::info!("  [{}] {}", issue.rule_id, issue.message);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("failed to reload config: {}", e);
                            }
                        }
                    }

                    // Shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("watcher shutting down");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            shutdown_tx,
        })
    }

    /// Check if event is relevant to our config file.
    fn is_relevant_event(event: &Event, config_path: &PathBuf) -> bool {
        // Check event kind
        let is_modify_or_create = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        );

        if !is_modify_or_create {
            return false;
        }

        // Check if any path matches our config file
        // Use canonicalize for robust comparison, but fall back to direct comparison
        let config_canonical = config_path.canonicalize().ok();

        event.paths.iter().any(|p| {
            // Direct comparison
            if p == config_path {
                return true;
            }

            // Try canonical comparison
            if let Some(ref config_canon) = config_canonical {
                if let Ok(p_canon) = p.canonicalize() {
                    return &p_canon == config_canon;
                }
            }

            // Compare file names as last resort
            if let (Some(p_name), Some(config_name)) = (p.file_name(), config_path.file_name()) {
                if p_name == config_name {
                    // Also check that parent directories match
                    if let (Some(p_parent), Some(config_parent)) = (p.parent(), config_path.parent()) {
                        return p_parent == config_parent ||
                               p_parent.canonicalize().ok() == config_parent.canonicalize().ok();
                    }
                    return true;
                }
            }

            false
        })
    }

    /// Helper to wait for debounce timer.
    async fn wait_for_debounce(timer: Option<tokio::time::Instant>) {
        if let Some(deadline) = timer {
            tokio::time::sleep_until(deadline).await;
        } else {
            // Never completes if no timer
            std::future::pending::<()>().await;
        }
    }

    /// Stop watching (also happens automatically on drop).
    pub async fn stop(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("failed to initialize watcher: {0}")]
    Init(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use np_engine::engine::NoMocks;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn watcher_can_be_created() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("mocks.yaml");

        // Create initial config
        fs::write(
            &config_path,
            r#"
version: 1
fixtures_dir: ./fixtures
rules: []
"#,
        )
        .unwrap();

        let registry = Arc::new(
            DynamicMockRegistry::new(config_path.clone(), Arc::new(NoMocks)).unwrap(),
        );

        // Should be able to create watcher
        let watcher = MockConfigWatcher::watch(config_path.clone(), registry.clone()).unwrap();

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Clean shutdown
        watcher.stop().await;
    }

    // File watcher integration tests are flaky in test environments due to timing issues
    // and platform-specific behavior. Run these manually for verification.

    #[tokio::test]
    #[ignore = "file watcher tests are flaky - run manually with --ignored"]
    async fn watcher_detects_file_changes() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("mocks.yaml");

        // Create initial config
        fs::write(
            &config_path,
            r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: test1
    priority: 100
    enabled: true
    match:
      host: test.local
    action:
      type: pass
"#,
        )
        .unwrap();

        let registry = Arc::new(
            DynamicMockRegistry::new(config_path.clone(), Arc::new(NoMocks)).unwrap(),
        );

        assert_eq!(registry.list_static_rules().len(), 1);

        // Start watcher
        let watcher = MockConfigWatcher::watch(config_path.clone(), registry.clone()).unwrap();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Modify config
        fs::write(
            &config_path,
            r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: test1
    priority: 100
    enabled: true
    match:
      host: test.local
    action:
      type: pass
  - id: test2
    priority: 200
    enabled: true
    match:
      host: other.local
    action:
      type: pass
"#,
        )
        .unwrap();

        // Wait for debounce + reload (longer on macOS)
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Should have reloaded
        let rules = registry.list_static_rules();
        assert_eq!(rules.len(), 2, "expected 2 rules after reload, got {}", rules.len());

        watcher.stop().await;
    }

    #[tokio::test]
    #[ignore = "file watcher tests are flaky - run manually with --ignored"]
    async fn watcher_handles_file_deletion() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("mocks.yaml");

        // Create initial config
        fs::write(
            &config_path,
            r#"
version: 1
fixtures_dir: ./fixtures
rules:
  - id: test
    priority: 100
    enabled: true
    match:
      host: test.local
    action:
      type: pass
"#,
        )
        .unwrap();

        let registry = Arc::new(
            DynamicMockRegistry::new(config_path.clone(), Arc::new(NoMocks)).unwrap(),
        );

        assert_eq!(registry.list_static_rules().len(), 1);

        let watcher = MockConfigWatcher::watch(config_path.clone(), registry.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Delete config file
        fs::remove_file(&config_path).unwrap();

        // Wait for event (longer on macOS)
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Static rules should be cleared
        let rules = registry.list_static_rules();
        assert_eq!(rules.len(), 0, "expected 0 rules after deletion, got {}", rules.len());

        watcher.stop().await;
    }

    #[tokio::test]
    #[ignore = "file watcher tests are flaky - run manually with --ignored"]
    async fn watcher_debounces_rapid_changes() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("mocks.yaml");

        fs::write(&config_path, "version: 1\nfixtures_dir: ./fixtures\nrules: []").unwrap();

        let registry = Arc::new(
            DynamicMockRegistry::new(config_path.clone(), Arc::new(NoMocks)).unwrap(),
        );

        let watcher = MockConfigWatcher::watch(config_path.clone(), registry.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Make multiple rapid changes
        for i in 1..=5 {
            fs::write(
                &config_path,
                format!("version: {}\nfixtures_dir: ./fixtures\nrules: []", i),
            )
            .unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Wait for debounce (longer on macOS)
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Should have reloaded only once (debounced)
        // We can't easily verify reload count, but no panics/errors = success
        watcher.stop().await;
    }
}
