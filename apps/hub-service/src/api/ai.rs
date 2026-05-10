use axum::{routing::{get, post}, Router, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::db::Database;
use crate::services::ai_service::{EmbeddingConfig, ChatConfig, AIProvider, EmbeddingService, ChatService, AIToolService, ChatMessage};

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/ai/test", post(test_ai_connection))
        .route("/api/ai/info", post(get_ai_info))
        .route("/api/ai/test-embedding", post(test_embedding))
        .route("/api/ai/test-chat", post(test_chat))
        .route("/api/ai/fetch-models", post(fetch_models))
        .route("/api/ai/extract-keywords", post(extract_keywords))
        .route("/api/ai/config", get(get_ai_config).post(save_ai_config))
}

#[derive(Debug, Deserialize)]
struct AITestRequest {
    provider: String,
    api_key: String,
    base_url: Option<String>,
    mode: Option<String>,  // "embedding" or "chat" — defaults to "embedding"
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestEmbeddingRequest {
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestChatRequest {
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FetchModelsRequest {
    provider: String,
    api_key: String,
    base_url: Option<String>,
}

async fn test_ai_connection(
    Json(req): Json<AITestRequest>,
) -> Json<Value> {
    let provider = AIProvider::from_str(&req.provider);
    let base_url = req.base_url.clone().unwrap_or_else(|| provider.default_base_url().to_string());
    let mode = req.mode.unwrap_or_else(|| "embedding".to_string());
    let model_name = req.model.clone().unwrap_or_default();
    let original_mode = mode.clone();
    
    // For providers that don't support /v1/embeddings (e.g. MiMo), force chat mode
    let effective_mode = if mode == "embedding" && !provider.supports_embedding() {
        "chat".to_string()
    } else if mode == "embedding" && !model_name.is_empty() && !AIProvider::is_embedding_model(&model_name) {
        // User configured a chat model (e.g. 'deepseek-chat') in the embedding model field
        // Auto-detect and use chat mode instead of failing
        "chat".to_string()
    } else {
        mode
    };
    
    if effective_mode == "chat" {
        // Test chat connection
        let model = req.model.unwrap_or_else(|| provider.default_chat_model().to_string());
        let chat_config = ChatConfig {
            provider: provider.clone(),
            api_key: req.api_key,
            base_url,
            model: model.clone(),
        };
        let chat_service = ChatService::new(chat_config);
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, please respond with 'OK'.".to_string(),
        }];
        match chat_service.chat(messages).await {
            Ok(_) => {
                let auto_detected = original_mode != effective_mode;
                let msg = if auto_detected {
                    format!("连接成功（检测到 {} 不是Embedding模型，已自动使用Chat模式测试）", model)
                } else {
                    "Chat连接成功".to_string()
                };
                Json(json!({
                    "success": true,
                    "provider": provider.as_str(),
                    "model": model,
                    "mode": "chat",
                    "auto_detected": auto_detected,
                    "message": msg
                }))
            },
            Err(e) => Json(json!({
                "success": false,
                "error": e
            })),
        }
    } else {
        // Test embedding connection
        let model = req.model.unwrap_or_else(|| provider.default_embedding_model().to_string());
        let embedding_config = EmbeddingConfig {
            provider: provider.clone(),
            api_key: req.api_key,
            base_url,
            model: model.clone(),
            max_batch_size: 10,
        };
        let embedding_service = EmbeddingService::new(embedding_config);
        match embedding_service.get_embedding("test").await {
            Ok(_) => Json(json!({
                "success": true,
                "provider": provider.as_str(),
                "model": model,
                "mode": "embedding",
                "message": "Embedding连接成功"
            })),
            Err(e) => Json(json!({
                "success": false,
                "error": e
            })),
        }
    }
}

async fn get_ai_info() -> Json<Value> {
    let providers: Vec<Value> = vec![
        json!({"id": "openai", "name": "OpenAI", "base_url": "https://api.openai.com/v1"}),
        json!({"id": "anthropic", "name": "Anthropic", "base_url": "https://api.anthropic.com"}),
        json!({"id": "google", "name": "Google", "base_url": "https://generativelanguage.googleapis.com/v1beta"}),
        json!({"id": "deepseek", "name": "DeepSeek", "base_url": "https://api.deepseek.com/v1"}),
        json!({"id": "qwen", "name": "通义千问", "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1"}),
        json!({"id": "zhipu", "name": "智谱", "base_url": "https://open.bigmodel.cn/api/paas/v4"}),
        json!({"id": "moonshot", "name": "Moonshot", "base_url": "https://api.moonshot.cn/v1"}),
        json!({"id": "mimo", "name": "MiMo", "base_url": "https://token-plan-sgp.xiaomimimo.com/v1"}),
        json!({"id": "ollama", "name": "Ollama", "base_url": "http://localhost:11434"}),
        json!({"id": "custom", "name": "自定义", "base_url": ""}),
    ];
    
    Json(json!({
        "providers": providers
    }))
}

async fn test_embedding(
    Json(req): Json<TestEmbeddingRequest>,
) -> Json<Value> {
    let provider = AIProvider::from_str(&req.provider);
    let base_url = req.base_url.clone().unwrap_or_else(|| provider.default_base_url().to_string());
    
    if base_url.is_empty() {
        return Json(json!({
            "success": false,
            "error": "base_url is required for this provider but was not provided"
        }));
    }
    
    let model = req.model.unwrap_or_else(|| provider.default_embedding_model().to_string());
    let test_text = req.text.unwrap_or_else(|| "Hello, this is a test embedding.".to_string());
    
    let config = EmbeddingConfig {
        provider: provider.clone(),
        api_key: req.api_key,
        base_url,
        model: model.clone(),
        max_batch_size: 10,
    };
    
    let service = EmbeddingService::new(config);
    
    match service.get_embedding(&test_text).await {
        Ok(embedding) => Json(json!({
            "success": true,
            "provider": provider.as_str(),
            "model": model,
            "embedding_dimensions": embedding.len(),
            "embedding_preview": embedding.iter().take(5).cloned().collect::<Vec<f32>>(),
            "message": "Embedding test successful"
        })),
        Err(e) => Json(json!({
            "success": false,
            "provider": provider.as_str(),
            "model": model,
            "error": e
        })),
    }
}

async fn test_chat(
    Json(req): Json<TestChatRequest>,
) -> Json<Value> {
    let provider = AIProvider::from_str(&req.provider);
    let base_url = req.base_url.clone().unwrap_or_else(|| provider.default_base_url().to_string());
    
    if base_url.is_empty() {
        return Json(json!({
            "success": false,
            "error": "base_url is required for this provider but was not provided"
        }));
    }
    
    let model = req.model.unwrap_or_else(|| provider.default_chat_model().to_string());
    let test_message = req.message.unwrap_or_else(|| "Hello, please respond with 'OK'.".to_string());
    
    let config = ChatConfig {
        provider: provider.clone(),
        api_key: req.api_key,
        base_url,
        model: model.clone(),
    };
    
    let service = ChatService::new(config);
    
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: test_message,
    }];
    
    match service.chat(messages).await {
        Ok(response) => Json(json!({
            "success": true,
            "provider": provider.as_str(),
            "model": model,
            "response": response,
            "message": "Chat test successful"
        })),
        Err(e) => Json(json!({
            "success": false,
            "provider": provider.as_str(),
            "model": model,
            "error": e
        })),
    }
}

/// 从API拉取可用模型列表（类似Hermes的/v1/models）
async fn fetch_models(
    Json(req): Json<FetchModelsRequest>,
) -> Json<Value> {
    let provider = AIProvider::from_str(&req.provider);
    let base_url = req.base_url.clone().unwrap_or_else(|| provider.default_base_url().to_string());
    
    if base_url.is_empty() {
        return Json(json!({
            "success": false,
            "error": "base_url is required for this provider but was not provided"
        }));
    }
    
    let tool_service = AIToolService::new();
    
    match tool_service.fetch_models(&provider, &base_url, &req.api_key).await {
        Ok(models) => Json(json!({
            "success": true,
            "provider": provider.as_str(),
            "models": models.iter().map(|m| json!({
                "id": m.id,
                "name": m.name,
                "created": m.created,
                "owned_by": m.owned_by
            })).collect::<Vec<Value>>(),
            "count": models.len()
        })),
        Err(e) => Json(json!({
            "success": false,
            "provider": provider.as_str(),
            "error": e
        })),
    }
}

#[derive(Debug, Deserialize)]
struct ExtractKeywordsRequest {
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
    title: String,
    content: String,
    max_chars: Option<usize>,
}

/// 用 Chat 模型提取关键词（替代 Embedding）
async fn extract_keywords(
    Json(req): Json<ExtractKeywordsRequest>,
) -> Json<Value> {
    let provider = AIProvider::from_str(&req.provider);
    let base_url = req.base_url.clone().unwrap_or_else(|| provider.default_base_url().to_string());
    
    if base_url.is_empty() {
        return Json(json!({
            "success": false,
            "error": "base_url is required"
        }));
    }
    
    let model = req.model.unwrap_or_else(|| provider.default_chat_model().to_string());
    let max_chars = req.max_chars.unwrap_or(1000);
    
    let config = ChatConfig {
        provider: provider.clone(),
        api_key: req.api_key,
        base_url,
        model: model.clone(),
    };
    
    let service = ChatService::new(config);
    
    match service.extract_keywords(&req.title, &req.content, max_chars).await {
        Ok(keywords) => Json(json!({
            "success": true,
            "provider": provider.as_str(),
            "model": model,
            "keywords": keywords,
            "message": "Keywords extracted successfully"
        })),
        Err(e) => Json(json!({
            "success": false,
            "provider": provider.as_str(),
            "model": model,
            "error": e
        })),
    }
}

/// AI配置持久化结构
#[derive(Debug, Deserialize, serde::Serialize)]
struct AIConfigFile {
    embedding_provider: Option<String>,
    embedding_api_key: Option<String>,
    embedding_base_url: Option<String>,
    embedding_model: Option<String>,
    chat_provider: Option<String>,
    chat_api_key: Option<String>,
    chat_base_url: Option<String>,
    chat_model: Option<String>,
}

/// GET /api/ai/config — 读取 data/ai_config.json (async, non-blocking)
async fn get_ai_config() -> Json<Value> {
    let config_path = std::path::Path::new(&std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("ai_config.json");
    match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => {
            match serde_json::from_str::<AIConfigFile>(&content) {
                Ok(config) => {
                    // Mask API keys - only show last 4 chars
                    let mask_key = |key: &Option<String>| -> String {
                        key.as_ref().map(|k| {
                            if k.len() > 4 { format!("***{}", &k[k.len()-4..]) } else { "***".to_string() }
                        }).unwrap_or_default()
                    };
                    Json(json!({
                        "success": true,
                        "embedding_provider": config.embedding_provider.unwrap_or_default(),
                        "embedding_api_key": mask_key(&config.embedding_api_key),
                        "embedding_base_url": config.embedding_base_url.unwrap_or_default(),
                        "embedding_model": config.embedding_model.unwrap_or_default(),
                        "chat_provider": config.chat_provider.unwrap_or_default(),
                        "chat_api_key": mask_key(&config.chat_api_key),
                        "chat_base_url": config.chat_base_url.unwrap_or_default(),
                        "chat_model": config.chat_model.unwrap_or_default(),
                    }))
                },
                Err(e) => Json(json!({"success": false, "error": format!("Failed to parse config: {}", e)})),
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // File doesn't exist yet, return defaults
                Json(json!({
                    "success": true,
                    "embedding_provider": "",
                    "embedding_api_key": "",
                    "embedding_base_url": "",
                    "embedding_model": "",
                    "chat_provider": "",
                    "chat_api_key": "",
                    "chat_base_url": "",
                    "chat_model": "",
                }))
            } else {
                Json(json!({"success": false, "error": format!("Failed to read config: {}", e)}))
            }
        }
    }
}

/// POST /api/ai/config — 保存到 data/ai_config.json (async, non-blocking)
async fn save_ai_config(
    Json(req): Json<AIConfigFile>,
) -> Json<Value> {
    let data_dir = std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    
    // Ensure data directory exists (async)
    if let Err(e) = tokio::fs::create_dir_all(&data_dir).await {
        return Json(json!({"success": false, "error": format!("Failed to create data dir: {}", e)}));
    }
    
    let config_path = std::path::Path::new(&data_dir).join("ai_config.json");
    match serde_json::to_string_pretty(&req) {
        Ok(json_str) => {
            match tokio::fs::write(config_path, json_str).await {
                Ok(_) => Json(json!({"success": true, "message": "AI config saved"})),
                Err(e) => Json(json!({"success": false, "error": format!("Failed to write config: {}", e)})),
            }
        },
        Err(e) => Json(json!({"success": false, "error": format!("Failed to serialize config: {}", e)})),
    }
}
