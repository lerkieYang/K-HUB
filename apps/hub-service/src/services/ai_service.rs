use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;

/// Safely truncate a string to at most `max_chars` bytes without panicking on UTF-8 boundaries.
fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    // Find the largest valid UTF-8 boundary <= max_chars
    let mut end = max_chars;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// AI提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AIProvider {
    // 国际
    OpenAI,
    Anthropic,
    Google,
    
    // 国内
    DeepSeek,
    Qwen,       // 通义千问
    Zhipu,      // 智谱GLM
    Moonshot,   // Kimi
    Baichuan,   // 百川
    Yi,         // 零一万物
    Spark,      // 讯飞星火
    Doubao,     // 豆包
    MiniMax,
    MiMo,       // 小米MiMo
    
    // 本地
    Ollama,
    
    // 自定义
    Custom,
}

impl AIProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "openai",
            AIProvider::Anthropic => "anthropic",
            AIProvider::Google => "google",
            AIProvider::DeepSeek => "deepseek",
            AIProvider::Qwen => "qwen",
            AIProvider::Zhipu => "zhipu",
            AIProvider::Moonshot => "moonshot",
            AIProvider::Baichuan => "baichuan",
            AIProvider::Yi => "yi",
            AIProvider::Spark => "spark",
            AIProvider::Doubao => "doubao",
            AIProvider::MiniMax => "minimax",
            AIProvider::MiMo => "mimo",
            AIProvider::Ollama => "ollama",
            AIProvider::Custom => "custom",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => AIProvider::OpenAI,
            "anthropic" | "claude" => AIProvider::Anthropic,
            "google" | "gemini" => AIProvider::Google,
            "deepseek" => AIProvider::DeepSeek,
            "qwen" | "tongyi" => AIProvider::Qwen,
            "zhipu" | "glm" | "chatglm" => AIProvider::Zhipu,
            "moonshot" | "kimi" => AIProvider::Moonshot,
            "baichuan" => AIProvider::Baichuan,
            "yi" | "lingyiwanwu" => AIProvider::Yi,
            "spark" | "xunfei" => AIProvider::Spark,
            "doubao" | "bytedance" => AIProvider::Doubao,
            "minimax" => AIProvider::MiniMax,
            "mimo" | "xiaomi" => AIProvider::MiMo,
            "ollama" => AIProvider::Ollama,
            _ => AIProvider::Custom,
        }
    }
    
    pub fn default_base_url(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "https://api.openai.com/v1",
            AIProvider::Anthropic => "https://api.anthropic.com",
            AIProvider::Google => "https://generativelanguage.googleapis.com/v1beta",
            AIProvider::DeepSeek => "https://api.deepseek.com/v1",
            AIProvider::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
            AIProvider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            AIProvider::Moonshot => "https://api.moonshot.cn/v1",
            AIProvider::Baichuan => "https://api.baichuan-ai.com/v1",
            AIProvider::Yi => "https://api.lingyiwanwu.com/v1",
            AIProvider::Spark => "https://spark-api-open.xf-yun.com/v1",
            AIProvider::Doubao => "https://ark.cn-beijing.volces.com/api/v3",
            AIProvider::MiniMax => "https://api.minimax.chat/v1",
            AIProvider::MiMo => "https://token-plan-sgp.xiaomimimo.com/v1",
            AIProvider::Ollama => "http://localhost:11434",
            AIProvider::Custom => "",
        }
    }
    
    pub fn default_embedding_models(&self) -> Vec<&'static str> {
        match self {
            AIProvider::OpenAI => vec!["text-embedding-3-small", "text-embedding-3-large", "text-embedding-ada-002"],
            AIProvider::Anthropic => vec!["voyage-3", "voyage-3-lite", "voyage-2"],
            AIProvider::Google => vec!["text-embedding-004", "embedding-001"],
            AIProvider::DeepSeek => vec!["text-embedding-v1"],
            AIProvider::Qwen => vec!["text-embedding-v3", "text-embedding-v2"],
            AIProvider::Zhipu => vec!["embedding-3", "embedding-2"],
            AIProvider::Moonshot => vec!["moonshot-v1-8k"],
            AIProvider::Baichuan => vec!["Baichuan-Text-Embedding"],
            AIProvider::Yi => vec!["yi-embedding"],
            AIProvider::Spark => vec!["general"],
            AIProvider::Doubao => vec!["doubao-embedding"],
            AIProvider::MiniMax => vec!["embo-01"],
            AIProvider::MiMo => vec!["text-embedding-3-small"],
            AIProvider::Ollama => vec!["nomic-embed-text", "mxbai-embed-large", "all-minilm"],
            AIProvider::Custom => vec!["text-embedding-3-small"],
        }
    }
    
    pub fn default_chat_models(&self) -> Vec<&'static str> {
        match self {
            AIProvider::OpenAI => vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
            AIProvider::Anthropic => vec!["claude-3-5-sonnet-20241022", "claude-3-haiku-20240307", "claude-3-opus-20240229"],
            AIProvider::Google => vec!["gemini-1.5-pro", "gemini-1.5-flash", "gemini-pro"],
            AIProvider::DeepSeek => vec!["deepseek-chat", "deepseek-coder", "deepseek-v3", "deepseek-r1"],
            AIProvider::Qwen => vec!["qwen-turbo", "qwen-plus", "qwen-max", "qwen-long"],
            AIProvider::Zhipu => vec!["glm-4", "glm-4-flash", "glm-4-air", "glm-3-turbo"],
            AIProvider::Moonshot => vec!["moonshot-v1-128k", "moonshot-v1-32k", "moonshot-v1-8k"],
            AIProvider::Baichuan => vec!["Baichuan4", "Baichuan3-Turbo", "Baichuan2-Turbo"],
            AIProvider::Yi => vec!["yi-large", "yi-medium", "yi-spark", "yi-34b-chat"],
            AIProvider::Spark => vec!["generalv3.5", "generalv3", "general"],
            AIProvider::Doubao => vec!["doubao-pro-32k", "doubao-pro-128k", "doubao-lite-32k"],
            AIProvider::MiniMax => vec!["abab6.5-chat", "abab6.5s-chat", "abab5.5-chat"],
            AIProvider::MiMo => vec!["mimo-v2.5-pro", "mimo-v2.5", "mimo-v2-pro", "mimo-v2-omni"],
            AIProvider::Ollama => vec!["llama3", "qwen2", "mistral", "codellama"],
            AIProvider::Custom => vec!["gpt-4o-mini"],
        }
    }
    
    pub fn default_embedding_model(&self) -> &'static str {
        self.default_embedding_models().first().unwrap_or(&"text-embedding-3-small")
    }
    
    pub fn default_chat_model(&self) -> &'static str {
        self.default_chat_models().first().unwrap_or(&"gpt-4o-mini")
    }
    
    /// 构建chat completion URL
    pub fn build_chat_url(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err("base_url is required but was empty".to_string());
        }
        if base.ends_with("/chat/completions") {
            Ok(base.to_string())
        } else {
            Ok(format!("{}/chat/completions", base))
        }
    }
    
    /// 构建embedding URL
    pub fn build_embedding_url(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err("base_url is required but was empty".to_string());
        }
        if base.ends_with("/embeddings") {
            Ok(base.to_string())
        } else {
            Ok(format!("{}/embeddings", base))
        }
    }
    
    /// 构建models列表URL（用于拉取可用模型）
    pub fn build_models_url(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err("base_url is required but was empty".to_string());
        }
        // 移除末尾的 /v1 如果有，然后添加 /v1/models
        let clean_base = base.strip_suffix("/v1")
            .unwrap_or(base)
            .to_string();
        Ok(format!("{}/v1/models", clean_base))
    }
    
    /// 是否使用OpenAI兼容格式
    #[allow(dead_code)]
    pub fn is_openai_compatible(&self) -> bool {
        matches!(self, 
            AIProvider::OpenAI | 
            AIProvider::DeepSeek | 
            AIProvider::Qwen | 
            AIProvider::Moonshot | 
            AIProvider::Yi | 
            AIProvider::Baichuan |
            AIProvider::MiMo |
            AIProvider::Custom |
            AIProvider::Ollama
        )
    }
    
    /// 是否支持 /v1/embeddings 端点
    /// MiMo 等部分 provider 不支持 embedding API，需要用 chat 模型提取关键词替代
    #[allow(dead_code)]
    pub fn supports_embedding(&self) -> bool {
        !matches!(self, AIProvider::MiMo)
    }
    
    /// 判断模型名是否是 embedding 模型（基于命名规则）
    /// embedding 模型通常包含 "embed" 关键词
    pub fn is_embedding_model(model_name: &str) -> bool {
        let lower = model_name.to_lowercase();
        lower.contains("embed") || lower.contains("voyage") || lower == "general"
    }
    
    /// 是否需要特殊header
    pub fn auth_header(&self, api_key: &str) -> Vec<(&str, String)> {
        match self {
            AIProvider::Anthropic => vec![
                ("x-api-key", api_key.to_string()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
            _ => vec![
                ("Authorization", format!("Bearer {}", api_key)),
            ],
        }
    }
}

/// AI配置（用于Embedding）
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: AIProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_batch_size: usize,
}

/// AI配置（用于Chat）
#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub provider: AIProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

/// 模型信息（从API拉取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub created: Option<u64>,
    pub owned_by: Option<String>,
}

/// 模型列表响应
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelData>,
}

#[derive(Debug, Deserialize)]
struct ModelData {
    id: String,
    created: Option<u64>,
    owned_by: Option<String>,
}

impl EmbeddingConfig {
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, String> {
        let provider = env::var("EMBEDDING_PROVIDER")
            .or_else(|_| env::var("AI_PROVIDER"))
            .unwrap_or_else(|_| "openai".to_string());
        let provider = AIProvider::from_str(&provider);
        
        let api_key = env::var("EMBEDDING_API_KEY")
            .or_else(|_| env::var("AI_API_KEY"))
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        
        let base_url = env::var("EMBEDDING_BASE_URL")
            .or_else(|_| env::var("AI_BASE_URL"))
            .unwrap_or_else(|_| provider.default_base_url().to_string());
        
        let model = env::var("EMBEDDING_MODEL")
            .or_else(|_| env::var("AI_EMBEDDING_MODEL"))
            .unwrap_or_else(|_| provider.default_embedding_model().to_string());
        
        let max_batch_size = env::var("AI_MAX_BATCH_SIZE")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100);
        
        Ok(Self {
            provider,
            api_key,
            base_url,
            model,
            max_batch_size,
        })
    }
}

impl ChatConfig {
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, String> {
        let provider = env::var("CHAT_PROVIDER")
            .or_else(|_| env::var("AI_PROVIDER"))
            .unwrap_or_else(|_| "openai".to_string());
        let provider = AIProvider::from_str(&provider);
        
        let api_key = env::var("CHAT_API_KEY")
            .or_else(|_| env::var("AI_API_KEY"))
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        
        let base_url = env::var("CHAT_BASE_URL")
            .or_else(|_| env::var("AI_BASE_URL"))
            .unwrap_or_else(|_| provider.default_base_url().to_string());
        
        let model = env::var("CHAT_MODEL")
            .or_else(|_| env::var("AI_CHAT_MODEL"))
            .unwrap_or_else(|_| provider.default_chat_model().to_string());
        
        Ok(Self {
            provider,
            api_key,
            base_url,
            model,
        })
    }
}

/// Embedding请求（OpenAI兼容格式）
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

/// Embedding响应
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

/// Chat请求
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Embedding服务
pub struct EmbeddingService {
    config: EmbeddingConfig,
    client: Client,
}

/// Chat服务
pub struct ChatService {
    config: ChatConfig,
    client: Client,
}

/// AI工具服务（用于拉取模型列表等）
pub struct AIToolService {
    client: Client,
}

impl AIToolService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    /// 从API拉取可用模型列表（类似Hermes的/v1/models）
    pub async fn fetch_models(&self, provider: &AIProvider, base_url: &str, api_key: &str) -> Result<Vec<ModelInfo>, String> {
        let url = provider.build_models_url(base_url)?;
        
        let mut req_builder = self.client
            .get(&url)
            .header("Accept", "application/json");
        
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = req_builder
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }
        
        let result: ModelsResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let models: Vec<ModelInfo> = result.data.into_iter().map(|m| ModelInfo {
            id: m.id.clone(),
            name: m.id,  // 使用id作为name
            created: m.created,
            owned_by: m.owned_by,
        }).collect();
        
        Ok(models)
    }
}

impl EmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    #[allow(dead_code)]
    pub fn config_info(&self) -> serde_json::Value {
        serde_json::json!({
            "provider": self.config.provider.as_str(),
            "base_url": self.config.base_url,
            "model": self.config.model,
            "has_api_key": !self.config.api_key.is_empty(),
        })
    }
    
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        let embeddings = self.get_embeddings(vec![text.to_string()]).await?;
        embeddings.into_iter().next().ok_or_else(|| "No embedding returned".to_string())
    }
    
    pub async fn get_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut all_embeddings = Vec::new();
        
        for chunk in texts.chunks(self.config.max_batch_size) {
            let embeddings = self.get_embeddings_openai_compat(chunk).await?;
            all_embeddings.extend(embeddings);
        }
        
        Ok(all_embeddings)
    }
    
    async fn get_embeddings_openai_compat(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.config.model.clone(),
        };
        
        let url = if self.config.provider == AIProvider::Ollama {
            format!("{}/api/embeddings", self.config.base_url)
        } else {
            self.config.provider.build_embedding_url(&self.config.base_url)?
        };
        
        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json");
        
        for (key, value) in self.config.provider.auth_header(&self.config.api_key) {
            req_builder = req_builder.header(key, value);
        }
        
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if self.config.provider == AIProvider::Ollama {
            let embedding = result["embedding"].as_array()
                .ok_or("No embedding in response")?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            return Ok(vec![embedding]);
        }
        
        let embedding_response: EmbeddingResponse = serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse embedding response: {}", e))?;
        
        let mut embeddings = vec![Vec::new(); texts.len()];
        for data in embedding_response.data {
            if data.index < embeddings.len() {
                embeddings[data.index] = data.embedding;
            }
        }
        
        Ok(embeddings)
    }
    
    #[allow(dead_code)]
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
    
    #[allow(dead_code)]
    pub fn find_most_similar(query: &[f32], embeddings: &[(String, Vec<f32>)], top_k: usize) -> Vec<(String, f32)> {
        let mut similarities: Vec<(String, f32)> = embeddings
            .iter()
            .map(|(id, emb)| (id.clone(), Self::cosine_similarity(query, emb)))
            .collect();
        
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(top_k);
        similarities
    }
}

impl ChatService {
    pub fn new(config: ChatConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    #[allow(dead_code)]
    pub fn config_info(&self) -> serde_json::Value {
        serde_json::json!({
            "provider": self.config.provider.as_str(),
            "base_url": self.config.base_url,
            "model": self.config.model,
            "has_api_key": !self.config.api_key.is_empty(),
        })
    }
    
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        self.chat_openai_compat(messages).await
    }
    
    /// 用 Chat 模型提取文本关键词/标签（替代 Embedding）
    /// 返回逗号分隔的关键词字符串
    pub async fn extract_keywords(&self, title: &str, content: &str, _max_chars: usize) -> Result<String, String> {
        // 不截取，直接用原文
        let prompt = format!(
            r#"从以下文档提取20个关键词/标签，用逗号分隔，不要解释。
包含：主题、概念、技术、实体、行为。保留原文+英文。

标题: {}
内容: {}"#,
            title, content
        );
        
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];
        
        // 使用低 temperature 和少 token
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.1,
            max_tokens: 200,
        };
        
        let url = self.config.provider.build_chat_url(&self.config.base_url)?;
        
        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json");
        
        for (key, value) in self.config.provider.auth_header(&self.config.api_key) {
            req_builder = req_builder.header(key, value);
        }
        
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let keywords = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        
        Ok(keywords)
    }
    
    /// 用 Chat 模型生成摘要（用于长文档）
    #[allow(dead_code)]
    pub async fn summarize(&self, content: &str, max_chars: usize) -> Result<String, String> {
        let truncated = safe_truncate(content, max_chars);
        
        let prompt = format!(
            r#"Summarize this document in 2-3 sentences. Focus on main topics and key information.
Return ONLY the summary, no prefix like "Summary:".

Content: {}"#,
            &truncated
        );
        
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];
        
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.3,
            max_tokens: 150,
        };
        
        let url = self.config.provider.build_chat_url(&self.config.base_url)?;
        
        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json");
        
        for (key, value) in self.config.provider.auth_header(&self.config.api_key) {
            req_builder = req_builder.header(key, value);
        }
        
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let summary = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        
        Ok(summary)
    }
    
    async fn chat_openai_compat(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: 1000,
        };
        
        let url = self.config.provider.build_chat_url(&self.config.base_url)?;
        
        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json");
        
        for (key, value) in self.config.provider.auth_header(&self.config.api_key) {
            req_builder = req_builder.header(key, value);
        }
        
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(content)
    }
}
