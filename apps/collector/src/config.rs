use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub hub_url: String,
    pub device_id: String,
    pub data_dir: String,
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source_type: String,
    pub recursive: bool,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub scan_mode: String,
    pub debounce_seconds: u64,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".knowledge-hub");
        let config_file = config_dir.join("collector.json");
        
        if config_file.exists() {
            let content = std::fs::read_to_string(config_file)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            // 默认配置
            Ok(Self {
                hub_url: "http://127.0.0.1:8443".to_string(),
                device_id: "local".to_string(),
                data_dir: config_dir.to_string_lossy().to_string(),
                sources: vec![],
            })
        }
    }
}
