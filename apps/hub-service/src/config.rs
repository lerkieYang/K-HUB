use serde::Deserialize;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct ResolvedEmbeddingConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[allow(dead_code)]
pub struct ResolvedChatConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    #[allow(dead_code)]
    pub data_dir: String,
    pub embedding_dir: Option<String>,
    
    // Embedding AI 配置（向量化用）
    pub embedding_provider: Option<String>,
    pub embedding_api_key: Option<String>,
    pub embedding_base_url: Option<String>,
    pub embedding_model: Option<String>,
    
    // Chat AI 配置（问答用）
    pub chat_provider: Option<String>,
    pub chat_api_key: Option<String>,
    pub chat_base_url: Option<String>,
    pub chat_model: Option<String>,
    
    // 兼容旧配置（单一 provider）
    pub ai_provider: Option<String>,
    pub ai_api_key: Option<String>,
    pub ai_base_url: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let data_dir = std::env::var("KH_DATA_DIR")
            .unwrap_or_else(|_| "./data".to_string());
        
        // 通用 API Key fallback
        let default_api_key = std::env::var("AI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .unwrap_or_default();
        
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| format!("sqlite:{}/knowledge-hub.db", data_dir)),
            data_dir: data_dir.clone(),
            embedding_dir: std::env::var("KH_EMBEDDING_DIR")
                .ok()
                .or_else(|| Some(format!("{}/embeddings", data_dir))),
            
            // Embedding 配置
            embedding_provider: std::env::var("EMBEDDING_PROVIDER").ok(),
            embedding_api_key: std::env::var("EMBEDDING_API_KEY").ok()
                .or_else(|| Some(default_api_key.clone())),
            embedding_base_url: std::env::var("EMBEDDING_BASE_URL").ok(),
            embedding_model: std::env::var("EMBEDDING_MODEL").ok(),
            
            // Chat 配置
            chat_provider: std::env::var("CHAT_PROVIDER").ok(),
            chat_api_key: std::env::var("CHAT_API_KEY").ok()
                .or_else(|| Some(default_api_key.clone())),
            chat_base_url: std::env::var("CHAT_BASE_URL").ok(),
            chat_model: std::env::var("CHAT_MODEL").ok(),
            
            // 兼容旧配置
            ai_provider: std::env::var("AI_PROVIDER").ok(),
            ai_api_key: std::env::var("AI_API_KEY").ok(),
            ai_base_url: std::env::var("AI_BASE_URL").ok(),
        })
    }
    
    /// 获取 Embedding 配置（优先使用专用配置，fallback 到通用配置）
    #[allow(dead_code)]
    pub fn get_embedding_config(&self) -> ResolvedEmbeddingConfig {
        let provider = self.embedding_provider.clone()
            .or_else(|| self.ai_provider.clone())
            .unwrap_or_else(|| "openai".to_string());

        let api_key = self.embedding_api_key.clone()
            .or_else(|| self.ai_api_key.clone())
            .unwrap_or_default();

        let base_url = self.embedding_base_url.clone()
            .or_else(|| self.ai_base_url.clone())
            .unwrap_or_default();

        let model = self.embedding_model.clone()
            .unwrap_or_default();

        ResolvedEmbeddingConfig { provider, api_key, base_url, model }
    }
    
    /// 获取 Chat 配置（优先使用专用配置，fallback 到通用配置）
    #[allow(dead_code)]
    pub fn get_chat_config(&self) -> ResolvedChatConfig {
        let provider = self.chat_provider.clone()
            .or_else(|| self.ai_provider.clone())
            .unwrap_or_else(|| "openai".to_string());

        let api_key = self.chat_api_key.clone()
            .or_else(|| self.ai_api_key.clone())
            .unwrap_or_default();

        let base_url = self.chat_base_url.clone()
            .or_else(|| self.ai_base_url.clone())
            .unwrap_or_default();

        let model = self.chat_model.clone()
            .unwrap_or_default();

        ResolvedChatConfig { provider, api_key, base_url, model }
    }
    
    #[allow(dead_code)]
    pub fn is_sqlite(&self) -> bool {
        self.database_url.starts_with("sqlite:")
    }
    
    #[allow(dead_code)]
    pub fn get_embedding_dir(&self) -> PathBuf {
        PathBuf::from(self.embedding_dir.as_deref().unwrap_or("./data/embeddings"))
    }
}
