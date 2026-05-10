pub mod retry;

use crate::config::Config;
use crate::queue::{Queue, PendingEvent};
use reqwest::Client;
use retry::RetryPolicy;

pub struct Uploader {
    config: Config,
    client: Client,
    queue: Queue,
    retry_policy: RetryPolicy,
}

impl Uploader {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let client = Client::new();
        let queue = Queue::new(&config.data_dir).await?;
        let retry_policy = RetryPolicy::default();
        
        Ok(Self { config, client, queue, retry_policy })
    }
    
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting uploader");
        
        loop {
            // 获取待上传的事件
            let pending = self.queue.get_pending(10).await?;
            
            for event in pending {
                match self.upload_event(&event).await {
                    Ok(_) => {
                        self.queue.mark_uploaded(&event.id).await?;
                        tracing::debug!("Uploaded: {}", event.path);
                    }
                    Err(e) => {
                        tracing::warn!("Upload failed for {}: {}", event.path, e);
                        
                        // 实现重试逻辑
                        let retry_count = self.queue.get_retry_count(&event.id).await.unwrap_or(0);
                        
                        if self.retry_policy.should_retry(retry_count as u32) {
                            let delay = self.retry_policy.get_delay(retry_count as u32);
                            tracing::info!("Will retry in {:?} (attempt {})", delay, retry_count + 1);
                            
                            // 更新重试次数
                            self.queue.increment_retry(&event.id).await?;
                            
                            // 等待后重试
                            tokio::time::sleep(delay).await;
                        } else {
                            tracing::error!("Max retries reached for {}", event.path);
                            self.queue.mark_failed(&event.id).await?;
                        }
                    }
                }
            }
            
            // 等待一段时间再检查
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
    
    async fn upload_event(&self, event: &PendingEvent) -> anyhow::Result<()> {
        let url = format!("{}/api/ingest/file-events", self.config.hub_url);
        
        let payload = serde_json::json!({
            "device_id": self.config.device_id,
            "events": [{
                "event_id": event.id,
                "data_source_id": event.source_id,
                "event_type": event.event_type,
                "path": event.path,
                "size_bytes": event.size_bytes,
                "mtime": event.mtime,
                "sha256": event.sha256
            }]
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Upload failed: {}", response.status()))
        }
    }
    
    /// 上传artifact
    pub async fn upload_artifact(&self, agent_id: &str, title: &str, content: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/artifacts", self.config.hub_url);
        
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "device_id": self.config.device_id,
            "title": title,
            "artifact_type": "document",
            "content": content,
            "mime_type": "text/markdown"
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Artifact upload failed: {}", response.status()))
        }
    }
    
    /// 上传memory candidate
    pub async fn upload_memory_candidate(&self, agent_id: &str, memory: &serde_json::Value) -> anyhow::Result<()> {
        let url = format!("{}/api/memory/candidates", self.config.hub_url);
        
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "device_id": self.config.device_id,
            "candidates": [memory]
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Memory upload failed: {}", response.status()))
        }
    }
}
