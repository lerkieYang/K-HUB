use std::collections::HashMap;
use std::sync::Mutex;
use std::process::{Command, Child};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppMode {
    #[serde(rename = "standalone")]
    Standalone,
    #[serde(rename = "hub")]
    Hub,
    #[serde(rename = "client")]
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub mode: AppMode,
    pub hub_url: Option<String>,
    pub data_dir: String,
    pub autostart_enabled: bool,
}

pub struct ServiceManager {
    running: Mutex<HashMap<String, bool>>,
    processes: Mutex<HashMap<String, Child>>,
    mode: Mutex<AppMode>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(HashMap::new()),
            processes: Mutex::new(HashMap::new()),
            mode: Mutex::new(AppMode::Standalone),
        }
    }

    /// 根据模式启动对应的服务
    pub async fn start_for_mode(&self, config: ModeConfig) -> Result<(), String> {
        // 先停止所有服务
        self.stop_all().await?;

        let mut running = self.running.lock().map_err(|e| e.to_string())?;
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;

        // 设置当前模式
        {
            let mut mode = self.mode.lock().map_err(|e| e.to_string())?;
            *mode = config.mode.clone();
        }

        match config.mode {
            AppMode::Standalone => {
                // 单机模式：启动所有本地服务
                self.start_standalone_services(&config, &mut running, &mut processes)?;
            }
            AppMode::Hub => {
                // Hub模式：启动完整服务 + 同步服务
                self.start_hub_services(&config, &mut running, &mut processes)?;
            }
            AppMode::Client => {
                // Client模式：只启动Collector和MCP代理
                self.start_client_services(&config, &mut running, &mut processes)?;
            }
        }

        Ok(())
    }

    /// 单机模式：启动所有本地服务
    fn start_standalone_services(
        &self,
        config: &ModeConfig,
        running: &mut HashMap<String, bool>,
        processes: &mut HashMap<String, Child>,
    ) -> Result<(), String> {
        // 1. 启动Hub Service (SQLite模式)
        let hub_service = Command::new("hub-service.exe")
            .env("DATABASE_URL", format!("sqlite:{}/knowledge-hub.db", config.data_dir))
            .env("KH_MODE", "standalone")
            .spawn()
            .map_err(|e| format!("Failed to start hub-service: {}", e))?;
        processes.insert("hub-service".to_string(), hub_service);
        running.insert("hub-service".to_string(), true);

        // 2. 启动MCP Server
        let mcp_server = Command::new("mcp-server.exe")
            .env("KH_HUB_URL", "http://127.0.0.1:8443")
            .spawn()
            .map_err(|e| format!("Failed to start mcp-server: {}", e))?;
        processes.insert("mcp-server".to_string(), mcp_server);
        running.insert("mcp-server".to_string(), true);

        // 3. 启动Collector
        let collector = Command::new("collector.exe")
            .env("KH_HUB_URL", "http://127.0.0.1:8443")
            .spawn()
            .map_err(|e| format!("Failed to start collector: {}", e))?;
        processes.insert("collector".to_string(), collector);
        running.insert("collector".to_string(), true);

        Ok(())
    }

    /// Hub模式：启动完整服务
    fn start_hub_services(
        &self,
        config: &ModeConfig,
        running: &mut HashMap<String, bool>,
        processes: &mut HashMap<String, Child>,
    ) -> Result<(), String> {
        // 1. 启动Hub Service (PostgreSQL模式)
        let hub_service = Command::new("hub-service.exe")
            .env("DATABASE_URL", "postgres://localhost/knowledge_hub")
            .env("KH_MODE", "hub")
            .spawn()
            .map_err(|e| format!("Failed to start hub-service: {}", e))?;
        processes.insert("hub-service".to_string(), hub_service);
        running.insert("hub-service".to_string(), true);

        // 2. 启动MCP Server
        let mcp_server = Command::new("mcp-server.exe")
            .env("KH_HUB_URL", "http://127.0.0.1:8443")
            .spawn()
            .map_err(|e| format!("Failed to start mcp-server: {}", e))?;
        processes.insert("mcp-server".to_string(), mcp_server);
        running.insert("mcp-server".to_string(), true);

        // 3. 启动Collector
        let collector = Command::new("collector.exe")
            .env("KH_HUB_URL", "http://127.0.0.1:8443")
            .spawn()
            .map_err(|e| format!("Failed to start collector: {}", e))?;
        processes.insert("collector".to_string(), collector);
        running.insert("collector".to_string(), true);

        // 4. 启动Sync Service (Hub模式特有)
        running.insert("sync-service".to_string(), true);

        Ok(())
    }

    /// Client模式：只启动必要服务
    fn start_client_services(
        &self,
        config: &ModeConfig,
        running: &mut HashMap<String, bool>,
        processes: &mut HashMap<String, Child>,
    ) -> Result<(), String> {
        let hub_url = config.hub_url.as_deref().unwrap_or("http://127.0.0.1:8443");

        // 1. 启动MCP代理
        let mcp_server = Command::new("mcp-server.exe")
            .env("KH_HUB_URL", hub_url)
            .env("KH_PROXY_MODE", "true")
            .spawn()
            .map_err(|e| format!("Failed to start mcp-proxy: {}", e))?;
        processes.insert("mcp-server".to_string(), mcp_server);
        running.insert("mcp-server".to_string(), true);

        // 2. 启动Collector (连接到远程Hub)
        let collector = Command::new("collector.exe")
            .env("KH_HUB_URL", hub_url)
            .spawn()
            .map_err(|e| format!("Failed to start collector: {}", e))?;
        processes.insert("collector".to_string(), collector);
        running.insert("collector".to_string(), true);

        // 3. 标记Hub服务未运行（Client模式不运行Hub）
        running.insert("hub-service".to_string(), false);

        Ok(())
    }

    /// 停止所有服务
    pub async fn stop_all(&self) -> Result<(), String> {
        let mut running = self.running.lock().map_err(|e| e.to_string())?;
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;

        // 终止所有进程
        for (name, process) in processes.iter_mut() {
            if let Err(e) = process.kill() {
                eprintln!("Failed to kill {}: {}", name, e);
            }
        }

        processes.clear();
        running.clear();

        Ok(())
    }

    /// 检查服务是否运行
    pub fn is_running(&self, name: &str) -> bool {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.get(name).copied().unwrap_or(false)
    }

    /// 获取当前模式
    pub fn get_mode(&self) -> AppMode {
        let mode = self.mode.lock().unwrap_or_else(|e| e.into_inner());
        mode.clone()
    }

    /// 获取所有服务状态
    pub fn get_status(&self) -> HashMap<String, bool> {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.clone()
    }
}
