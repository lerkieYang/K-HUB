use axum::{routing::post, Router, Json, extract::State};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use chrono::Utc;
use crate::db::Database;
use crate::services::agent_discovery::AgentDiscovery;

pub fn routes() -> Router<Database> {
    Router::new()
        // Legacy endpoints (backward compatibility)
        .route("/api/memory/pull-from-agents", post(pull_from_agents))
        .route("/api/memory/pull-sessions", post(pull_sessions))
        // New split endpoints
        .route("/api/session/pull", post(pull_sessions_only))
        .route("/api/memory/pull", post(pull_memory_only))
}

/// 从 Agent 目录拉取 Memory 和 Sessions
async fn pull_from_agents(
    State(db): State<Database>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.scan_agents();
    
    let mut total_pulled = 0;
    let mut agent_results = Vec::new();
    let mut sources = Vec::new();
    
    // 拉取 Memory
    for agent in &agents {
        if !agent.installed {
            continue;
        }
        
        let memory_path = match &agent.output_path {
            Some(p) => PathBuf::from(p),
            None => continue,
        };
        
        if !memory_path.exists() {
            continue;
        }
        
        let count = match agent.agent_type {
            crate::services::agent_discovery::AgentType::Hermes => {
                pull_hermes_memory(&db, &memory_path).await
            }
            crate::services::agent_discovery::AgentType::OpenClaw => {
                pull_openclaw_memory(&db, &memory_path).await
            }
            crate::services::agent_discovery::AgentType::Codex => {
                pull_codex_memory(&db, &memory_path).await
            }
            crate::services::agent_discovery::AgentType::Gemini => {
                pull_gemini_memory(&db, &memory_path).await
            }
            _ => 0,
        };
        
        if count > 0 {
            total_pulled += count;
            sources.push(agent.id.clone());
            agent_results.push(json!({
                "agent": agent.id,
                "environment": agent.environment,
                "count": count,
                "type": "memory"
            }));
        }
    }
    
    // 拉取 Sessions
    let session_result = pull_sessions(State(db)).await;
    let session_data = session_result.0;
    let session_total = session_data.get("total_pulled").and_then(|v| v.as_i64()).unwrap_or(0);
    
    if session_total > 0 {
        total_pulled += session_total as i32;
        if let Some(results) = session_data.get("agent_results").and_then(|v| v.as_array()) {
            for r in results {
                agent_results.push(r.clone());
            }
        }
    }
    
    Json(json!({
        "success": true,
        "total_pulled": total_pulled,
        "sources": sources,
        "agent_results": agent_results,
        "pulled_at": Utc::now().to_rfc3339()
    }))
}

/// POST /api/session/pull — Pull sessions only from all agents into doc + session_ext
async fn pull_sessions_only(
    State(db): State<Database>,
) -> Json<Value> {
    // Register task in task registry
    let (task_id, cancel_flag, progress) = crate::services::task_registry::register_task(
        "pulling_sessions",
        "拉取会话记录"
    ).await;
    
    let task_id_clone = task_id.clone();
    
    // Spawn background task for pulling
    let db_clone = db.clone();
    tokio::spawn(async move {
        let discovery = AgentDiscovery::new();
        let agents = discovery.scan_agents();
        let installed_count = agents.iter().filter(|a| a.installed).count() as u64;
        progress.set_total_files(installed_count);
        progress.set_stage("pulling").await;

        let mut total_pulled = 0i32;
        let mut agent_results = Vec::new();
        let mut all_debug: Vec<String> = Vec::new();

        for agent in &agents {
            // Check if task was cancelled
            if cancel_flag.load(Ordering::Relaxed) {
                all_debug.push("Task cancelled".to_string());
                break;
            }
            
            if !agent.installed {
                all_debug.push(format!("{}: not installed", agent.id));
                continue;
            }

            let config_dir = match &agent.config_path {
                Some(p) => {
                    let dir = PathBuf::from(p).parent().map(|p| p.to_path_buf()).unwrap_or_default();
                    if !dir.exists() && dir.to_string_lossy().contains("wsl.localhost") {
                        let wsl_path = PathBuf::from("/home").join(
                            dir.components().skip_while(|c| !c.as_os_str().to_string_lossy().contains("home"))
                                .skip(1)
                                .collect::<PathBuf>()
                        );
                        if wsl_path.exists() {
                            all_debug.push(format!("{}: WSL fallback {} -> {}", agent.id, dir.display(), wsl_path.display()));
                            wsl_path
                        } else {
                            all_debug.push(format!("{}: config_dir={} (not found)", agent.id, dir.display()));
                            progress.inc_indexed_count();
                            continue;
                        }
                    } else {
                        all_debug.push(format!("{}: config_dir={}", agent.id, dir.display()));
                        dir
                    }
                },
                None => {
                    all_debug.push(format!("{}: no config_path", agent.id));
                    progress.inc_indexed_count();
                    continue;
                }
            };

            let sessions_exist = if agent.agent_type == crate::services::agent_discovery::AgentType::Gemini {
                config_dir.join("tmp").exists()
            } else if agent.agent_type == crate::services::agent_discovery::AgentType::OpenClaw {
                config_dir.join("agents").join("main").join("sessions").exists()
            } else {
                config_dir.join("sessions").exists()
            };

            if !sessions_exist {
                all_debug.push(format!("{}: no sessions dir", agent.id));
                progress.inc_indexed_count();
                continue;
            }

            let count = match agent.agent_type {
                crate::services::agent_discovery::AgentType::Hermes => {
                    let (c, d) = pull_hermes_sessions(&db_clone, &config_dir).await;
                    all_debug.extend(d);
                    c
                }
                crate::services::agent_discovery::AgentType::Codex => {
                    let (c, d) = pull_codex_sessions(&db_clone, &config_dir).await;
                    all_debug.extend(d);
                    c
                }
                crate::services::agent_discovery::AgentType::Gemini => {
                    let (c, d) = pull_gemini_sessions(&db_clone, &config_dir).await;
                    all_debug.extend(d);
                    c
                }
                crate::services::agent_discovery::AgentType::OpenClaw => {
                    let (c, d) = pull_openclaw_sessions(&db_clone, &config_dir).await;
                    all_debug.extend(d);
                    c
                }
                _ => {
                    all_debug.push(format!("{}: unsupported agent type", agent.id));
                    0
                }
            };

            if count > 0 {
                total_pulled += count;
                agent_results.push(json!({
                    "agent": agent.id,
                    "environment": agent.environment,
                    "sessions_count": count
                }));
            }
            progress.inc_indexed_count();
        }

        progress.set_stage("done").await;
        // Unregister task
        crate::services::task_registry::unregister_task(&task_id_clone).await;
        
        tracing::info!("Session pull completed: {} sessions pulled", total_pulled);
    });
    
    // Return immediately
    Json(json!({
        "success": true,
        "message": "会话拉取任务已启动，请在任务中心查看进度",
        "task_id": task_id
    }))
}

/// POST /api/memory/pull — Pull memory only from all agents into doc + memory_ext
async fn pull_memory_only(
    State(db): State<Database>,
) -> Json<Value> {
    // Register task in task registry
    let (task_id, cancel_flag, progress) = crate::services::task_registry::register_task(
        "pulling_memory",
        "拉取记忆"
    ).await;
    
    let task_id_clone = task_id.clone();
    
    // Spawn background task for pulling
    let db_clone = db.clone();
    tokio::spawn(async move {
        let discovery = AgentDiscovery::new();
        let agents = discovery.scan_agents();
        let installed_count = agents.iter().filter(|a| a.installed).count() as u64;
        progress.set_total_files(installed_count);
        progress.set_stage("pulling").await;

        let mut total_pulled = 0i32;
        let mut agent_results = Vec::new();
        let mut sources = Vec::new();

        for agent in &agents {
            // Check if task was cancelled
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            
            if !agent.installed {
                continue;
            }

            let memory_path = match &agent.output_path {
                Some(p) => PathBuf::from(p),
                None => continue,
            };

            if !memory_path.exists() {
                progress.inc_indexed_count();
                continue;
            }

            let count = match agent.agent_type {
                crate::services::agent_discovery::AgentType::Hermes => {
                    pull_hermes_memory(&db_clone, &memory_path).await
                }
                crate::services::agent_discovery::AgentType::OpenClaw => {
                    pull_openclaw_memory(&db_clone, &memory_path).await
                }
                crate::services::agent_discovery::AgentType::Codex => {
                    pull_codex_memory(&db_clone, &memory_path).await
                }
                crate::services::agent_discovery::AgentType::Gemini => {
                    pull_gemini_memory(&db_clone, &memory_path).await
                }
                _ => 0,
            };

            if count > 0 {
                total_pulled += count;
                sources.push(agent.id.clone());
                agent_results.push(json!({
                    "agent": agent.id,
                    "environment": agent.environment,
                    "count": count,
                    "type": "memory"
                }));
            }
            progress.inc_indexed_count();
        }

        progress.set_stage("done").await;
        // Unregister task
        crate::services::task_registry::unregister_task(&task_id_clone).await;
        
        tracing::info!("Memory pull completed: {} memories pulled", total_pulled);
    });
    
    // Return immediately
    Json(json!({
        "success": true,
        "message": "记忆拉取任务已启动，请在任务中心查看进度",
        "task_id": task_id
    }))
}

/// 从 Hermes 拉取 Memory (MEMORY.md + USER.md)
async fn pull_hermes_memory(db: &Database, memory_dir: &PathBuf) -> i32 {
    let mut count = 0;
    
    // 读取 MEMORY.md
    let memory_file = memory_dir.join("MEMORY.md");
    if memory_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&memory_file).await {
            let entries = parse_hermes_memory(&content, "memory");
            for entry in entries {
                insert_doc_entry(db, &entry, "hermes", "MEMORY.md", "memory", None, None, None).await;
            }
        }
    }
    
    // 读取 USER.md
    let user_file = memory_dir.join("USER.md");
    if user_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&user_file).await {
            let entries = parse_hermes_memory(&content, "user");
            for entry in entries {
                insert_doc_entry(db, &entry, "hermes", "USER.md", "memory", None, None, None).await;
            }
        }
    }
    
    count
}

/// 解析 Hermes Memory 格式 (§ 分隔)
fn parse_hermes_memory(content: &str, category: &str) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();
    
    for (i, section) in content.split("§").enumerate() {
        let section = section.trim();
        if section.is_empty() {
            continue;
        }
        
        // 尝试提取日期和标题
        let (title, body) = if let Some(colon_pos) = section.find(':') {
            let first_line = &section[..colon_pos];
            if first_line.len() < 50 && !first_line.contains('\n') {
                (first_line.trim().to_string(), section[colon_pos + 1..].trim().to_string())
            } else {
                (format!("{}_{}", category, i), section.to_string())
            }
        } else {
            (format!("{}_{}", category, i), section.to_string())
        };
        
        entries.push(MemoryEntry {
            title,
            content: body,
            source: "hermes".to_string(),
            category: category.to_string(),
        });
    }
    
    entries
}

/// 从 OpenClaw 拉取 Memory (SQLite)
async fn pull_openclaw_memory(db: &Database, memory_dir: &PathBuf) -> i32 {
    let sqlite_file = memory_dir.join("main.sqlite");
    if !sqlite_file.exists() {
        return 0;
    }
    
    // UNC 路径 (\\wsl.localhost\...) SQLite 直接打开会无限挂起
    // 解决方案：先复制到本地临时文件再读取
    let sqlite_path_str = sqlite_file.to_string_lossy().to_string();
    let is_unc = sqlite_path_str.starts_with("\\\\") || sqlite_path_str.starts_with("//");
    
    let local_copy = if is_unc {
        let tmp = std::env::temp_dir().join("openclaw_memory_copy.sqlite");
        match std::fs::copy(&sqlite_file, &tmp) {
            Ok(_) => {
                tracing::info!("Copied OpenClaw SQLite from UNC to local: {}", tmp.display());
                Some(tmp)
            }
            Err(e) => {
                tracing::warn!("Failed to copy OpenClaw SQLite from UNC path: {}", e);
                return 0;
            }
        }
    } else {
        None
    };
    
    let effective_path = local_copy.as_ref().unwrap_or(&sqlite_file);
    
    // 使用 sqlx 读取 OpenClaw 的 SQLite 数据库
    // 尝试多种方式打开 SQLite，防止数据库被锁定
    // 1. 先尝试只读模式
    // 2. 如果失败，尝试 WAL 模式（允许读取被写的数据库）
    // 3. 如果仍失败，尝试 immutable 模式（最快的只读模式）
    let sqlite_path = effective_path.to_string_lossy();
    let connection_strings = [
        format!("sqlite:{}?mode=ro", sqlite_path),
        format!("sqlite:{}?mode=rw", sqlite_path),
        format!("sqlite:{}?mode=immutable", sqlite_path),
    ];
    
    let mut openclaw_db = None;
    for conn_str in &connection_strings {
        match sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(conn_str)
            .await 
        {
            Ok(pool) => {
                // 启用 WAL 模式以减少锁定问题
                let _ = sqlx::query("PRAGMA journal_mode=WAL")
                    .execute(&pool)
                    .await;
                openclaw_db = Some(pool);
                break;
            }
            Err(e) => {
                tracing::debug!("SQLite connect failed ({}): {}", conn_str, e);
                continue;
            }
        }
    }
    
    let openclaw_db = match openclaw_db {
        Some(pool) => pool,
        None => {
            tracing::warn!("Failed to open OpenClaw SQLite at {} (all modes failed)", sqlite_path);
            return 0;
        }
    };
    
    let mut count = 0;
    
    // OpenClaw 使用 chunks 表存储记忆（SQLite schema: id, path, source, text）
    // 尝试查询 chunks 表（OpenClaw 2026+ schema）
    let chunks_result = sqlx::query_as::<_, (String, String, String)>(
        "SELECT path, text, source FROM chunks WHERE source = 'memory'"
    )
    .fetch_all(&openclaw_db)
    .await;
    
    match chunks_result {
        Ok(rows) => {
            for (path, text, source) in rows {
                // 使用文件名作为 title
                let title = std::path::Path::new(&path)
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "openclaw-memory".to_string());
                
                let entry = MemoryEntry {
                    title,
                    content: text.clone(),
                    source: "openclaw".to_string(),
                    category: source.clone(),
                };
                
                insert_doc_entry(db, &entry, "openclaw", &path, "memory", None, None, None).await;
            }
        }
        Err(e) => {
            // 如果 chunks 表不存在，尝试旧的 memory 表
            tracing::debug!("chunks table not found, trying memory table: {}", e);
            let memory_result = sqlx::query_as::<_, (String, String, String, String, String)>(
                "SELECT id, title, content, category, created_at FROM memory WHERE deleted = 0"
            )
            .fetch_all(&openclaw_db)
            .await;
            
            match memory_result {
                Ok(rows) => {
                    for (id, title, content, category, _created_at) in rows {
                        let entry = MemoryEntry {
                            title: title.clone(),
                            content: content.clone(),
                            source: "openclaw".to_string(),
                            category: category.clone(),
                        };
                        
                        let file_path = format!("openclaw:{}", id);
                        insert_doc_entry(db, &entry, "openclaw", &file_path, "memory", None, None, None).await;
                    }
                }
                Err(e2) => {
                    tracing::warn!("Failed to query OpenClaw memory (neither chunks nor memory table): {}", e2);
                }
            }
        }
    }
    
    // 关闭连接
    openclaw_db.close().await;
    
    // 清理临时文件
    if let Some(tmp) = local_copy {
        let _ = std::fs::remove_file(&tmp);
    }
    
    count
}

/// 从 Codex 拉取 Memory
/// 注意: Codex 默认没有 memory 文件，只有 sessions
/// 这个函数查找 ~/.codex/memory/*.md 文件（如果有的话）
async fn pull_codex_memory(db: &Database, memory_dir: &PathBuf) -> i32 {
    let mut count = 0;
    // Codex memory files are in ~/.codex/memory/*.md
    // The output_path is already ~/.codex/memory, so we look for *.md files directly
    if memory_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(memory_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        let title = path.file_stem()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "untitled".to_string());
                        
                        let entry = MemoryEntry {
                            title,
                            content,
                            source: "codex".to_string(),
                            category: "memory".to_string(),
                        };
                        
                        insert_doc_entry(db, &entry, "codex", &path.to_string_lossy(), "memory", None, None, None).await;
                    }
                }
            }
        }
    }
    count
}

#[allow(dead_code)]
struct MemoryEntry {
    title: String,
    content: String,
    source: String,
    category: String,
}

/// 插入 Memory 条目到数据库
/// 插入 Memory 条目 — DEPRECATED: memory table is no longer used.
/// All data now goes to doc table via insert_doc_entry.
/// This function is kept as a no-op for backward compatibility.
async fn insert_memory_entry(_db: &Database, _entry: &MemoryEntry, _source: &str, _file_path: &str) -> bool {
    false // no-op: doc table handles all storage
}

/// 插入 Doc 条目到 doc + ext 表 (新 schema)
/// 使用确定性 ID (基于 source+path+title 的 hash) 以支持 INSERT OR IGNORE 去重
/// 内容清洗在此处统一执行
async fn insert_doc_entry(
    db: &Database,
    entry: &MemoryEntry,
    source: &str,
    file_path: &str,
    record_type: &str, // "memory" or "session"
    model: Option<&str>,
    platform: Option<&str>,
    message_count: Option<i32>,
) -> bool {
    use crate::services::content_cleaner::ContentCleaner;
    let cleaner = ContentCleaner::with_defaults();

    // B7: 清洗标题（移除token统计等）
    let cleaned_title = if record_type == "session" {
        cleaner.clean_session_title(&entry.title)
    } else {
        entry.title.clone()
    };

    // 根据类型执行不同的清洗管道
    let cleaned_content = match record_type {
        "session" => {
            // B8: 丢弃空session
            if cleaner.is_empty_session(&entry.content) {
                return false;
            }
            cleaner.clean_session(&entry.content)
        }
        "memory" => cleaner.clean_memory(&entry.content),
        _ => entry.content.clone(),
    };

    // Use full source_type (keep _session suffix for search compatibility)
    let source_type = source.to_string();

    // Generate deterministic ID from key to avoid duplicates on re-pull
    let key = format!("{}:{}:{}", source_type, file_path, cleaned_title);
    let id = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    };

    let now = Utc::now().to_rfc3339();

    // Compute content hash for change detection
    let content_hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        cleaned_content.hash(&mut h);
        format!("{:016x}", h.finish())
    };

    // Insert into doc table (INSERT OR IGNORE to avoid duplicates)
    let doc_result = if db.is_sqlite {
        sqlx::query(
            r#"INSERT OR IGNORE INTO doc (id, type, title, content, content_hash, status, quality_score, source_type, source_path, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, 'active', 0.0, ?, ?, '{}', ?, ?)"#
        )
        .bind(&id)
        .bind(record_type)
        .bind(&cleaned_title)
        .bind(&cleaned_content)
        .bind(&content_hash)
        .bind(&source_type)
        .bind(file_path)
        .bind(&now)
        .bind(&now)
        .execute(&db.pool)
        .await
    } else {
        sqlx::query(
            r#"INSERT INTO doc (id, type, title, content, content_hash, status, quality_score, source_type, source_path, metadata, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, 'active', 0.0, $6, $7, '{}', $8, $9)
               ON CONFLICT (id) DO NOTHING"#
        )
        .bind(&id)
        .bind(record_type)
        .bind(&cleaned_title)
        .bind(&cleaned_content)
        .bind(&content_hash)
        .bind(&source_type)
        .bind(file_path)
        .bind(&now)
        .bind(&now)
        .execute(&db.pool)
        .await
    };

    let is_new = match &doc_result {
        Ok(result) => result.rows_affected() > 0,
        Err(_) => false,
    };

    if !is_new {
        return false; // Already exists, skip ext insert
    }

    // Insert into ext table based on record type
    let ext_result = if record_type == "memory" {
        if db.is_sqlite {
            sqlx::query(
                r#"INSERT OR IGNORE INTO memory_ext (id, agent_id, category, owner, visibility)
                   VALUES (?, ?, ?, '', 'shared')"#
            )
            .bind(&id)
            .bind(&source_type)
            .bind(&entry.category)
            .execute(&db.pool)
            .await
        } else {
            sqlx::query(
                r#"INSERT INTO memory_ext (id, agent_id, category, owner, visibility)
                   VALUES ($1, $2, $3, '', 'shared')
                   ON CONFLICT (id) DO NOTHING"#
            )
            .bind(&id)
            .bind(&source_type)
            .bind(&entry.category)
            .execute(&db.pool)
            .await
        }
    } else {
        // session_ext
        if db.is_sqlite {
            sqlx::query(
                r#"INSERT OR IGNORE INTO session_ext (id, agent_id, model, platform, message_count)
                   VALUES (?, ?, ?, ?, ?)"#
            )
            .bind(&id)
            .bind(&source_type)
            .bind(model.unwrap_or(""))
            .bind(platform.unwrap_or(""))
            .bind(message_count.unwrap_or(0))
            .execute(&db.pool)
            .await
        } else {
            sqlx::query(
                r#"INSERT INTO session_ext (id, agent_id, model, platform, message_count)
                   VALUES ($1, $2, $3, $4, $5)
                   ON CONFLICT (id) DO NOTHING"#
            )
            .bind(&id)
            .bind(&source_type)
            .bind(model.unwrap_or(""))
            .bind(platform.unwrap_or(""))
            .bind(message_count.unwrap_or(0))
            .execute(&db.pool)
            .await
        }
    };

    if let Err(e) = ext_result {
        tracing::warn!("Failed to insert into {}_ext for doc {}: {}", record_type, id, e);
    }

    true
}

/// 从 Gemini 拉取 Memory
/// 注意: Gemini 默认没有 memory 文件，只有 sessions
/// 这个函数查找 ~/.gemini/history/*.json 文件（如果有的话）
async fn pull_gemini_memory(db: &Database, memory_dir: &PathBuf) -> i32 {
    // Gemini 使用 history 目录存储会话历史
    let history_dir = memory_dir.join("history");
    if !history_dir.exists() {
        return 0;
    }
    
    let mut count = 0;
    
    // 读取 history 目录中的 JSON 文件
    if let Ok(entries) = std::fs::read_dir(&history_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    // 尝试解析 JSON 获取会话摘要
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                        let title = data.get("title")
                            .and_then(|t| t.as_str())
                            .or_else(|| path.file_stem().and_then(|n| n.to_str()))
                            .unwrap_or("Gemini Session");
                        
                        let summary = data.get("summary")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        
                        if !summary.is_empty() {
                            let entry = MemoryEntry {
                                title: title.to_string(),
                                content: summary.to_string(),
                                source: "gemini".to_string(),
                                category: "session".to_string(),
                            };
                            
                            insert_doc_entry(db, &entry, "gemini", &path.to_string_lossy(), "memory", None, None, None).await;
                        }
                    }
                }
            }
        }
    }
    
    count
}

/// 从 Agent 拉取聊天记录
async fn pull_sessions(
    State(db): State<Database>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.scan_agents();
    
    let mut total_pulled = 0;
    let mut agent_results = Vec::new();
    let mut debug_info = Vec::new();
    
    for agent in &agents {
        if !agent.installed {
            debug_info.push(format!("{}: not installed", agent.id));
            continue;
        }
        
        let config_dir = match &agent.config_path {
            Some(p) => {
                let dir = PathBuf::from(p).parent().map(|p| p.to_path_buf()).unwrap_or_default();
                // WSL UNC 路径回退：如果 \\wsl.localhost\... 不可用，尝试 /home/...
                if !dir.exists() && dir.to_string_lossy().contains("wsl.localhost") {
                    let wsl_path = PathBuf::from("/home").join(
                        dir.components().skip_while(|c| !c.as_os_str().to_string_lossy().contains("home"))
                            .skip(1)
                            .collect::<PathBuf>()
                    );
                    if wsl_path.exists() {
                        debug_info.push(format!("{}: WSL fallback {} -> {}", agent.id, dir.display(), wsl_path.display()));
                        wsl_path
                    } else {
                        debug_info.push(format!("{}: config_dir={} (not found)", agent.id, dir.display()));
                        continue;
                    }
                } else {
                    debug_info.push(format!("{}: config_dir={}", agent.id, dir.display()));
                    dir
                }
            },
            None => {
                debug_info.push(format!("{}: no config_path", agent.id));
                continue;
            }
        };
        
        // Gemini 用 tmp/<user>/chats/ 而不是 sessions/
        // OpenClaw 用 agents/main/sessions/
        let sessions_exist = if agent.agent_type == crate::services::agent_discovery::AgentType::Gemini {
            config_dir.join("tmp").exists()
        } else if agent.agent_type == crate::services::agent_discovery::AgentType::OpenClaw {
            config_dir.join("agents").join("main").join("sessions").exists()
        } else {
            config_dir.join("sessions").exists()
        };
        debug_info.push(format!("{}: sessions_exist={}", agent.id, sessions_exist));
        
        if !sessions_exist {
            continue;
        }
        
        let count = match agent.agent_type {
            crate::services::agent_discovery::AgentType::Hermes => {
                let (c, hermes_debug) = pull_hermes_sessions(&db, &config_dir).await;
                debug_info.extend(hermes_debug);
                debug_info.push(format!("{}: hermes sessions={}", agent.id, c));
                c
            }
            crate::services::agent_discovery::AgentType::Codex => {
                let (c, codex_debug) = pull_codex_sessions(&db, &config_dir).await;
                debug_info.extend(codex_debug);
                debug_info.push(format!("{}: codex sessions={}", agent.id, c));
                c
            }
            crate::services::agent_discovery::AgentType::Gemini => {
                let (c, gemini_debug) = pull_gemini_sessions(&db, &config_dir).await;
                debug_info.extend(gemini_debug);
                debug_info.push(format!("{}: gemini sessions={}", agent.id, c));
                c
            }
            crate::services::agent_discovery::AgentType::OpenClaw => {
                let (c, openclaw_debug) = pull_openclaw_sessions(&db, &config_dir).await;
                debug_info.extend(openclaw_debug);
                debug_info.push(format!("{}: openclaw sessions={}", agent.id, c));
                c
            }
            _ => {
                debug_info.push(format!("{}: unsupported agent type", agent.id));
                0
            }
        };
        
        if count > 0 {
            total_pulled += count;
            agent_results.push(json!({
                "agent": agent.id,
                "environment": agent.environment,
                "sessions_count": count
            }));
        }
    }
    
    Json(json!({
        "success": true,
        "total_sessions": total_pulled,
        "agent_results": agent_results,
        "debug": debug_info,
        "pulled_at": Utc::now().to_rfc3339()
    }))
}

#[allow(dead_code)]
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // 找到不超过 max_bytes 的最大字符边界
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// 从 Hermes 拉取聊天记录
/// Hermes sessions 在 ~/.hermes/sessions/ 下
/// 格式1: .json 单对象 {session_id, model, messages: [{role, content}...]}
/// 格式2: .jsonl 逐行 {role, content, timestamp}
/// 实际数据: 623 .json + 8 .jsonl, 共 209 MB
async fn pull_hermes_sessions(db: &Database, config_dir: &PathBuf) -> (i32, Vec<String>) {
    let sessions_dir = config_dir.join("sessions");
    if !sessions_dir.exists() {
        return (0, vec!["sessions dir not found".to_string()]);
    }
    
    let mut count = 0;
    let mut debug = Vec::new();
    
    let entries = match std::fs::read_dir(&sessions_dir) {
        Ok(e) => e,
        Err(e) => return (0, vec![format!("read_dir failed: {}", e)]),
    };
    
    let all_entries: Vec<_> = entries.flatten().collect();
    debug.push(format!("total entries: {}", all_entries.len()));
    
    // 过滤 session 文件（排除 sessions.json 注册表和非 json/jsonl 文件）
    let mut session_files: Vec<_> = all_entries
        .into_iter()
        .filter(|e| {
            let path = e.path();
            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            // 排除 sessions.json 注册表、request_dump、checkpoint 等
            (ext == "jsonl" || ext == "json")
                && file_name != "sessions.json"
                && !file_name.starts_with("request_dump")
                && !file_name.starts_with("checkpoint")
                && file_name.starts_with("session")
        })
        .collect();
    
    debug.push(format!("filtered session_files: {}", session_files.len()));
    
    // 按修改时间排序，最新的在前
    session_files.sort_by(|a, b| {
        b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(&a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
    });
    
    // 处理所有 session 文件
    for entry in session_files.iter() {
        let path = entry.path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                debug.push(format!("read failed for {}: {}", path.display(), e));
                continue;
            }
        };
        
        let session_id = path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        
        let mut messages = Vec::new();
        let mut session_title = String::new();
        let mut session_model = String::new();
        let mut _session_platform = String::new();
        
        if ext == "json" {
            // JSON 格式：单个对象，元数据在顶层，消息在 messages 数组中
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                session_title = json.get("session_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                session_model = json.get("model")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                _session_platform = json.get("platform")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if let Some(msgs) = json.get("messages").and_then(|m| m.as_array()) {
                    for msg in msgs {
                        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
                        // content 可能是 string 或 array（tool_calls 等）
                        let msg_content = if let Some(s) = msg.get("content").and_then(|c| c.as_str()) {
                            s.to_string()
                        } else if let Some(arr) = msg.get("content").and_then(|c| c.as_array()) {
                            // 处理 content array（multimodal 等）
                            arr.iter()
                                .filter_map(|item| {
                                    item.get("text").and_then(|t| t.as_str())
                                        .or_else(|| item.get("content").and_then(|c| c.as_str()))
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            continue;
                        };
                        
                        // 跳过 system 和工具消息
                        if (role == "user" || role == "assistant") && !msg_content.is_empty() {
                            // 跳过纯工具调用消息
                            if msg_content.starts_with("tool_call:") || msg_content.starts_with("```tool") {
                                continue;
                            }
                            messages.push(format!("{}: {}", role, &msg_content));
                        }
                    }
                }
            }
        } else {
            // JSONL 格式：每行一个 JSON
            for line in content.lines() {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
                    
                    if role == "session_meta" {
                        if let Some(title) = msg.get("title").and_then(|t| t.as_str()) {
                            session_title = title.to_string();
                        }
                        if let Some(model) = msg.get("model").and_then(|m| m.as_str()) {
                            session_model = model.to_string();
                        }
                        if let Some(platform) = msg.get("platform").and_then(|p| p.as_str()) {
                            _session_platform = platform.to_string();
                        }
                    } else if role == "user" || role == "assistant" {
                        let msg_content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        if !msg_content.is_empty() && !msg_content.starts_with("tool_call:") {
                            messages.push(format!("{}: {}", role, msg_content));
                        }
                    }
                }
            }
        }
        
        if messages.is_empty() {
            continue;
        }
        
        let conversation = messages.join("\n");
        let summary = conversation;
        
        let title = if session_title.is_empty() {
            format!("Hermes Session {}", session_id)
        } else {
            session_title
        };
        
        // 构建更丰富的标题
        let enriched_title = if !session_model.is_empty() {
            format!("{} [{}]", title, session_model)
        } else {
            title
        };
        
        let entry = MemoryEntry {
            title: enriched_title,
            content: summary,
            source: "hermes_session".to_string(),
            category: "conversation".to_string(),
        };
        
        insert_doc_entry(db, &entry, "hermes_session", &path.to_string_lossy(), "session", None, None, None).await;
    }
    
    debug.push(format!("pulled: {} / {}", count, session_files.len()));
    (count, debug)
}

/// 从 Gemini 拉取聊天记录
/// Gemini session 文件在 ~/.gemini/tmp/<user>/chats/session-*.jsonl
/// 格式: JSONL, type=user 时 content 是 [{text: "..."}], type=gemini 时 content 是 string
/// Gemini 响应可能包含 thoughts[], tokens{input,output,cached}, model
/// 实际数据: 11 个 session 文件, 7.9 MB
async fn pull_gemini_sessions(db: &Database, config_dir: &PathBuf) -> (i32, Vec<String>) {
    let mut debug = Vec::new();
    let mut count = 0;
    
    // Gemini sessions 在 tmp/<username>/chats/ 下
    let tmp_dir = config_dir.join("tmp");
    if !tmp_dir.exists() {
        debug.push("gemini: tmp dir not found".to_string());
        return (0, debug);
    }
    
    let mut session_files = Vec::new();
    
    // 遍历 tmp 下的用户目录 (e.g. tmp/user/chats/)
    if let Ok(user_dirs) = std::fs::read_dir(&tmp_dir) {
        for user_entry in user_dirs.flatten() {
            let chats_dir = user_entry.path().join("chats");
            if !chats_dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&chats_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    // 只要 session-*.jsonl，排除子目录里的重复文件
                    if ext == "jsonl" && file_name.starts_with("session-") {
                        session_files.push(path);
                    }
                }
            }
        }
    }
    
    debug.push(format!("gemini: found {} session files", session_files.len()));
    
    if session_files.is_empty() {
        return (0, debug);
    }
    
    // 按修改时间排序，最新的在前
    session_files.sort_by(|a, b| {
        b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(&a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
    });
    
    for path in session_files.iter() {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                debug.push(format!("  read error: {}", e));
                continue;
            }
        };
        
        let session_id = path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let mut messages = Vec::new();
        let mut session_model = String::new();
        let mut total_tokens_in = 0i64;
        let mut total_tokens_out = 0i64;
        
        for line in content.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                
                // 跳过 $set 更新操作和 sessionId 等元数据行
                if msg_type.is_empty() || msg == serde_json::json!({}) {
                    continue;
                }
                
                match msg_type {
                    "user" => {
                        // Gemini user content 是数组: [{"text": "..."}]
                        if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<&str> = content_arr.iter()
                                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                                .collect();
                            if !text_parts.is_empty() {
                                let text = text_parts.join(" ");
                                messages.push(format!("user: {}", &text));
                            }
                        }
                    },
                    "gemini" => {
                        // Gemini assistant 响应
                        // 提取 model 信息
                        if let Some(model) = msg.get("model").and_then(|m| m.as_str()) {
                            if session_model.is_empty() {
                                session_model = model.to_string();
                            }
                        }
                        
                        // 提取 token 统计
                        if let Some(tokens) = msg.get("tokens") {
                            total_tokens_in += tokens.get("input").and_then(|t| t.as_i64()).unwrap_or(0);
                            total_tokens_out += tokens.get("output").and_then(|t| t.as_i64()).unwrap_or(0);
                        }
                        
                        // 提取 thoughts（思考过程）— 可选包含
                        let thoughts = if let Some(thoughts_arr) = msg.get("thoughts").and_then(|t| t.as_array()) {
                            let thought_texts: Vec<&str> = thoughts_arr.iter()
                                .filter_map(|t| t.get("text").and_then(|t| t.as_str()))
                                .collect();
                            if !thought_texts.is_empty() {
                                Some(format!("[思考: {}]", &thought_texts.join("; ")))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        // 提取主要回复内容
                        let content_text = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        if !content_text.is_empty() {
                            let mut response = String::new();
                            if let Some(t) = thoughts {
                                response.push_str(&t);
                                response.push(' ');
                            }
                            response.push_str(content_text);
                            messages.push(format!("gemini: {}", response));
                        }
                    },
                    _ => {
                        // 跳过 sessionId, projectHash, $set, startTime, lastUpdated 等
                    }
                }
            }
        }
        
        if messages.is_empty() {
            continue;
        }
        
        let conversation = messages.join("\n");
        let summary = conversation;
        
        // 构建标题（包含模型和 token 统计）
        let title = if !session_model.is_empty() {
            format!("Gemini Session {} [{}] ({}in/{}out tokens)", 
                session_id, session_model, total_tokens_in, total_tokens_out)
        } else {
            format!("Gemini Session {}", session_id)
        };
        
        let entry = MemoryEntry {
            title,
            content: summary,
            source: "gemini_session".to_string(),
            category: "conversation".to_string(),
        };
        
        insert_doc_entry(db, &entry, "gemini_session", &path.to_string_lossy(), "session", None, None, None).await;
    }
    
    debug.push(format!("gemini: pulled {} / {}", count, session_files.len()));
    (count, debug)
}

/// 从 Codex 拉取聊天记录
/// Codex sessions 在 ~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl
/// 格式: JSONL, 多种 event 类型
///   - session_meta: {id, cwd, originator, cli_version, model, personality, instructions}
///   - event_msg: {message: "..."} — 用户消息
///   - response_item: {role, type, content} — 助手响应
///   - turn_context: {model, approval_policy, sandbox_policy} — 模型上下文
/// 实际数据: 13 个 session 文件 (4 WSL + 9 Win), 最大 1.6 MB
async fn pull_codex_sessions(db: &Database, config_dir: &PathBuf) -> (i32, Vec<String>) {
    let mut debug = Vec::new();
    let sessions_dir = config_dir.join("sessions");
    if !sessions_dir.exists() {
        debug.push("codex: sessions dir not found".to_string());
        return (0, debug);
    }
    
    let mut count = 0;
    
    // 递归查找 rollout-*.jsonl 文件
    fn find_rollout_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    find_rollout_files(&path, files);
                } else {
                    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if ext == "jsonl" && file_name.starts_with("rollout-") {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    let mut session_files = Vec::new();
    find_rollout_files(&sessions_dir, &mut session_files);
    debug.push(format!("codex: found {} session files", session_files.len()));
    
    // 按修改时间排序
    session_files.sort_by(|a, b| {
        b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(&a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
    });
    
    for path in session_files.iter() {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                debug.push(format!("  codex read error for {}: {}", path.display(), e));
                continue;
            }
        };
        
        let session_id = path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let mut messages = Vec::new();
        let mut session_model = String::new();
        let mut session_cwd = String::new();
        
        for line in content.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let payload = msg.get("payload").unwrap_or(&msg);
                
                match msg_type {
                    "session_meta" => {
                        // 提取会话元数据
                        if let Some(model) = payload.get("model").and_then(|m| m.as_str()) {
                            session_model = model.to_string();
                        }
                        if let Some(cwd) = payload.get("cwd").and_then(|c| c.as_str()) {
                            session_cwd = cwd.to_string();
                        }
                    },
                    "event_msg" => {
                        // 用户消息
                        if let Some(message) = payload.get("message").and_then(|m| m.as_str()) {
                            if !message.is_empty() {
                                messages.push(format!("user: {}", message));
                            }
                        }
                    },
                    "response_item" => {
                        // 助手响应
                        let role = payload.get("role").and_then(|r| r.as_str()).unwrap_or("");
                        let ptype = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if ptype != "message" { continue; }
                        if role != "user" && role != "assistant" { continue; }
                        
                        let content_val = payload.get("content");
                        let text = if let Some(s) = content_val.and_then(|c| c.as_str()) {
                            s.to_string()
                        } else if let Some(arr) = content_val.and_then(|c| c.as_array()) {
                            arr.iter()
                                .filter_map(|c| {
                                    c.get("text").or(c.get("content")).and_then(|t| t.as_str())
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            continue;
                        };
                        
                        if !text.is_empty() {
                            messages.push(format!("{}: {}", role, &text));
                        }
                    },
                    _ => {
                        // 跳过 turn_context 等其他类型
                    }
                }
            }
        }
        
        if messages.is_empty() {
            continue;
        }
        
        let conversation = messages.join("\n");
        let summary = conversation;
        
        // 构建标题（包含模型和工作目录）
        let title = if !session_model.is_empty() {
            let cwd_short = session_cwd.rsplit(['/', '\\']).next().unwrap_or(&session_cwd);
            format!("Codex Session {} [{}] ({})", session_id, session_model, cwd_short)
        } else {
            format!("Codex Session {}", session_id)
        };
        
        let entry = MemoryEntry {
            title,
            content: summary,
            source: "codex_session".to_string(),
            category: "conversation".to_string(),
        };
        
        insert_doc_entry(db, &entry, "codex_session", &path.to_string_lossy(), "session", None, None, None).await;
    }
    
    debug.push(format!("codex: pulled {} / {}", count, session_files.len()));
    (count, debug)
}

/// 从 OpenClaw 拉取聊天记录
/// OpenClaw session 文件在 ~/.openclaw/agents/main/sessions/*.jsonl
/// 格式: JSONL, type=message 时 message.role=user/assistant, message.content=[{type:"text",text:"..."}]
async fn pull_openclaw_sessions(db: &Database, config_dir: &PathBuf) -> (i32, Vec<String>) {
    let mut debug = Vec::new();
    let mut count = 0;
    
    // OpenClaw sessions 在 agents/main/sessions/ 下
    let sessions_dir = config_dir.join("agents").join("main").join("sessions");
    if !sessions_dir.exists() {
        debug.push("openclaw: sessions dir not found".to_string());
        return (0, debug);
    }
    
    let mut session_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            // 只要 .jsonl，排除 deleted/reset/bak/trajectory
            if ext == "jsonl" 
                && !file_name.contains(".deleted.") 
                && !file_name.contains(".reset.") 
                && !file_name.contains(".bak-")
                && !file_name.contains(".trajectory.") 
                && file_name != "sessions.json"
            {
                session_files.push(path);
            }
        }
    }
    
    debug.push(format!("openclaw: found {} session files", session_files.len()));
    
    if session_files.is_empty() {
        return (0, debug);
    }
    
    // 按修改时间排序，最新的在前
    session_files.sort_by(|a, b| {
        b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(&a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
    });
    
    for path in session_files.iter() {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                debug.push(format!("  openclaw read error: {}", e));
                continue;
            }
        };
        
        let session_id = path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let mut messages = Vec::new();
        let mut line_count = 0;
        let mut message_count = 0;
        
        for line in content.lines() {
            line_count += 1;
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                
                if msg_type == "message" {
                    if let Some(message) = msg.get("message") {
                        let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("");
                        if role != "user" && role != "assistant" {
                            continue;
                        }
                        message_count += 1;
                        
                        // content 是数组: [{type:"text",text:"..."}, {type:"thinking",thinking:"..."}]
                        if let Some(content_arr) = message.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<&str> = content_arr.iter()
                                .filter_map(|item| {
                                    // 只取 text 类型，跳过 thinking
                                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        item.get("text").and_then(|t| t.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !text_parts.is_empty() {
                                messages.push(format!("{}: {}", role, text_parts.join(" ")));
                            }
                        }
                    }
                }
            }
        }
        
        debug.push(format!("  openclaw session {}: {} lines, {} messages, {} parsed",
            session_id, line_count, message_count, messages.len()));
        
        if messages.is_empty() {
            continue;
        }
        
        let conversation = messages.join("\n");
        let summary = conversation;
        
        let entry = MemoryEntry {
            title: format!("OpenClaw Session {}", session_id),
            content: summary,
            source: "openclaw_session".to_string(),
            category: "conversation".to_string(),
        };
        
        insert_doc_entry(db, &entry, "openclaw_session", &path.to_string_lossy(), "session", None, None, None).await;
    }
    
    debug.push(format!("openclaw: pulled {}", count));
    (count, debug)
}
