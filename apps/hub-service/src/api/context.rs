use axum::{routing::post, Router, Json, extract::State};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::db::Database;

fn escape_like(s: &str) -> String {
    s.replace('%', "\\%").replace('_', "\\_")
}

#[derive(Debug, Deserialize)]
pub struct ContextRequest {
    pub agent_id: String,
    pub device_id: Option<String>,
    pub task: String,
    #[allow(dead_code)]
    pub need: Option<Vec<String>>,
    pub max_tokens: Option<i64>,
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/context/request", post(request_context))
}

async fn request_context(
    State(db): State<Database>,
    Json(req): Json<ContextRequest>,
) -> Json<Value> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let expires_at = (now + chrono::Duration::hours(1)).to_rfc3339();
    let like_query = format!("%{}%", escape_like(&req.task));
    let limit = req.max_tokens.unwrap_or(10) as i64;
    
    // 1. 搜索相关 memory（去重：同一 source_file_path 只取最新）
    let memories = sqlx::query_as::<_, (String, String, String, Option<String>, f64, String, String)>(
        r#"SELECT d.id, d.title, SUBSTR(d.content, 1, 500), d.source_file_path, COALESCE(me.confidence, 0.8), d.source_type, COALESCE(me.memory_type, 'semantic')
           FROM doc d
           LEFT JOIN memory_ext me ON d.id = me.id
           WHERE d.status = 'active'
           AND (d.title LIKE ? OR d.content LIKE ? OR d.metadata LIKE ?)
           ORDER BY
             CASE WHEN d.type = 'knowledge' THEN 0 ELSE 1 END,
             COALESCE(me.confidence, 0.8) DESC,
             d.updated_at DESC
           LIMIT ?"#
    )
    .bind(&like_query)
    .bind(&like_query)
    .bind(&like_query)
    .bind(limit * 2)  // 多取一些用于去重
    .fetch_all(&db.pool)
    .await;
    
    // 去重：同一 source_file_path 只保留第一条（最高优先级）
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut relevant_memory: Vec<Value> = Vec::new();
    
    for (id, title, content, source_path, confidence, source_type, memory_type) in memories.unwrap_or_default() {
        // 去重逻辑
        if let Some(ref path) = source_path {
            if !path.is_empty() && seen_paths.contains(path) {
                continue;
            }
            if !path.is_empty() {
                seen_paths.insert(path.clone());
            }
        }
        
        // 来源标注
        let source_label = match source_type.as_str() {
            "knowledge" => "📚 知识库",
            "hermes" | "hermes_session" => "🤖 Hermes",
            "openclaw" | "openclaw_session" => "🐾 OpenClaw",
            "codex" | "codex_session" => "💻 Codex",
            "gemini" | "gemini_session" => "✨ Gemini",
            "agent" => "🤖 Agent",
            "manual" => "📝 手动",
            _ => "📄 其他",
        };
        
        relevant_memory.push(json!({
            "memory_id": id,
            "title": title,
            "content": content,
            "type": memory_type,
            "confidence": confidence,
            "source": source_label,
            "source_path": source_path
        }));
        
        if relevant_memory.len() >= limit as usize {
            break;
        }
    }
    
    // 2. 搜索相关文档
    let docs = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT id, title, SUBSTR(summary, 1, 300) FROM document WHERE title LIKE ? OR summary LIKE ? LIMIT ?"
    )
    .bind(&like_query)
    .bind(&like_query)
    .bind(5)
    .fetch_all(&db.pool)
    .await;
    
    let relevant_docs: Vec<Value> = docs.unwrap_or_default().into_iter().map(|(id, title, summary)| {
        json!({
            "doc_id": id,
            "title": title,
            "excerpt": summary
        })
    }).collect();
    
    // 3. 计算置信度
    let confidence = if relevant_memory.is_empty() && relevant_docs.is_empty() {
        0.0
    } else if relevant_memory.iter().any(|m| m["source"].as_str() == Some("📚 知识库")) {
        0.95
    } else {
        0.80
    };
    
    Json(json!({
        "request_id": request_id,
        "agent_id": req.agent_id,
        "device_id": req.device_id,
        "task_summary": req.task,
        "matched_intent": detect_intent(&req.task),
        "context_pack": {
            "relevant_memory": relevant_memory,
            "relevant_docs": relevant_docs,
            "templates": [],
            "rules": [
                "Agent不得直接写正式memory，所有候选memory必须进入Review Queue"
            ],
            "warnings": get_warnings(&req.task),
            "forbidden_actions": []
        },
        "sources": {
            "memory_count": relevant_memory.len(),
            "docs_count": relevant_docs.len()
        },
        "confidence": confidence,
        "expires_at": expires_at
    }))
}

fn detect_intent(task: &str) -> String {
    let task_lower = task.to_lowercase();
    
    if task_lower.contains("设计") || task_lower.contains("design") {
        "technical_design".to_string()
    } else if task_lower.contains("实现") || task_lower.contains("implement") {
        "implementation".to_string()
    } else if task_lower.contains("测试") || task_lower.contains("test") {
        "testing".to_string()
    } else if task_lower.contains("部署") || task_lower.contains("deploy") {
        "deployment".to_string()
    } else if task_lower.contains("修复") || task_lower.contains("fix") || task_lower.contains("bug") {
        "bugfix".to_string()
    } else {
        "general".to_string()
    }
}

fn get_warnings(task: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    
    if task.contains("SMB") || task.contains("smb") {
        warnings.push("SMB文件监听可能漏报，必须使用定时扫描兜底".to_string());
    }
    if task.contains("删除") || task.contains("delete") {
        warnings.push("删除操作请注意数据备份".to_string());
    }
    if task.contains("密码") || task.contains("password") || task.contains("secret") {
        warnings.push("涉及敏感信息，请使用 policy.check 工具检查".to_string());
    }
    
    warnings
}
