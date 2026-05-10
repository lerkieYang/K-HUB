mod config;
mod watcher;
mod queue;
mod uploader;
mod agent_watcher;
mod proxy;

use config::Config;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let config = Config::load()?;
    
    tracing::info!("Starting Collector");
    tracing::info!("Hub URL: {}", config.hub_url);
    tracing::info!("Device ID: {}", config.device_id);
    tracing::info!("Sources: {}", config.sources.len());
    
    // 启动文件监听
    let file_watcher = watcher::FileWatcher::new(config.clone())?;
    let file_watcher_handle = tokio::spawn(async move {
        if let Err(e) = file_watcher.start().await {
            tracing::error!("File watcher error: {}", e);
        }
    });
    
    // 启动Agent产出监听
    let agent_watcher = agent_watcher::AgentWatcher::new(config.clone())?;
    let agent_watcher_handle = tokio::spawn(async move {
        if let Err(e) = agent_watcher.start().await {
            tracing::error!("Agent watcher error: {}", e);
        }
    });
    
    // 启动上传服务
    let uploader = uploader::Uploader::new(config.clone())?;
    let uploader_handle = tokio::spawn(async move {
        if let Err(e) = uploader.start().await {
            tracing::error!("Uploader error: {}", e);
        }
    });
    
    // 启动MCP代理
    let proxy = proxy::McpProxy::new(config.clone())?;
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy.start().await {
            tracing::error!("MCP proxy error: {}", e);
        }
    });
    
    tracing::info!("Collector started successfully");
    
    // 等待所有任务完成
    tokio::select! {
        _ = file_watcher_handle => {},
        _ = agent_watcher_handle => {},
        _ = uploader_handle => {},
        _ = proxy_handle => {},
    }
    
    Ok(())
}
