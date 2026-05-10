pub mod memory_export;

use crate::config::Config;
use crate::queue::{Queue, PendingEvent};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::path::Path;
use tokio::sync::mpsc;

pub struct AgentWatcher {
    config: Config,
    queue: Queue,
}

impl AgentWatcher {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let queue = Queue::new(&config.data_dir).await?;
        Ok(Self { config, queue })
    }
    
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting agent watcher");
        
        // 获取已配置的Agent输出目录
        let config_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".knowledge-hub");
        let outputs_dir = config_dir.join("outputs");
        
        if !outputs_dir.exists() {
            std::fs::create_dir_all(&outputs_dir)?;
        }
        
        // 使用notify监听Agent输出目录
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
        
        // 监听所有Agent输出目录
        if let Ok(entries) = std::fs::read_dir(&outputs_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    watcher.watch(&entry.path(), RecursiveMode::Recursive)?;
                    tracing::info!("Watching agent output: {:?}", entry.path());
                }
            }
        }
        
        // 处理文件事件
        while let Some(event) = rx.recv().await {
            self.handle_agent_event(event).await;
        }
        
        Ok(())
    }
    
    async fn handle_agent_event(&self, event: Event) {
        use notify::EventKind;
        
        if !matches!(event.kind, EventKind::Create(_)) {
            return;
        }
        
        for path in event.paths {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            // 从路径提取agent_id
            let agent_id = self.extract_agent_id(&path);
            
            if file_name == "memory-export.json" {
                tracing::info!("Found memory-export.json from {:?}", agent_id);
                self.process_memory_export(&agent_id, &path).await;
            } else if file_name.ends_with(".md") || file_name.ends_with(".txt") {
                tracing::info!("New artifact from {:?}: {}", agent_id, file_name);
                self.process_artifact(&agent_id, &path).await;
            }
        }
    }
    
    fn extract_agent_id(&self, path: &Path) -> String {
        // 从路径提取agent_id，例如 ~/.knowledge-hub/outputs/hermes/file.txt -> hermes
        let config_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".knowledge-hub");
        let outputs_dir = config_dir.join("outputs");
        
        if let Ok(relative) = path.strip_prefix(&outputs_dir) {
            if let Some(first_component) = relative.components().next() {
                return first_component.as_os_str().to_string_lossy().to_string();
            }
        }
        
        "unknown".to_string()
    }
    
    async fn process_memory_export(&self, agent_id: &str, path: &Path) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to read memory-export.json: {}", e);
                return;
            }
        };
        
        let export: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Failed to parse memory-export.json: {}", e);
                return;
            }
        };
        
        // 提取memories
        if let Some(memories) = export.get("memories").and_then(|m| m.as_array()) {
            for memory in memories {
                let pending_event = PendingEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    event_type: "memory_candidate".to_string(),
                    path: path.to_string_lossy().to_string(),
                    source_id: Some(agent_id.to_string()),
                    device_id: Some(self.config.device_id.clone()),
                    size_bytes: None,
                    mtime: None,
                    sha256: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                
                if let Err(e) = self.queue.push(pending_event).await {
                    tracing::error!("Failed to push memory event: {}", e);
                }
            }
        }
        
        // 移动到已处理目录
        let processed_dir = path.parent().unwrap().join("processed");
        std::fs::create_dir_all(&processed_dir).unwrap_or_default();
        let new_path = processed_dir.join(path.file_name().unwrap());
        std::fs::rename(path, new_path).unwrap_or_default();
    }
    
    async fn process_artifact(&self, agent_id: &str, path: &Path) {
        let pending_event = PendingEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: "artifact".to_string(),
            path: path.to_string_lossy().to_string(),
            source_id: Some(agent_id.to_string()),
            device_id: Some(self.config.device_id.clone()),
            size_bytes: std::fs::metadata(path).ok().map(|m| m.len() as i64),
            mtime: std::fs::metadata(path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
            sha256: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        
        if let Err(e) = self.queue.push(pending_event).await {
            tracing::error!("Failed to push artifact event: {}", e);
        }
    }
}
