use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use chrono::Utc;

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    pub installed: bool,
    pub configured: bool,
    pub config_path: Option<String>,
    pub output_path: Option<String>,
    pub mcp_config_path: Option<String>,
    pub environment: String,  // "windows" or "wsl"
}

/// Agent类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    // CLI Agents
    Hermes,
    Codex,
    Gemini,
    OpenClaw,
    OpenCode,
    ClaudeCode,
    Aider,
    
    // IDE Agents
    Cursor,
    Windsurf,
    Continue,
    Cody,
    Copilot,
    Cline,
    
    Unknown,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Hermes => "hermes",
            AgentType::Codex => "codex",
            AgentType::Gemini => "gemini",
            AgentType::OpenClaw => "openclaw",
            AgentType::OpenCode => "opencode",
            AgentType::ClaudeCode => "claude-code",
            AgentType::Aider => "aider",
            AgentType::Cursor => "cursor",
            AgentType::Windsurf => "windsurf",
            AgentType::Continue => "continue",
            AgentType::Cody => "cody",
            AgentType::Copilot => "copilot",
            AgentType::Cline => "cline",
            AgentType::Unknown => "unknown",
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Hermes => "Hermes Agent",
            AgentType::Codex => "OpenAI Codex CLI",
            AgentType::Gemini => "Gemini CLI",
            AgentType::OpenClaw => "OpenClaw",
            AgentType::OpenCode => "OpenCode",
            AgentType::ClaudeCode => "Claude Code",
            AgentType::Aider => "Aider",
            AgentType::Cursor => "Cursor",
            AgentType::Windsurf => "Windsurf",
            AgentType::Continue => "Continue",
            AgentType::Cody => "Sourcegraph Cody",
            AgentType::Copilot => "GitHub Copilot",
            AgentType::Cline => "Cline",
            AgentType::Unknown => "Unknown",
        }
    }
}

/// Agent发现服务
pub struct AgentDiscovery {
    home_dirs: Vec<(PathBuf, String)>,  // (path, environment)
}

impl AgentDiscovery {
    pub fn new() -> Self {
        let mut home_dirs = Vec::new();
        
        // Windows用户目录
        if let Some(win_home) = dirs::home_dir() {
            home_dirs.push((win_home, "windows".to_string()));
        }
        
        // Try WSL paths via \\wsl.localhost\Ubuntu\home\* (accessible from Windows)
        let wsl_unc_prefix = PathBuf::from(r"\\wsl.localhost\Ubuntu\home");
        if wsl_unc_prefix.exists() {
            if let Ok(entries) = std::fs::read_dir(&wsl_unc_prefix) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().map(|n| n != "lost+found").unwrap_or(false) {
                        let has_agents = path.join(".hermes").exists()
                            || path.join(".openclaw").exists()
                            || path.join(".codex").exists();
                        if has_agents && !home_dirs.iter().any(|(p, _)| p == &path) {
                            home_dirs.push((path, "wsl".to_string()));
                        }
                    }
                }
            }
        }
        
        // Also try native /home/* paths (works when hub-service runs in WSL)
        let wsl_user_home = PathBuf::from("/home/user");
        if wsl_user_home.exists() && !home_dirs.iter().any(|(p, _)| p == &wsl_user_home) {
            home_dirs.push((wsl_user_home, "wsl".to_string()));
        }
        
        if let Ok(username) = std::env::var("USER") {
            let user_home = PathBuf::from("/home").join(&username);
            if user_home.exists() && !home_dirs.iter().any(|(p, _)| p == &user_home) {
                home_dirs.push((user_home, "wsl".to_string()));
            }
        }
        
        // Scan all /home/* directories for agent configs (detect other users)
        let wsl_home = PathBuf::from("/home");
        if wsl_home.exists() {
            if let Ok(entries) = std::fs::read_dir(&wsl_home) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().map(|n| n != "lost+found").unwrap_or(false) {
                        let has_agents = path.join(".hermes").exists()
                            || path.join(".openclaw").exists()
                            || path.join(".codex").exists();
                        if has_agents && !home_dirs.iter().any(|(p, _)| p == &path) {
                            home_dirs.push((path, "wsl".to_string()));
                        }
                    }
                }
            }
        }
        
        Self { home_dirs }
    }
    
    /// 扫描所有已安装的Agent
    pub fn scan_agents(&self) -> Vec<AgentInfo> {
        let mut agents = Vec::new();
        let mut seen_types = std::collections::HashSet::new();
        
        for (home_dir, env) in &self.home_dirs {
            // CLI Agents
            let cli_agents = vec![
                self.check_agent(home_dir, env, AgentType::Hermes, ".hermes", "config.yaml"),
                self.check_agent(home_dir, env, AgentType::Codex, ".codex", "config.toml"),
                self.check_agent(home_dir, env, AgentType::Gemini, ".gemini", "settings.json"),
                self.check_agent(home_dir, env, AgentType::OpenClaw, ".openclaw", "openclaw.json"),
                self.check_agent(home_dir, env, AgentType::OpenCode, ".opencode", "config.json"),
                self.check_agent(home_dir, env, AgentType::ClaudeCode, ".claude", "settings.json"),
                self.check_agent(home_dir, env, AgentType::Aider, ".aider", "config.yml"),
            ];
            
            // IDE Agents (通常在Windows上)
            let ide_agents = if env == "windows" {
                vec![
                    self.check_ide_agent(home_dir, AgentType::Cursor, ".cursor"),
                    self.check_ide_agent(home_dir, AgentType::Windsurf, ".windsurf"),
                    self.check_ide_agent(home_dir, AgentType::Continue, ".continue"),
                ]
            } else {
                vec![]
            };
            
            for agent in cli_agents.into_iter().chain(ide_agents.into_iter()) {
                if agent.installed {
                    // Allow same agent type in different environments
                    let key = format!("{}-{}", agent.agent_type.as_str(), agent.environment);
                    if !seen_types.contains(&key) {
                        seen_types.insert(key);
                        agents.push(agent);
                    }
                }
            }
        }
        
        agents
    }
    
    /// 检查CLI Agent
    fn check_agent(&self, home_dir: &Path, env: &str, agent_type: AgentType, dir_name: &str, config_file: &str) -> AgentInfo {
        let config_dir = home_dir.join(dir_name);
        let config_path = config_dir.join(config_file);
        
        // 根据 agent 类型确定 output 目录
        let output_dir = match agent_type {
            AgentType::Hermes => {
                // Hermes 使用 memories 目录（注意是 memories 不是 memory）
                if config_dir.join("memories").exists() {
                    Some(config_dir.join("memories"))
                } else if config_dir.join("memory").exists() {
                    Some(config_dir.join("memory"))
                } else {
                    Some(config_dir.join("memories"))
                }
            },
            AgentType::Codex => {
                if config_dir.join("memory").exists() {
                    Some(config_dir.join("memory"))
                } else {
                    Some(config_dir.join("memory"))
                }
            },
            AgentType::Gemini => {
                // Gemini 用 tmp/<user>/chats/ 存储会话
                // 遍历 tmp/ 找第一个有 chats 的子目录
                let tmp_dir = config_dir.join("tmp");
                let mut found_chats = None;
                if tmp_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                        for entry in entries.flatten() {
                            let chats_dir = entry.path().join("chats");
                            if chats_dir.exists() {
                                found_chats = Some(chats_dir);
                                break;
                            }
                        }
                    }
                }
                found_chats.or_else(|| Some(config_dir.join("tmp")))
            },
            AgentType::OpenClaw => {
                // OpenClaw 使用 main.sqlite 在 memory 子目录
                let memory_dir = config_dir.join("memory");
                if memory_dir.exists() {
                    Some(memory_dir)
                } else {
                    Some(config_dir.clone())
                }
            },
            _ => {
                if config_dir.join("memory").exists() {
                    Some(config_dir.join("memory"))
                } else if config_dir.join("output").exists() {
                    Some(config_dir.join("output"))
                } else {
                    Some(config_dir.join("memory"))
                }
            }
        };
        
        let mut installed = config_dir.exists();
        // Validate that the agent directory contains a real installation,
        // not just an empty shell (e.g. empty .codex dir from project trust entries)
        if installed {
            installed = self.validate_agent_installation(&agent_type, &config_dir);
        }
        
        AgentInfo {
            id: format!("{}-{}", agent_type.as_str(), env),
            name: agent_type.display_name().to_string(),
            agent_type,
            installed,
            configured: self.check_mcp_configured(&config_dir),
            config_path: if config_path.exists() { Some(config_path.to_string_lossy().to_string()) } else { None },
            output_path: output_dir.map(|p| p.to_string_lossy().to_string()),
            mcp_config_path: Some(config_path.to_string_lossy().to_string()),
            environment: env.to_string(),
        }
    }
    
    /// Validate that an agent directory contains a real installation,
    /// not just an empty shell directory. This prevents empty .codex dirs
    /// (created by project trust entries) from showing as installed Codex.
    fn validate_agent_installation(&self, agent_type: &AgentType, config_dir: &Path) -> bool {
        match agent_type {
            AgentType::Hermes => {
                // Hermes: .hermes/config.yaml must exist and have content >100 bytes
                let config = config_dir.join("config.yaml");
                match std::fs::metadata(&config) {
                    Ok(meta) => meta.len() > 100,
                    Err(_) => false,
                }
            }
            AgentType::Codex => {
                // Codex: .codex/config.toml must exist and contain real config keys
                // (not just [projects.*] trust entries)
                let config = config_dir.join("config.toml");
                match std::fs::read_to_string(&config) {
                    Ok(content) => {
                        let lower = content.to_lowercase();
                        lower.contains("model")
                            || lower.contains("api_key")
                            || lower.contains("provider")
                            || lower.contains("plugins")
                            || lower.contains("windows")
                            || lower.contains("marketplaces")
                            || lower.contains("sandbox")
                    }
                    Err(_) => false,
                }
            }
            AgentType::OpenClaw => {
                // OpenClaw: .openclaw/openclaw.json must exist and be >100 bytes
                let config = config_dir.join("openclaw.json");
                match std::fs::metadata(&config) {
                    Ok(meta) => meta.len() > 100,
                    Err(_) => false,
                }
            }
            _ => {
                // Other agents: just having the config_dir is enough
                true
            }
        }
    }

    /// 检查IDE Agent
    fn check_ide_agent(&self, home_dir: &Path, agent_type: AgentType, dir_name: &str) -> AgentInfo {
        let config_dir = home_dir.join(dir_name);
        
        AgentInfo {
            id: format!("{}-windows", agent_type.as_str()),
            name: agent_type.display_name().to_string(),
            agent_type,
            installed: config_dir.exists(),
            configured: false,
            config_path: None,
            output_path: None,
            mcp_config_path: None,
            environment: "windows".to_string(),
        }
    }
    
    /// 检查MCP是否已配置
    fn check_mcp_configured(&self, config_dir: &Path) -> bool {
        let config_files = vec![
            config_dir.join("config.yaml"),
            config_dir.join("config.json"),
            config_dir.join("config.toml"),
            config_dir.join("settings.json"),
            config_dir.join("mcp.yaml"),
            config_dir.join("mcp.json"),
            config_dir.join("openclaw.json"),
        ];
        
        for config_file in config_files {
            if config_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_file) {
                    if content.contains("k-hub") || content.contains("knowledge-hub") || content.contains("8443") {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// 配置Agent的MCP连接
    pub fn configure_agent(&self, agent_id: &str, hub_url: &str) -> Result<String, String> {
        let agent_type = match agent_id {
            "hermes" => AgentType::Hermes,
            "codex" => AgentType::Codex,
            "gemini" => AgentType::Gemini,
            "openclaw" => AgentType::OpenClaw,
            "opencode" => AgentType::OpenCode,
            "claude-code" => AgentType::ClaudeCode,
            _ => return Err(format!("Agent {} configuration not supported yet", agent_id)),
        };
        
        // 找到对应的home目录
        for (home_dir, env) in &self.home_dirs {
            let dir_name = match agent_type {
                AgentType::Hermes => ".hermes",
                AgentType::Codex => ".codex",
                AgentType::Gemini => ".gemini",
                AgentType::OpenClaw => ".openclaw",
                AgentType::OpenCode => ".opencode",
                AgentType::ClaudeCode => ".claude",
                _ => continue,
            };
            
            let config_dir = home_dir.join(dir_name);
            if config_dir.exists() {
                // OpenClaw 特殊处理：修改 openclaw.json
                if agent_type == AgentType::OpenClaw {
                    return self.configure_openclaw_mcp(&config_dir, hub_url, env);
                }
                return self.write_mcp_config(&config_dir, hub_url, env);
            }
        }
        
        Err(format!("Agent {} not found", agent_id))
    }
    
    /// OpenClaw MCP 配置（修改 openclaw.json）
    fn configure_openclaw_mcp(&self, config_dir: &Path, hub_url: &str, env: &str) -> Result<String, String> {
        let config_path = config_dir.join("openclaw.json");
        
        // 读取现有配置
        let mut config: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read openclaw.json: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse openclaw.json: {}", e))?
        } else {
            serde_json::json!({})
        };
        
        // 添加 MCP 配置
        let mcp_config = serde_json::json!({
            "k-hub": {
                "url": format!("{}/mcp", hub_url),
                "transport": "http"
            }
        });
        
        // 确保 mcp.servers 存在
        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("mcp").is_none() {
            config["mcp"] = serde_json::json!({});
        }
        if config["mcp"].get("servers").is_none() {
            config["mcp"]["servers"] = serde_json::json!({});
        }
        
        // 写入 MCP 服务器配置
        config["mcp"]["servers"]["k-hub"] = mcp_config;
        
        // 写回文件
        let json_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, json_str)
            .map_err(|e| format!("Failed to write openclaw.json: {}", e))?;
        
        Ok(format!("Configured OpenClaw MCP in {} ({})", config_path.display(), env))
    }
    
    /// 写入MCP配置
    fn write_mcp_config(&self, config_dir: &Path, hub_url: &str, env: &str) -> Result<String, String> {
        let mcp_config = format!(
            r#"mcp:
  servers:
    k-hub:
      url: {}/mcp
      transport: http
"#,
            hub_url
        );
        
        // 创建输出目录
        let output_dir = config_dir.join("memory");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
        
        // 写入配置
        let config_path = config_dir.join("mcp.yaml");
        std::fs::write(&config_path, mcp_config).map_err(|e| e.to_string())?;
        
        Ok(format!("Configured {}. MCP config: {} ({})", 
            config_dir.file_name().unwrap_or_default().to_string_lossy(),
            config_path.display(),
            env
        ))
    }
    
    /// 取消配置Agent的MCP连接
    pub fn unconfigure_agent(&self, agent_id: &str) -> Result<String, String> {
        let agent_type = match agent_id {
            "hermes" => AgentType::Hermes,
            "codex" => AgentType::Codex,
            "gemini" => AgentType::Gemini,
            "openclaw" => AgentType::OpenClaw,
            "opencode" => AgentType::OpenCode,
            "claude-code" => AgentType::ClaudeCode,
            _ => return Err(format!("Agent {} unconfiguration not supported yet", agent_id)),
        };

        for (home_dir, _env) in &self.home_dirs {
            let dir_name = match agent_type {
                AgentType::Hermes => ".hermes",
                AgentType::Codex => ".codex",
                AgentType::Gemini => ".gemini",
                AgentType::OpenClaw => ".openclaw",
                AgentType::OpenCode => ".opencode",
                AgentType::ClaudeCode => ".claude",
                _ => continue,
            };

            let config_dir = home_dir.join(dir_name);
            if config_dir.exists() {
                // OpenClaw 特殊处理：从 openclaw.json 移除 MCP 配置
                if agent_type == AgentType::OpenClaw {
                    return self.unconfigure_openclaw_mcp(&config_dir);
                }
                
                let mcp_path = config_dir.join("mcp.yaml");
                if mcp_path.exists() {
                    std::fs::remove_file(&mcp_path).map_err(|e| e.to_string())?;
                    return Ok(format!("Removed MCP config: {}", mcp_path.display()));
                } else {
                    return Err(format!("No mcp.yaml found in {}", config_dir.display()));
                }
            }
        }

        Err(format!("Agent {} config directory not found", agent_id))
    }
    
    /// OpenClaw 取消 MCP 配置（从 openclaw.json 移除）
    fn unconfigure_openclaw_mcp(&self, config_dir: &Path) -> Result<String, String> {
        let config_path = config_dir.join("openclaw.json");
        
        if !config_path.exists() {
            return Err(format!("openclaw.json not found in {}", config_dir.display()));
        }
        
        // 读取现有配置
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read openclaw.json: {}", e))?;
        let mut config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse openclaw.json: {}", e))?;
        
        // 移除 MCP 配置
        if let Some(mcp) = config.get_mut("mcp") {
            if let Some(servers) = mcp.get_mut("servers") {
                if let Some(obj) = servers.as_object_mut() {
                    obj.remove("k-hub");
                    obj.remove("knowledge-hub");
                }
            }
        }
        
        // 写回文件
        let json_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, json_str)
            .map_err(|e| format!("Failed to write openclaw.json: {}", e))?;
        
        Ok(format!("Removed OpenClaw MCP config from {}", config_path.display()))
    }

    /// 生成memory-export.json模板
    #[allow(dead_code)]
    pub fn generate_export_template(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": "1.0",
            "agent_id": "example-agent",
            "device_id": "local",
            "session_id": "ses_example",
            "task_id": "task_example",
            "created_at": Utc::now().to_rfc3339(),
            "task_summary": "Example task summary",
            "user_request": "Example user request",
            "artifacts": [],
            "memories": [
                {
                    "scope": "project",
                    "type": "decision",
                    "content": "Example memory content",
                    "reason": "Example reason",
                    "confidence": 0.9,
                    "suggested_visibility": "shared"
                }
            ]
        })
    }
}
