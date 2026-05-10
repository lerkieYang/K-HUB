pub mod debouncer;

use crate::config::{Config, SourceConfig};
use crate::queue::{Queue, PendingEvent};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::path::Path;
use tokio::sync::mpsc;

pub struct FileWatcher {
    config: Config,
    queue: Queue,
}

impl FileWatcher {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let queue = Queue::new(&config.data_dir).await?;
        Ok(Self { config, queue })
    }
    
    pub async fn start(&self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let queue = self.queue.clone();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;
        
        // 监听所有配置的源目录
        for source in &self.config.sources {
            if source.scan_mode == "watch" || source.scan_mode == "watch_and_scan" {
                let path = Path::new(&source.path);
                if path.exists() {
                    let mode = if source.recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    watcher.watch(path, mode)?;
                    tracing::info!("Watching: {}", source.path);
                }
            }
        }
        
        // 处理文件事件
        while let Some(event) = rx.recv().await {
            self.handle_event(event).await;
        }
        
        Ok(())
    }
    
    async fn handle_event(&self, event: Event) {
        use notify::EventKind;
        
        let event_type = match event.kind {
            EventKind::Create(_) => "file_created",
            EventKind::Modify(_) => "file_modified",
            EventKind::Remove(_) => "file_deleted",
            _ => return,
        };
        
        for path in event.paths {
            if self.should_process(&path) {
                tracing::info!("{}: {:?}", event_type, path);
                
                // 写入本地队列
                let pending_event = PendingEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    event_type: event_type.to_string(),
                    path: path.to_string_lossy().to_string(),
                    source_id: self.find_source_id(&path),
                    device_id: Some(self.config.device_id.clone()),
                    size_bytes: std::fs::metadata(&path).ok().map(|m| m.len() as i64),
                    mtime: std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                    sha256: None, // 可以后台计算
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                
                if let Err(e) = self.queue.push(pending_event).await {
                    tracing::error!("Failed to push to queue: {}", e);
                }
            }
        }
    }
    
    fn should_process(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // 过滤临时文件
        if path_str.contains("~$") || path_str.ends_with(".tmp") {
            return false;
        }
        
        // 检查是否匹配任何源的包含规则
        for source in &self.config.sources {
            if path_str.starts_with(&source.path) {
                // 检查排除规则
                for exclude in &source.exclude_globs {
                    if path_str.contains(exclude) {
                        return false;
                    }
                }
                
                // 检查包含规则
                if source.include_globs.is_empty() {
                    return true;
                }
                
                for include in &source.include_globs {
                    if path_str.ends_with(&include.replace("*", "")) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    fn find_source_id(&self, path: &Path) -> Option<String> {
        let path_str = path.to_string_lossy();
        self.config.sources.iter()
            .find(|s| path_str.starts_with(&s.path))
            .map(|s| s.id.clone())
    }
}
