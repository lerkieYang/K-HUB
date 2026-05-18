use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::db::Database;

/// Supported file extensions for watching (same as KnowledgeIndexer)
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "md", "txt", "markdown", "rst", "asciidoc", "adoc",
    "json", "yaml", "yml", "toml", "csv", "tsv", "xml",
    "docx", "doc", "xlsx", "xls", "pptx", "ppt",
    "odt", "ods", "odp", "pdf",
    "html", "htm", "mhtml", "mht", "epub", "rtf",
    "ini", "cfg", "conf", "properties", "env",
    "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
    "log", "sql", "graphql", "proto",
];

/// Debounce window in seconds
const DEBOUNCE_SECS: u64 = 5;

/// Cooldown after indexing completes (seconds) — prevents file watcher from
/// immediately re-triggering indexing due to background file events.
const POST_INDEX_COOLDOWN_SECS: u64 = 120;

/// Global timestamp (epoch seconds) of the last indexing completion.
/// Set by `run_indexing_task` when it finishes; checked by file watcher.
static LAST_INDEXING_COMPLETED: AtomicU64 = AtomicU64::new(0);

/// Call this from the indexing task when it completes.
pub fn notify_indexing_completed() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_INDEXING_COMPLETED.store(now, Ordering::Relaxed);
    tracing::info!("File watcher: indexing completion recorded, cooldown {}s", POST_INDEX_COOLDOWN_SECS);
}

/// Check if we're still within the post-indexing cooldown window.
fn is_in_cooldown() -> bool {
    let completed = LAST_INDEXING_COMPLETED.load(Ordering::Relaxed);
    if completed == 0 { return false; }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(completed) < POST_INDEX_COOLDOWN_SECS
}

/// Simplified file event for cross-thread communication
#[derive(Debug, Clone)]
struct FileEvent {
    kind: EventKind,
    paths: Vec<PathBuf>,
}

/// Cached config info for quick path-to-config lookup
#[derive(Debug, Clone)]
struct ConfigInfo {
    id: String,
    name: String,
    paths: Vec<String>,
    root_paths: Vec<PathBuf>,
}

/// File system watcher that monitors knowledge base directories
/// and triggers automatic reindexing when files change.
pub struct FileWatcher {
    db: Database,
    watcher: Option<RecommendedWatcher>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    processor_handle: Option<tokio::task::JoinHandle<()>>,
    watched_dirs: Arc<Mutex<HashSet<PathBuf>>>,
    configs: Arc<Mutex<Vec<ConfigInfo>>>,
}

impl FileWatcher {
    /// Create a new FileWatcher instance (does not start watching yet)
    pub fn new(db: Database) -> Self {
        Self {
            db,
            watcher: None,
            shutdown_tx: None,
            processor_handle: None,
            watched_dirs: Arc::new(Mutex::new(HashSet::new())),
            configs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start the file watcher service.
    /// Reads knowledge_base_config from DB and begins watching all configured paths.
    pub async fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting file watcher service...");

        let (event_tx, event_rx) = mpsc::unbounded_channel::<FileEvent>();
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        // Load initial configs from DB
        let config_infos = self.load_configs().await;
        if config_infos.is_empty() {
            tracing::info!("No knowledge base configs with auto_index=1, file watcher idle");
        }
        *self.configs.lock().await = config_infos.clone();

        // Collect directories to watch
        let mut dirs_to_watch = HashSet::new();
        for config in &config_infos {
            for path_str in &config.paths {
                let path = PathBuf::from(path_str);
                if path.exists() && path.is_dir() {
                    dirs_to_watch.insert(path);
                } else if !path.exists() {
                    tracing::warn!("Config '{}' path does not exist: {}", config.name, path_str);
                }
            }
        }

        // Create the notify watcher with event forwarding
        let tx_clone = event_tx.clone();
        let mut watcher = notify::recommended_watcher(
            move |event: Result<Event, notify::Error>| {
                match event {
                    Ok(event) => {
                        let file_event = FileEvent {
                            kind: event.kind,
                            paths: event.paths,
                        };
                        let _ = tx_clone.send(file_event);
                    }
                    Err(e) => {
                        tracing::warn!("File watcher error: {}", e);
                    }
                }
            },
        )?;

        // Start watching all directories recursively
        for dir in &dirs_to_watch {
            if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                tracing::warn!("Failed to watch directory {}: {}", dir.display(), e);
            } else {
                tracing::info!("Watching directory: {}", dir.display());
            }
        }

        *self.watched_dirs.lock().await = dirs_to_watch;

        // Spawn the event processing task
        let configs = self.configs.clone();
        let db_clone = self.db.clone();
        let processor_handle = tokio::spawn(Self::process_events(
            event_rx,
            shutdown_rx,
            configs,
            db_clone,
        ));

        self.watcher = Some(watcher);
        self.shutdown_tx = Some(shutdown_tx);
        self.processor_handle = Some(processor_handle);

        tracing::info!("File watcher service started successfully");
        Ok(())
    }

    /// Stop the file watcher service gracefully.
    #[allow(dead_code)]
    pub async fn stop(&mut self) {
        tracing::info!("Stopping file watcher service...");

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Drop the notify watcher (stops all watches)
        self.watcher.take();

        // Wait for the processor task to finish
        if let Some(handle) = self.processor_handle.take() {
            let _ = handle.await;
        }

        tracing::info!("File watcher service stopped");
    }

    /// Refresh watched directories based on current DB configs.
    /// Call this when configs are added/removed/updated.
    #[allow(dead_code)]
    pub async fn refresh(&mut self) -> anyhow::Result<()> {
        tracing::info!("Refreshing file watcher configuration...");

        let new_configs = self.load_configs().await;
        let mut current_dirs = self.watched_dirs.lock().await;
        let mut new_dirs = HashSet::new();

        for config in &new_configs {
            for path_str in &config.paths {
                let path = PathBuf::from(path_str);
                if path.exists() && path.is_dir() {
                    new_dirs.insert(path);
                }
            }
        }

        if let Some(ref mut watcher) = self.watcher {
            // Unwatch directories that are no longer in any config
            for dir in current_dirs.difference(&new_dirs) {
                match watcher.unwatch(dir) {
                    Ok(_) => tracing::info!("Stopped watching: {}", dir.display()),
                    Err(e) => tracing::warn!("Failed to unwatch {}: {}", dir.display(), e),
                }
            }

            // Watch new directories that weren't being watched before
            for dir in new_dirs.difference(&*current_dirs) {
                match watcher.watch(dir, RecursiveMode::Recursive) {
                    Ok(_) => tracing::info!("Now watching: {}", dir.display()),
                    Err(e) => tracing::warn!("Failed to watch {}: {}", dir.display(), e),
                }
            }
        }

        *current_dirs = new_dirs;
        *self.configs.lock().await = new_configs;

        tracing::info!("File watcher configuration refreshed");
        Ok(())
    }

    // ---- Private helpers ----

    /// Load knowledge base configs from the database (only auto_index enabled ones)
    async fn load_configs(&self) -> Vec<ConfigInfo> {
        let rows: Vec<(String, String, String, i64)> = if self.db.is_sqlite {
            sqlx::query_as(
                "SELECT id, name, paths, auto_index FROM knowledge_base_config WHERE auto_index = 1",
            )
            .fetch_all(&self.db.pool)
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as(
                "SELECT id, name, paths, auto_index FROM knowledge_base_config WHERE auto_index = 1",
            )
            .fetch_all(&self.db.pool)
            .await
            .unwrap_or_default()
        };

        rows.into_iter()
            .map(|(id, name, paths_json, _auto_index)| {
                let paths: Vec<String> =
                    serde_json::from_str(&paths_json).unwrap_or_default();
                let root_paths: Vec<PathBuf> = paths
                    .iter()
                    .map(|p| {
                        PathBuf::from(p)
                            .canonicalize()
                            .unwrap_or_else(|_| PathBuf::from(p))
                    })
                    .collect();
                ConfigInfo {
                    id,
                    name,
                    paths,
                    root_paths,
                }
            })
            .collect()
    }

    /// Main event processing loop — runs as a spawned tokio task
    async fn process_events(
        mut event_rx: mpsc::UnboundedReceiver<FileEvent>,
        mut shutdown_rx: mpsc::Receiver<()>,
        configs: Arc<Mutex<Vec<ConfigInfo>>>,
        db: Database,
    ) {
        let mut last_reindex: HashMap<String, Instant> = HashMap::new();
        let debounce = Duration::from_secs(DEBOUNCE_SECS);

        loop {
            tokio::select! {
                // Process incoming file events
                Some(event) = event_rx.recv() => {
                    Self::handle_event(
                        event,
                        &configs,
                        &db,
                        &mut last_reindex,
                        debounce,
                    ).await;
                }
                // Graceful shutdown
                _ = shutdown_rx.recv() => {
                    tracing::info!("File watcher event processor shutting down");
                    break;
                }
            }
        }
    }

    /// Handle a single file event: check extension, find owning config, debounce, reindex
    async fn handle_event(
        event: FileEvent,
        configs: &Arc<Mutex<Vec<ConfigInfo>>>,
        db: &Database,
        last_reindex: &mut HashMap<String, Instant>,
        debounce: Duration,
    ) {
        // Only react to file creation, modification, and removal
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
            _ => return,
        }

        for path in &event.paths {
            // Skip unsupported file types
            if !Self::is_supported_file(path) {
                continue;
            }

            // Find which config owns this file
            let configs_guard = configs.lock().await;
            let owning_config = Self::find_owning_config(path, &configs_guard);
            drop(configs_guard);

            let config = match owning_config {
                Some(c) => c,
                None => continue,
            };

            // Check debounce — don't reindex the same config within the debounce window
            if let Some(last) = last_reindex.get(&config.id) {
                if last.elapsed() < debounce {
                    tracing::debug!(
                        "Debounced: file '{}' change for config '{}' (within {}s window)",
                        path.display(),
                        config.name,
                        DEBOUNCE_SECS
                    );
                    continue;
                }
            }

            // Post-indexing cooldown — skip events right after indexing completes
            if is_in_cooldown() {
                tracing::debug!(
                    "Post-index cooldown active ({}s), skipping file change: {}",
                    POST_INDEX_COOLDOWN_SECS,
                    path.display()
                );
                continue;
            }

            // Queue reindex for the affected config
            tracing::info!(
                "File change detected: {} -> scheduling reindex for config '{}'",
                path.display(),
                config.name
            );
            last_reindex.insert(config.id.clone(), Instant::now());

            crate::api::knowledge::spawn_single_config_indexing(
                db,
                config.id.clone(),
                config.name.clone(),
                config.paths.clone(),
            )
            .await;
        }
    }

    /// Check if a file has a supported extension (same logic as KnowledgeIndexer)
    fn is_supported_file(path: &Path) -> bool {
        // Skip hidden files and directories
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                return false;
            }
            // Skip agent memory lock files
            let lower = name_str.to_lowercase();
            if matches!(
                lower.as_str(),
                "memory.md" | "user.md" | "memory.md.lock" | "user.md.lock"
            ) {
                return false;
            }
        }

        match path.extension() {
            Some(ext) => {
                let ext = ext.to_string_lossy().to_lowercase();
                SUPPORTED_EXTENSIONS.contains(&ext.as_str())
            }
            None => false,
        }
    }

    /// Find which config owns the given file path by checking root path prefixes
    fn find_owning_config(path: &Path, configs: &[ConfigInfo]) -> Option<ConfigInfo> {
        // Try canonicalized path first for accurate matching
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        for config in configs {
            for root in &config.root_paths {
                if canonical.starts_with(root) {
                    return Some(config.clone());
                }
            }
        }

        // Fallback: try raw path matching (handles cases where canonicalize fails)
        for config in configs {
            for path_str in &config.paths {
                if path.starts_with(path_str) {
                    return Some(config.clone());
                }
            }
        }

        None
    }
}
