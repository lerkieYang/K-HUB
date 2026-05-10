use axum::{routing::{get, post}, Router, Json, extract::{State, Path, Query}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::Utc;
use std::path::PathBuf;
use crate::db::Database;
use crate::services::knowledge_indexer::KnowledgeIndexer;

/// 安全截断 UTF-8 字符串，不会在多字节字符中间 panic
#[allow(dead_code)]
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes { return s; }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

/// 知识库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct KnowledgeBaseConfig {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
    pub file_patterns: Vec<String>,
    pub auto_index: bool,
    pub auto_embed: bool,
    pub embedding_dir: Option<String>,
    pub last_indexed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建知识库配置请求
#[derive(Debug, Deserialize)]
pub struct CreateKnowledgeConfigRequest {
    pub name: String,
    pub paths: Vec<String>,
    pub file_patterns: Option<Vec<String>>,
    pub auto_index: Option<bool>,
    pub auto_embed: Option<bool>,
    pub embedding_dir: Option<String>,
}

/// 更新知识库配置请求
#[derive(Debug, Deserialize)]
pub struct UpdateKnowledgeConfigRequest {
    pub name: Option<String>,
    pub paths: Option<Vec<String>>,
    pub file_patterns: Option<Vec<String>>,
    pub auto_index: Option<bool>,
    pub auto_embed: Option<bool>,
    pub embedding_dir: Option<String>,
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/knowledge/config", get(list_configs).post(create_config))
        .route("/api/knowledge/config/:id", get(get_config).put(update_config).delete(delete_config))
        .route("/api/knowledge/reindex", post(reindex_all))
        .route("/api/knowledge/config/:id/reindex", post(reindex_single_config))
        .route("/api/knowledge/restore-deleted", post(restore_deleted))
        .route("/api/knowledge/light-embed", post(light_embed_all))
        .route("/api/knowledge/progress", get(get_progress))
        .route("/api/knowledge/stats", get(knowledge_stats))
        .route("/api/knowledge/tasks", get(list_tasks_handler))
         .route("/api/knowledge/tasks/:id/stop", post(stop_task_handler))
         .route("/api/knowledge/tasks/:id/force-stop", post(force_stop_task_handler))
         .route("/api/knowledge/tasks/stop-all", post(stop_all_tasks_handler))
         .route("/api/data/clear", post(clear_all_data))
         .route("/api/data/stats", get(data_stats))
         .route("/api/data/cleanup-old-versions", post(cleanup_old_versions))
}

/// 列出所有知识库配置
async fn list_configs(
    State(db): State<Database>,
) -> Json<Value> {
    let configs = sqlx::query_as::<_, (String, String, String, String, i64, i64, Option<String>, Option<String>, String, String)>(
        r#"SELECT id, name, paths, file_patterns, auto_index, auto_embed, embedding_dir, last_indexed_at, created_at, updated_at
           FROM knowledge_base_config
           ORDER BY created_at DESC"#
    )
    .fetch_all(&db.pool)
    .await;
    
    match configs {
        Ok(rows) => {
            let configs: Vec<Value> = rows.into_iter().map(|(id, name, paths, patterns, auto_index, auto_embed, emb_dir, last_indexed, created, updated)| {
                let paths_vec: Vec<String> = serde_json::from_str(&paths).unwrap_or_default();
                let patterns_vec: Vec<String> = serde_json::from_str(&patterns).unwrap_or_else(|_| vec!["*.md".to_string(), "*.txt".to_string()]);
                
                json!({
                    "id": id,
                    "name": name,
                    "paths": paths_vec,
                    "file_patterns": patterns_vec,
                    "auto_index": auto_index != 0,
                    "auto_embed": auto_embed != 0,
                    "embedding_dir": emb_dir,
                    "last_indexed_at": last_indexed,
                    "created_at": created,
                    "updated_at": updated
                })
            }).collect();
            
            Json(json!({
                "configs": configs,
                "total": configs.len()
            }))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 获取单个配置
async fn get_config(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let config = sqlx::query_as::<_, (String, String, String, String, i64, i64, Option<String>, Option<String>, String, String)>(
        r#"SELECT id, name, paths, file_patterns, auto_index, auto_embed, embedding_dir, last_indexed_at, created_at, updated_at
           FROM knowledge_base_config WHERE id = ?"#
    )
    .bind(&id)
    .fetch_one(&db.pool)
    .await;
    
    match config {
        Ok((id, name, paths, patterns, auto_index, auto_embed, emb_dir, last_indexed, created, updated)) => {
            let paths_vec: Vec<String> = serde_json::from_str(&paths).unwrap_or_default();
            let patterns_vec: Vec<String> = serde_json::from_str(&patterns).unwrap_or_else(|_| vec!["*.md".to_string()]);
            
            Json(json!({
                "id": id,
                "name": name,
                "paths": paths_vec,
                "file_patterns": patterns_vec,
                "auto_index": auto_index != 0,
                "auto_embed": auto_embed != 0,
                "embedding_dir": emb_dir,
                "last_indexed_at": last_indexed,
                "created_at": created,
                "updated_at": updated
            }))
        },
        Err(_) => Json(json!({"error": "Config not found"})),
    }
}

/// 创建配置
async fn create_config(
    State(db): State<Database>,
    Json(req): Json<CreateKnowledgeConfigRequest>,
) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let paths_json = serde_json::to_string(&req.paths).unwrap_or_else(|_| "[]".to_string());
    let patterns = req.file_patterns.unwrap_or_else(|| vec!["*.md".to_string(), "*.txt".to_string(), "*.markdown".to_string()]);
    let patterns_json = serde_json::to_string(&patterns).unwrap_or_else(|_| "[]".to_string());
    let auto_index = if req.auto_index.unwrap_or(true) { 1 } else { 0 };
    let auto_embed = if req.auto_embed.unwrap_or(false) { 1 } else { 0 };
    
    let result = sqlx::query(
        r#"INSERT INTO knowledge_base_config (id, name, paths, file_patterns, auto_index, auto_embed, embedding_dir, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&paths_json)
    .bind(&patterns_json)
    .bind(auto_index)
    .bind(auto_embed)
    .bind(&req.embedding_dir)
    .bind(&now)
    .bind(&now)
    .execute(&db.pool)
    .await;
    
    match result {
        Ok(_) => {
            // 使用统一的索引逻辑（排队执行）
            spawn_single_config_indexing(&db, id.clone(), req.name.clone(), req.paths.clone()).await;

            Json(json!({
                "success": true,
                "id": id,
                "name": req.name,
                "paths": req.paths,
                "auto_embed": req.auto_embed.unwrap_or(false),
                "queued": true
            }))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 更新配置
async fn update_config(
    State(db): State<Database>,
    Path(id): Path<String>,
    Json(req): Json<UpdateKnowledgeConfigRequest>,
) -> Json<Value> {
    let now = Utc::now().to_rfc3339();
    
    // 获取现有配置
    let existing = sqlx::query_as::<_, (String, String, String, i64, i64, Option<String>)>(
        "SELECT name, paths, file_patterns, auto_index, auto_embed, embedding_dir FROM knowledge_base_config WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&db.pool)
    .await;
    
    match existing {
        Ok((old_name, old_paths, old_patterns, old_auto_index, old_auto_embed, old_emb_dir)) => {
            let name = req.name.unwrap_or(old_name);
            let paths = req.paths.map(|p| serde_json::to_string(&p).unwrap_or_default()).unwrap_or(old_paths);
            let patterns = req.file_patterns.map(|p| serde_json::to_string(&p).unwrap_or_default()).unwrap_or(old_patterns);
            let auto_index = req.auto_index.map(|a| if a { 1 } else { 0 }).unwrap_or(old_auto_index);
            let auto_embed = req.auto_embed.map(|a| if a { 1 } else { 0 }).unwrap_or(old_auto_embed);
            let embedding_dir = req.embedding_dir.or(old_emb_dir);
            
            let result = sqlx::query(
                "UPDATE knowledge_base_config SET name = ?, paths = ?, file_patterns = ?, auto_index = ?, auto_embed = ?, embedding_dir = ?, updated_at = ? WHERE id = ?"
            )
            .bind(&name)
            .bind(&paths)
            .bind(&patterns)
            .bind(auto_index)
            .bind(auto_embed)
            .bind(&embedding_dir)
            .bind(&now)
            .bind(&id)
            .execute(&db.pool)
            .await;
            
            match result {
                Ok(_) => Json(json!({
                    "success": true,
                    "auto_embed": auto_embed != 0
                })),
                Err(e) => Json(json!({"error": e.to_string()})),
            }
        },
        Err(_) => Json(json!({"error": "Config not found"})),
    }
}

/// 删除配置（同时删除相关的 doc 和 memory 记录）
async fn delete_config(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    // 先获取配置信息
    let config = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, name, paths FROM knowledge_base_config WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&db.pool)
    .await;
    
    let _now = chrono::Utc::now().to_rfc3339();
    
    // 删除相关的 doc + memory 记录（通过 source_path/file_path 匹配）
    if let Ok(Some((_, _, paths_str))) = &config {
        if let Ok(paths) = serde_json::from_str::<Vec<String>>(paths_str) {
            for path in &paths {
                let like_pattern = format!("{}%", path);
                // Hard delete from doc table
                if let Err(e) = sqlx::query("DELETE FROM doc WHERE source_path LIKE ? AND type = 'knowledge'")
                    .bind(&like_pattern)
                    .execute(&db.pool)
                    .await
                {
                    tracing::warn!("Failed to mark doc deleted for {}: {}", like_pattern, e);
                }
                // Hard delete from knowledge_ext
                if let Err(e) = sqlx::query("DELETE FROM knowledge_ext WHERE file_path LIKE ?")
                    .bind(&like_pattern)
                    .execute(&db.pool)
                    .await
                {
                    tracing::warn!("Failed to mark knowledge_ext deleted for {}: {}", like_pattern, e);
                }

            }
        }
    }
    
    // 删除配置
    let result = sqlx::query("DELETE FROM knowledge_base_config WHERE id = ?")
        .bind(&id)
        .execute(&db.pool)
        .await;
    
    // 清除索引器状态文件，使重新添加相同文件夹时能正确索引
    let data_dir = std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let state_file = std::path::Path::new(&data_dir).join("embeddings").join("index_state.json");
    if state_file.exists() {
        let _ = std::fs::remove_file(&state_file);
        tracing::info!("Cleared indexer state file on config delete");
    }
    
    match result {
        Ok(_) => Json(json!({"success": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 恢复被删除的记录（所有类型）
async fn restore_deleted(
    State(db): State<Database>,
) -> Json<Value> {
    let now = Utc::now().to_rfc3339();
    
    // 恢复 doc 表中所有被删除的记录
    let doc_result = sqlx::query(
        "UPDATE doc SET status = 'active', updated_at = ? WHERE status = 'deleted'"
    )
    .bind(&now)
    .execute(&db.pool)
    .await;
    let doc_count = doc_result.as_ref().map(|r| r.rows_affected()).unwrap_or(0);

    Json(json!({
        "success": true,
        "restored_count": doc_count,
        "doc_restored_count": doc_count,
        "message": format!("Restored {} doc records", doc_count)
    }))
}

/// 重新索引所有知识库 — 使用队列系统
async fn reindex_all(
    State(db): State<Database>,
) -> Json<Value> {
    // 获取所有配置的路径
    let configs = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, name, paths, file_patterns, auto_embed FROM knowledge_base_config WHERE auto_index = 1"
    )
    .fetch_all(&db.pool).await;
    
    let rows = match configs {
        Ok(r) => r,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };
    
    if rows.is_empty() {
        return Json(json!({
            "success": false,
            "error": "没有配置的知识库目录，请先在设置中添加"
        }));
    }
    
    // 合并所有配置的路径
    let mut all_paths: Vec<String> = Vec::new();
    for (_, _, paths_json, _, _) in &rows {
        let paths: Vec<String> = serde_json::from_str(paths_json).unwrap_or_default();
        all_paths.extend(paths);
    }
    
    // 更新所有配置的最后索引时间
    let now = Utc::now().to_rfc3339();
    for (config_id, _, _, _, _) in &rows {
        if let Err(e) = sqlx::query("UPDATE knowledge_base_config SET last_indexed_at = ? WHERE id = ?")
            .bind(&now).bind(config_id).execute(&db.pool).await
        {
            tracing::warn!("Failed to update last_indexed_at for {}: {}", config_id, e);
        }
    }
    
    // 使用排队任务系统
    spawn_all_configs_indexing(&db, format!("索引全部知识库 ({}个配置)", rows.len()), all_paths).await;
    
    Json(json!({
        "success": true,
        "message": format!("索引任务已排队: {} 个配置", rows.len())
    }))
}

/// 重新索引单个知识库配置 — 使用与 reindex_all 相同的版本化逻辑
async fn reindex_single_config(
    State(db): State<Database>,
    Path(config_id): Path<String>,
) -> Json<Value> {
    // 查询单个配置
    let config = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, name, paths, file_patterns, auto_embed FROM knowledge_base_config WHERE id = ?"
    )
    .bind(&config_id)
    .fetch_one(&db.pool).await;

    let row = match config {
        Ok(r) => r,
        Err(e) => return Json(json!({"error": format!("Config not found: {}", e)})),
    };

    let (_, name, paths_json, _, _) = &row;
    let paths: Vec<String> = serde_json::from_str(paths_json).unwrap_or_default();

    if paths.is_empty() {
        return Json(json!({"success": false, "error": "配置没有路径"}));
    }

    // 更新 last_indexed_at
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query("UPDATE knowledge_base_config SET last_indexed_at = ? WHERE id = ?")
        .bind(&now).bind(&config_id).execute(&db.pool).await;

    spawn_single_config_indexing(&db, config_id.clone(), name.clone(), paths.clone()).await;

    Json(json!({
        "success": true,
        "message": format!("索引任务已排队: {}", name)
    }))
}

/// 共享索引核心逻辑 — 扫描、索引、版本化、双写、删除处理
async fn run_indexing_task(
    db_pool: sqlx::SqlitePool,
    paths: Vec<String>,
    cache_dir: PathBuf,
    embedding_dir: PathBuf,
    cancel_flag: crate::services::task_registry::CancelFlag,
    progress: crate::services::task_registry::ProgressHandle,
    task_id: String,
) {
    // === 阶段1: 扫描文件 ===
    progress.set_stage("scanning").await;

    // Use spawn_blocking for ALL synchronous filesystem I/O to avoid blocking the tokio runtime.
    // std::fs::read_dir / std::fs::metadata are blocking calls that starve async worker threads.
    let paths_clone = paths.clone();
    let progress_for_scan = &progress;
    let cancel_for_scan = cancel_flag.clone();
    let all_files: Vec<PathBuf> = tokio::task::spawn_blocking(move || {
        let supported_exts: std::collections::HashSet<&str> = [
            "md", "txt", "markdown", "rst", "asciidoc", "adoc",
            "json", "yaml", "yml", "toml", "csv", "tsv", "xml",
            "docx", "doc", "xlsx", "xls", "pptx", "ppt",
            "odt", "ods", "odp", "pdf",
            "html", "htm", "mhtml", "mht", "epub", "rtf",
            "ini", "cfg", "conf", "properties", "env",
            "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
            "log", "sql", "graphql", "proto",
        ].iter().copied().collect();

        let mut files: Vec<PathBuf> = Vec::new();
        for path_str in &paths_clone {
            let path_buf = PathBuf::from(path_str);
            if !path_buf.exists() { continue; }
            let mut dirs: Vec<PathBuf> = vec![path_buf];
            while let Some(dir) = dirs.pop() {
                if crate::services::task_registry::is_cancelled(&cancel_for_scan) { break; }
                if !dir.is_dir() { continue; }
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            if !name.starts_with('.') { dirs.push(p); }
                        } else if p.is_file() {
                            let ext_ok = p.extension().and_then(|e| e.to_str())
                                .map(|ext| supported_exts.contains(ext.to_lowercase().as_str())).unwrap_or(false);
                            let fname = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                            let excluded = matches!(fname.as_str(), "memory.md" | "user.md" | "memory.md.lock" | "user.md.lock");
                            if ext_ok && !excluded {
                                files.push(p);
                            }
                        }
                    }
                }
            }
        }
        files
    }).await.unwrap_or_default();

    // Update scan count after blocking scan completes (progress uses Atomics, safe from blocking thread)
    for _ in 0..all_files.len() {
        progress_for_scan.inc_scan_count();
    }

    let scan_total = all_files.len() as u64;
    progress.set_total_files(scan_total);
    tracing::info!("Scan done: {} files found (task {})", scan_total, task_id);

    if crate::services::task_registry::is_cancelled(&cancel_flag) { return; }

    // === 阶段2: 加载已有记录 + 索引器状态 ===
    progress.set_stage("indexing").await;

    let existing_records: Vec<(String, Option<String>, String, Option<String>, i64, String)> = sqlx::query_as(
        "SELECT d.id, k.file_path, d.content, d.metadata, k.version, d.created_at \
         FROM doc d LEFT JOIN knowledge_ext k ON d.id = k.id \
         WHERE d.type = 'knowledge' AND d.status = 'active' AND (k.is_current IS NULL OR k.is_current = 1)"
    ).fetch_all(&db_pool).await.unwrap_or_default();

    let mut existing_map: std::collections::HashMap<String, (String, String, Option<String>, i64, String)> = std::collections::HashMap::new();
    for (id, file_path, content, metadata, version, created_at) in existing_records {
        if let Some(fp) = file_path {
            existing_map.insert(fp, (id, content, metadata, version, created_at));
        }
    }
    tracing::info!("Found {} existing knowledge records (task {})", existing_map.len(), task_id);

    let mut indexer = KnowledgeIndexer::new(paths.clone(), embedding_dir.clone(), cache_dir);
    // 清空索引器状态文件，确保每次索引从零开始（防止旧哈希导致文件被跳过）
    let state_file = embedding_dir.join("index_state.json");
    if state_file.exists() {
        let _ = tokio::fs::remove_file(&state_file).await;
        tracing::info!("Cleared indexer state before re-indexing (task {})", task_id);
    }
    if let Err(e) = indexer.load_state().await {
        tracing::warn!("Failed to load indexer state: {}", e);
    }

    let mut inserted = 0u64;
    let mut updated = 0u64;
    let mut skipped = 0u64;
    let mut scanned_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    // === 阶段3: 逐文件索引（版本化 + 双写） ===
    for file_path in &all_files {
        if crate::services::task_registry::is_cancelled(&cancel_flag) { break; }

        let path_str = file_path.to_string_lossy().to_string();
        scanned_paths.insert(path_str.clone());

        // Use spawn_blocking for metadata check to avoid blocking the tokio runtime
        let fp_meta = file_path.clone();
        let file_size = tokio::task::spawn_blocking(move || {
            std::fs::metadata(&fp_meta).map(|m| m.len()).unwrap_or(0)
        }).await.unwrap_or(0);
        if file_size > 10 * 1024 * 1024 {
            tracing::warn!("Skipping large file ({} bytes): {}", file_size, path_str);
            skipped += 1;
            progress.set_indexed_count(inserted + updated + skipped);
            continue;
        }

        let index_result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            indexer.index_file(file_path)
        ).await;

        let memory = match index_result {
            Ok(Some(m)) => m,
            Ok(None) => { skipped += 1; progress.set_indexed_count(inserted + updated + skipped); continue; },
            Err(_) => { tracing::warn!("Timeout indexing: {}", path_str); skipped += 1; progress.set_indexed_count(inserted + updated + skipped); continue; }
        };

        let fp = match &memory.source {
            crate::models::memory::MemorySource::KnowledgeFile { file_path } => file_path.clone(),
            _ => continue,
        };

        let tags_json = serde_json::to_string(&memory.tags).unwrap_or_else(|_| "[]".to_string());
        let related_json = serde_json::to_string(&memory.related_to).unwrap_or_else(|_| "[]".to_string());
        let derived_json = serde_json::to_string(&memory.derived_from).unwrap_or_else(|_| "[]".to_string());
        let now = chrono::Utc::now().to_rfc3339();

        if let Some((old_id, old_content, _old_metadata, old_version, old_created_at)) = existing_map.get(&fp) {
            // 内容没变 → 只更新时间，不创建新版本
            if old_content == &memory.content {
                let _ = sqlx::query("UPDATE doc SET updated_at = ? WHERE id = ?")
                    .bind(&now).bind(old_id).execute(&db_pool).await;
                skipped += 1;
                progress.set_indexed_count(inserted + updated + skipped);
                continue;
            }

            // 内容变了 → 版本化更新（doc + knowledge_ext only）
            let new_id = uuid::Uuid::new_v4().to_string();
            let new_version = old_version + 1;

            // doc + knowledge_ext
            if let Err(e) = sqlx::query("UPDATE doc SET status = 'archived', updated_at = ? WHERE id = ?")
                .bind(&now).bind(old_id).execute(&db_pool).await
            { tracing::warn!("Failed to mark old doc archived: {}", e); }
            if let Err(e) = sqlx::query(
                "INSERT INTO doc (id, type, title, content, status, source_type, source_path, metadata, created_at, updated_at) VALUES (?, 'knowledge', ?, ?, 'active', 'knowledge', ?, ?, ?, ?)"
            )
            .bind(&new_id).bind(&memory.title).bind(&memory.content)
            .bind(Some(fp.as_str())).bind(&tags_json)
            .bind(old_created_at).bind(&now)
            .execute(&db_pool).await
            { tracing::warn!("Failed to insert new doc: {}", e); }
            if let Err(e) = sqlx::query("UPDATE knowledge_ext SET is_current = 0 WHERE id = ?")
                .bind(old_id).execute(&db_pool).await
            { tracing::warn!("Failed to mark old knowledge_ext: {}", e); }
            if let Err(e) = sqlx::query(
                "INSERT INTO knowledge_ext (id, file_path, version, is_current) VALUES (?, ?, ?, 1)"
            )
            .bind(&new_id).bind(Some(fp.as_str())).bind(new_version)
            .execute(&db_pool).await
            { tracing::warn!("Failed to insert new knowledge_ext: {}", e); }

            updated += 1;
        } else {
            // 新文件 → doc + knowledge_ext
            inserted += 1;

            if let Err(e) = sqlx::query(
                "INSERT INTO doc (id, type, title, content, status, source_type, source_path, metadata, created_at, updated_at) VALUES (?, 'knowledge', ?, ?, 'active', 'knowledge', ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET title=excluded.title, content=excluded.content, status='active', source_path=excluded.source_path, metadata=excluded.metadata, updated_at=excluded.updated_at"
            )
            .bind(&memory.id).bind(&memory.title).bind(&memory.content)
            .bind(Some(fp.as_str())).bind(&tags_json)
            .bind(&memory.created_at.to_rfc3339()).bind(&now)
            .execute(&db_pool).await
            { tracing::warn!("Failed to insert doc for {}: {}", fp, e); }
            if let Err(e) = sqlx::query(
                "INSERT INTO knowledge_ext (id, file_path, version, is_current) VALUES (?, ?, 1, 1) ON CONFLICT(id) DO UPDATE SET file_path=excluded.file_path, is_current=1"
            )
            .bind(&memory.id).bind(Some(fp.as_str()))
            .execute(&db_pool).await
            { tracing::warn!("Failed to insert knowledge_ext for {}: {}", fp, e); }
        }
        progress.set_indexed_count(inserted + updated + skipped);
    }

    // === 阶段4: 标记已删除的文件 ===
    let mut deleted_count = 0;
    let now = chrono::Utc::now().to_rfc3339();
    for (old_path, (old_id, _, _, _, _)) in &existing_map {
        if !scanned_paths.contains(old_path) {
            if let Err(e) = sqlx::query("UPDATE doc SET status = 'deleted', updated_at = ? WHERE id = ? AND type = 'knowledge'")
                .bind(&now).bind(old_id).execute(&db_pool).await
            { tracing::warn!("Failed to mark doc deleted: {}", e); }
            if let Err(e) = sqlx::query("UPDATE knowledge_ext SET is_current = 0 WHERE id = ?")
                .bind(old_id).execute(&db_pool).await
            { tracing::warn!("Failed to mark knowledge_ext deleted: {}", e); }
            deleted_count += 1;
        }
    }

    if let Err(e) = indexer.save_state().await {
        tracing::warn!("Failed to save indexer state: {}", e);
    }

    progress.set_stage("done").await;
    tracing::info!("Reindex done: {} inserted, {} updated, {} skipped, {} deleted (task {})", inserted, updated, skipped, deleted_count, task_id);
}

/// 全配置索引 — 供 reindex_all 使用
pub(crate) async fn spawn_all_configs_indexing(
    db: &Database,
    description: String,
    paths: Vec<String>,
) {
    // Dedup: skip if an indexing task is already running or queued
    if crate::services::task_registry::has_running_or_queued_task("indexing").await {
        tracing::info!("Indexing task already running or queued, skipping duplicate spawn_all_configs_indexing");
        return;
    }

    let db_pool = db.pool.clone();
    let cache_dir = PathBuf::from(std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("cache");
    let embedding_dir = PathBuf::from(std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("embeddings");

    tokio::spawn(async move {
        let (task_id, cancel_flag, progress) = crate::services::task_registry::register_queued_task("indexing", &description).await;
        tracing::info!("Reindex all queued: {} (task {})", description, task_id);

        let can_execute = crate::services::task_registry::wait_for_turn(&task_id).await;
        if !can_execute { tracing::info!("Task {} cancelled while waiting", task_id); return; }

        run_indexing_task(db_pool, paths, cache_dir, embedding_dir, cancel_flag, progress, task_id.clone()).await;

        crate::services::task_registry::unregister_task(&task_id).await;
    });
}

/// 单配置索引 — 供 create_config、reindex_single_config 和 file_watcher 使用
pub(crate) async fn spawn_single_config_indexing(
    db: &Database,
    config_id: String,
    config_name: String,
    paths: Vec<String>,
) {
    // Dedup: skip if an indexing task is already running or queued
    if crate::services::task_registry::has_running_or_queued_task("indexing").await {
        tracing::info!("Indexing task already running or queued, skipping duplicate spawn_single_config_indexing for '{}'", config_name);
        return;
    }

    let db_pool = db.pool.clone();
    let cache_dir = PathBuf::from(std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("cache");
    let embedding_dir = PathBuf::from(std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("embeddings");

    tokio::spawn(async move {
        let (task_id, cancel_flag, progress) = crate::services::task_registry::register_queued_task("indexing", &config_name).await;
        tracing::info!("Reindex single config queued: {} (task {})", config_id, task_id);

        let can_execute = crate::services::task_registry::wait_for_turn(&task_id).await;
        if !can_execute { tracing::info!("Task {} cancelled while waiting", task_id); return; }

        run_indexing_task(db_pool, paths, cache_dir, embedding_dir, cancel_flag, progress, task_id.clone()).await;

        crate::services::task_registry::unregister_task(&task_id).await;
    });
}

/// 从 ai_config.json 创建 ChatService
#[allow(dead_code)]
fn create_chat_service_from_config(path: &std::path::Path) -> Option<crate::services::ai_service::ChatService> {
    if !path.exists() { return None; }
    std::fs::read_to_string(path).ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|config| {
            let model = config.get("chat_model").and_then(|v| v.as_str()).unwrap_or("");
            let provider = config.get("chat_provider").and_then(|v| v.as_str()).unwrap_or("openai");
            let api_key = config.get("chat_api_key").and_then(|v| v.as_str()).unwrap_or("");
            let base_url = config.get("chat_base_url").and_then(|v| v.as_str()).unwrap_or("");
            
            if model.is_empty() || api_key.is_empty() { return None; }
            
            let p = crate::services::ai_service::AIProvider::from_str(provider);
            let url = if base_url.is_empty() { p.default_base_url().to_string() } else { base_url.to_string() };
            Some(crate::services::ai_service::ChatService::new(crate::services::ai_service::ChatConfig {
                provider: p, api_key: api_key.to_string(), base_url: url, model: model.to_string(),
            }))
        })
}

/// 知识库统计
async fn knowledge_stats(
    State(db): State<Database>,
) -> Json<Value> {
    // 统计知识库 doc 数量（直接查 doc 表，不依赖 knowledge_ext）
    let memory_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active'"
    )
    .fetch_one(&db.pool)
    .await
    .unwrap_or(0);

    let embedded_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    )
    .fetch_one(&db.pool)
    .await
    .unwrap_or(0);
    
    Json(json!({
        "total_memories": memory_count,
        "embedded_count": embedded_count,
        "pending_embedding": memory_count - embedded_count
    }))
}

/// 获取知识库索引和向量化进度
async fn get_progress(
    State(db): State<Database>,
) -> Json<Value> {
    // 先检查是否有运行中的任务（有实时进度）
    let active_progress = crate::services::task_registry::get_active_progress().await;
    
    // 始终从数据库查询索引数据（不受向量化任务影响）
    let db_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_ext WHERE is_current = 1"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    
    let db_indexed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    
    let db_vectorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    
    let (stage, scan_count, total_files, indexed_count, vectorized_count, index_progress, vectorize_progress) = 
    if let Some((stage, scan, task_total, task_indexed, task_vectorized)) = active_progress {
        // 索引进度始终用数据库值（不受向量化任务影响）
        let idx_pct = if db_total > 0 { (db_indexed as f64 / db_total as f64 * 100.0).min(100.0) } else { 0.0 };
        if stage == "vectorizing" || stage.starts_with("vectorizing") {
            // 向量化任务：向量化进度用任务的 vectorized_count
            let vec_pct = if task_total > 0 {
                (task_vectorized as f64 / task_total as f64 * 100.0).min(100.0)
            } else { 0.0 };
            ("vectorizing".to_string(), scan, db_total as u64, db_indexed as u64, db_vectorized as i64, idx_pct, vec_pct)
        } else {
            // 索引任务：向量化进度用数据库值
            let vec_pct = if db_total > 0 { (db_vectorized as f64 / db_total as f64 * 100.0).min(100.0) } else { 0.0 };
            (stage, scan, task_total, task_indexed, db_vectorized, idx_pct, vec_pct)
        }
    } else {
        // 没有任务，用数据库值
        let idx_pct = if db_total > 0 { (db_indexed as f64 / db_total as f64 * 100.0).min(100.0) } else { 0.0 };
        let vec_pct = if db_total > 0 { (db_vectorized as f64 / db_total as f64 * 100.0).min(100.0) } else { 0.0 };
        ("idle".to_string(), 0, db_total as u64, db_indexed as u64, db_vectorized, idx_pct, vec_pct)
    };
    
    // 从 config 表获取配置数
    let configs_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM knowledge_base_config")
        .fetch_one(&db.pool).await.unwrap_or(0);

    let last_indexed: Option<String> = sqlx::query_scalar("SELECT MAX(updated_at) FROM doc WHERE type = 'knowledge'")
        .fetch_one(&db.pool).await.ok();
    
    // 进度已在上面计算好，直接使用
    Json(json!({
        "stage": stage,
        "scan_count": scan_count,
        "total_files": total_files,
        "indexed_count": indexed_count,
        "vectorized_count": vectorized_count,
        "index_progress": (index_progress * 100.0).round() / 100.0,
        "vectorize_progress": (vectorize_progress * 100.0).round() / 100.0,
        "configs_count": configs_count,
        "last_indexed": last_indexed
    }))
}

/// 列出所有运行中的任务
async fn list_tasks_handler() -> Json<Value> {
    let tasks = crate::services::task_registry::list_tasks().await;
    Json(tasks)
}

 /// 停止指定任务
 async fn stop_task_handler(
     Path(task_id): Path<String>,
 ) -> Json<Value> {
     match crate::services::task_registry::stop_task(&task_id).await {
         Ok(msg) => Json(json!({"success": true, "message": msg})),
         Err(msg) => Json(json!({"success": false, "error": msg})),
     }
 }
 
 /// 强制停止并移除卡死的任务
 async fn force_stop_task_handler(
     Path(task_id): Path<String>,
 ) -> Json<Value> {
     match crate::services::task_registry::force_stop_task(&task_id).await {
         Ok(msg) => Json(json!({"success": true, "message": msg})),
         Err(msg) => Json(json!({"success": false, "error": msg})),
     }
 }
 
 /// 停止所有任务
async fn stop_all_tasks_handler(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let task_type = params.get("type").map(|s| s.as_str());
    let count = crate::services::task_registry::stop_all_tasks(task_type).await;
    Json(json!({
        "success": true,
        "stopped": count,
        "message": format!("Marked {} tasks for cancellation", count)
    }))
}

/// 轻 embedding：对记录用 Chat 模型提取关键词/标签
/// 支持 type 参数: "knowledge", "memory"(hermes/codex/gemini/openclaw), "session"(*_session), 或具体source_type
async fn light_embed_all(
    State(db): State<Database>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let type_filter = params.get("type").map(|s| s.as_str());
    
    // 将 type 参数展开为实际的 source_type 列表
    let source_types: Vec<&str> = match type_filter {
        Some("knowledge") => vec!["knowledge"],
        Some("memory") => vec!["hermes", "codex", "gemini", "openclaw"],
        Some("session") => vec!["hermes_session", "codex_session", "gemini_session", "openclaw_session"],
        Some(t) => vec![t], // 单个具体 source_type
        None => vec![], // 空 = 所有
    };
    
    // 读取 AI 配置
    let ai_config_path = std::path::Path::new(&std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string())).join("ai_config.json");
    let chat_service = if ai_config_path.exists() {
        std::fs::read_to_string(ai_config_path).ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .and_then(|config| {
                let chat_model = config.get("chat_model").and_then(|v| v.as_str()).unwrap_or("");
                let chat_provider = config.get("chat_provider").and_then(|v| v.as_str()).unwrap_or("openai");
                let chat_api_key = config.get("chat_api_key").and_then(|v| v.as_str()).unwrap_or("");
                let chat_base_url = config.get("chat_base_url").and_then(|v| v.as_str()).unwrap_or("");
                
                if chat_model.is_empty() || chat_api_key.is_empty() {
                    return None;
                }
                
                let provider = crate::services::ai_service::AIProvider::from_str(chat_provider);
                let base_url = if chat_base_url.is_empty() {
                    provider.default_base_url().to_string()
                } else {
                    chat_base_url.to_string()
                };
                Some(crate::services::ai_service::ChatService::new(
                    crate::services::ai_service::ChatConfig {
                        provider,
                        api_key: chat_api_key.to_string(),
                        base_url,
                        model: chat_model.to_string(),
                    }
                ))
            })
    } else {
        None
    };
    
    let chat = match chat_service {
        Some(cs) => cs,
        None => {
            return Json(json!({
                "success": false,
                "error": "AI 配置不完整，请先在设置中配置 Chat 模型"
            }));
        }
    };
    
    // 查询待处理记录 — 从 doc 表查询，按 type 过滤
    let records: Vec<(String, String, String)> = match type_filter {
        Some("knowledge") => {
            let sql = "SELECT d.id, d.title, d.content FROM doc d LEFT JOIN knowledge_ext k ON d.id = k.id WHERE d.type = 'knowledge' AND d.status = 'active' AND (k.is_current IS NULL OR k.is_current = 1) AND (d.metadata IS NULL OR d.metadata = '{}' OR d.metadata = '' OR d.metadata = '[]')";
            sqlx::query_as::<_, (String, String, String)>(sql)
            .fetch_all(&db.pool).await.unwrap_or_default()
        },
        Some("memory") | Some("session") => {
            let doc_type = type_filter.unwrap();
            let sql = format!("SELECT d.id, d.title, d.content FROM doc d WHERE d.type = '{}' AND d.status = 'active' AND (d.metadata IS NULL OR d.metadata = '{{}}' OR d.metadata = '' OR d.metadata = '[]')", doc_type);
            sqlx::query_as::<_, (String, String, String)>(&sql)
            .fetch_all(&db.pool).await.unwrap_or_default()
        },
        Some(t) => {
            let sql = "SELECT d.id, d.title, d.content FROM doc d WHERE d.source_type = ? AND d.status = 'active' AND (d.metadata IS NULL OR d.metadata = '{}' OR d.metadata = '' OR d.metadata = '[]')";
            sqlx::query_as::<_, (String, String, String)>(sql)
            .bind(t)
            .fetch_all(&db.pool).await.unwrap_or_default()
        },
        None => {
            let sql = "SELECT d.id, d.title, d.content FROM doc d WHERE d.status = 'active' AND (d.metadata IS NULL OR d.metadata = '{}' OR d.metadata = '' OR d.metadata = '[]')";
            sqlx::query_as::<_, (String, String, String)>(sql)
            .fetch_all(&db.pool).await.unwrap_or_default()
        },
    };
    
    if records.is_empty() {
        return Json(json!({
            "success": true,
            "processed": 0,
            "message": format!("所有 {:?} 记录已有标签，无需处理", type_filter.unwrap_or("全部"))
        }));
    }
    
    // 注册任务 — 按类型区分
    let type_label = type_filter.unwrap_or("全部");
    let task_type = match type_filter {
        Some("knowledge") => "vectorizing_knowledge",
        Some("memory") => "vectorizing_memory",
        Some("session") => "vectorizing_session",
        _ => "vectorizing",
    };
    // Dedup: skip if a vectorizing task of the same type is already running or queued
    if crate::services::task_registry::has_running_or_queued_task(task_type).await {
        return Json(json!({
            "success": true,
            "total": 0,
            "message": format!("{}向量化任务已在运行或排队中，跳过重复启动", type_label)
        }));
    }
    // Use queued task + wait_for_turn to prevent multiple vectorizing tasks from running simultaneously
    let (task_id, cancel_flag, progress) = crate::services::task_registry::register_queued_task(
        task_type,
        &format!("向量化{} ({}条记录)", type_label, records.len())
    ).await;
    
    let total = records.len();
    let db_pool = db.pool.clone();
    
    // 初始化进度

    // Spawn 背景任务
    tokio::spawn(async move {
        // Wait for our turn in the queue — prevents multiple vectorizing tasks running at once
        let can_execute = crate::services::task_registry::wait_for_turn(&task_id).await;
        if !can_execute { tracing::info!("Vectorize task {} cancelled while waiting", task_id); return; }

        progress.set_stage("vectorizing").await;
        progress.set_total_files(total as u64);

        let mut processed = 0;
        let mut failed = 0;
        
        // 并发处理，最多 5 个并发请求
        let concurrency = 5usize;
        let mut iter = records.into_iter();
        let mut in_flight: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = (String, Result<String, String>)> + Send>>> = Vec::new();
        let chat = std::sync::Arc::new(chat);
        
        // 启动前 concurrency 个任务
        for _ in 0..concurrency {
            if let Some((id, title, content)) = iter.next() {
                let chat_clone = chat.clone();
                in_flight.push(Box::pin(async move {
                    let result = chat_clone.extract_keywords(&title, &content, 0).await;
                    (id, result)
                }));
            }
        }
        
        while !in_flight.is_empty() {
            // 等待任意一个完成
            let (result, _index, remaining) = futures::future::select_all(in_flight).await;
            in_flight = remaining;
            
            // 检查取消
            if crate::services::task_registry::is_cancelled(&cancel_flag) {
                tracing::info!("Light embed task {} cancelled", task_id);
                break;
            }
            
            match result {
                (id, Ok(ai_keywords)) => {
                    let ai_tags: Vec<String> = ai_keywords
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    if !ai_tags.is_empty() {
                        let tags_json = serde_json::to_string(&ai_tags).unwrap_or_else(|_| "[]".to_string());
                        let now = chrono::Utc::now().to_rfc3339();

                        let _ = sqlx::query(
                            "UPDATE doc SET metadata = ?, updated_at = ? WHERE id = ?"
                        )
                        .bind(&tags_json)
                        .bind(&now)
                        .bind(&id)
                        .execute(&db_pool)
                        .await;

                        processed += 1;
                        progress.inc_vectorized_count();
                    }
                    progress.inc_indexed_count();
                }
                (id, Err(e)) => {
                    tracing::warn!("Failed to extract keywords for {}: {}", id, e);
                    failed += 1;
                    progress.inc_indexed_count();
                }
            }
            
            // 补充新任务
            if let Some((id, title, content)) = iter.next() {
                let chat_clone = chat.clone();
                in_flight.push(Box::pin(async move {
                    let result = chat_clone.extract_keywords(&title, &content, 0).await;
                    (id, result)
                }));
            }
        }
        
        tracing::info!("Light embed complete: {}/{} processed, {} failed", processed, total, failed);
        crate::services::task_registry::unregister_task(&task_id).await;
    });
    
    Json(json!({
        "success": true,
        "total": total,
        "message": format!("开始对 {} 条记录提取关键词", total)
    }))
}

/// 数据统计
async fn data_stats(
    State(db): State<Database>,
) -> Json<Value> {
    let data_dir = std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    
    // 统计各表记录数 (from doc table)
    let memory_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let memory_active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let memory_deleted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'deleted'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let memory_archived: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'archived'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let memory_candidate: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'candidate'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let knowledge_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'session' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);

    // Session by_source distribution
    let session_by_source: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source_type, COUNT(*) FROM doc WHERE type = 'session' AND status = 'active' GROUP BY source_type"
    ).fetch_all(&db.pool).await.unwrap_or_default();
    let memory_only_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let artifact_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifact")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let document_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let config_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_base_config")
        .fetch_one(&db.pool).await.unwrap_or(0);
    
    // doc table stats
    let doc_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let doc_knowledge: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    let doc_active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE status = 'active'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    
    // 向量化统计（metadata 有值即为已向量化）
    let total_vectorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    let knowledge_vectorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE type = 'knowledge' AND status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    let memory_vectorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE type = 'memory' AND status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    let session_vectorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM doc WHERE type = 'session' AND status = 'active' AND metadata IS NOT NULL AND metadata != '' AND metadata != '{}' AND metadata != '[]'"
    ).fetch_one(&db.pool).await.unwrap_or(0);
    let knowledge_ext_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_ext")
        .fetch_one(&db.pool).await.unwrap_or(0);
    
    // 计算缓存目录大小
    let cache_dir = std::path::Path::new(&data_dir).join("cache");
    let embedding_dir = std::path::Path::new(&data_dir).join("embeddings");
    let cache_size = dir_size(&cache_dir);
    let embedding_size = dir_size(&embedding_dir);
    
    // DB 文件大小
    let db_path = std::path::Path::new(&data_dir).join("knowledge-hub.db");
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    
    // 旧版本数 = archived 状态的记录
    let all_archived: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE status = 'archived'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    // 已删除数 = 所有类型的 deleted 记录
    let all_deleted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM doc WHERE status = 'deleted'")
        .fetch_one(&db.pool).await.unwrap_or(0);
    
    Json(json!({
        "memory": {
            "total": memory_total,
            "active": memory_active,
            "deleted": memory_deleted,
            "archived": memory_archived,
            "candidate": memory_candidate,
            "knowledge": knowledge_count,
            "count": memory_only_count
        },
        "sessions": {
            "count": session_count,
            "total": session_count,
            "by_source": session_by_source.iter().map(|(source, count)| {
                serde_json::json!({"source": source, "count": count})
            }).collect::<Vec<_>>()
        },
        "doc": {
            "total": doc_total,
            "knowledge": doc_knowledge,
            "active": doc_active,
            "total_vectorized": total_vectorized,
            "knowledge_vectorized": knowledge_vectorized,
            "memory_vectorized": memory_vectorized,
            "session_vectorized": session_vectorized,
            "all_deleted": all_deleted,
            "all_archived": all_archived
        },
        "knowledge_ext_count": knowledge_ext_count,
        "artifact_count": artifact_count,
        "document_count": document_count,
        "config_count": config_count,
        "cache_size_bytes": cache_size,
        "cache_size_mb": (cache_size as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0,
        "embedding_size_bytes": embedding_size,
        "embedding_size_mb": (embedding_size as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0,
        "db_size_bytes": db_size,
        "db_size_mb": (db_size as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0,
        "data_dir": data_dir
    }))
}

 /// 清理旧版本记录（is_current = 0）— from knowledge_ext
 async fn cleanup_old_versions(
     State(db): State<Database>,
 ) -> Json<Value> {
     // Count old versions (from knowledge_ext)
     let count: i64 = sqlx::query_scalar(
         "SELECT COUNT(*) FROM knowledge_ext WHERE is_current = 0"
     ).fetch_one(&db.pool).await.unwrap_or(0);
     
     // Delete old version ext records
     let result = sqlx::query("DELETE FROM knowledge_ext WHERE is_current = 0")
         .execute(&db.pool).await;
     
     // Also clean up corresponding orphaned doc records
     let _ = sqlx::query(
         "DELETE FROM doc WHERE type = 'knowledge' AND id NOT IN (SELECT id FROM knowledge_ext)"
     ).execute(&db.pool).await;
     
     match result {
         Ok(r) => {
             // VACUUM to reclaim space
             let _ = sqlx::query("VACUUM").execute(&db.pool).await;
             Json(json!({
                 "success": true,
                 "deleted": count,
                 "rows_affected": r.rows_affected(),
                 "message": format!("已清理 {} 条旧版本记录", count)
             }))
         }
         Err(e) => Json(json!({
             "success": false,
             "error": e.to_string()
         }))
     }
 }

 /// 清除所有数据
#[derive(Debug, Deserialize)]
struct ClearDataRequest {
    clear_memory: Option<bool>,
    clear_sessions: Option<bool>,
    clear_knowledge: Option<bool>,
    clear_artifacts: Option<bool>,
    clear_documents: Option<bool>,
    clear_cache: Option<bool>,
    clear_embeddings: Option<bool>,
    clear_configs: Option<bool>,
    clear_all: Option<bool>,
    clear_deleted: Option<bool>,
    clear_memory_metadata: Option<bool>,
    clear_session_metadata: Option<bool>,
    clear_knowledge_metadata: Option<bool>,
    confirmation: Option<String>,
}

async fn clear_all_data(
    State(db): State<Database>,
    Json(req): Json<ClearDataRequest>,
) -> Json<Value> {
    // 安全确认：防止误操作
    if req.confirmation.as_deref() != Some("CONFIRM_CLEAR") {
        return Json(json!({
            "success": false,
            "error": "Missing or invalid confirmation. Set confirmation=\"CONFIRM_CLEAR\" to proceed."
        }));
    }
    let data_dir = std::env::var("KH_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let mut cleared = Vec::new();
    
    let clear_all = req.clear_all.unwrap_or(false);
    
    // 清除已删除的记录（永久删除）
    if req.clear_deleted.unwrap_or(false) {
        let r_doc = sqlx::query("DELETE FROM doc WHERE status = 'deleted'").execute(&db.pool).await;
        if let Ok(result) = r_doc {
            cleared.push(format!("deleted from doc: {} rows", result.rows_affected()));
        }
        let _ = sqlx::query("VACUUM").execute(&db.pool).await;
        return Json(json!({
            "success": true,
            "cleared": cleared,
            "message": format!("Cleared {} deleted records", cleared.len())
        }));
    }

    // 清除特定类型的向量化数据 (from doc table only)
    if req.clear_memory_metadata.unwrap_or(false) {
        let r = sqlx::query("UPDATE doc SET metadata = NULL, embedding = NULL WHERE type = 'memory'").execute(&db.pool).await;
        if let Ok(result) = r { cleared.push(format!("memory metadata: {} rows", result.rows_affected())); }
    }
    if req.clear_session_metadata.unwrap_or(false) {
        let r = sqlx::query("UPDATE doc SET metadata = NULL, embedding = NULL WHERE type = 'session'").execute(&db.pool).await;
        if let Ok(result) = r { cleared.push(format!("session metadata: {} rows", result.rows_affected())); }
    }
    if req.clear_knowledge_metadata.unwrap_or(false) {
        let r = sqlx::query("UPDATE doc SET metadata = NULL, embedding = NULL WHERE type = 'knowledge'").execute(&db.pool).await;
        if let Ok(result) = r { cleared.push(format!("knowledge metadata: {} rows", result.rows_affected())); }
    }

    // 先停止所有运行中的任务，防止清除后又被写回
    let stopped = crate::services::task_registry::stop_all_tasks(None).await;
    if stopped > 0 {
        cleared.push(format!("stopped {} running tasks", stopped));
        // 等一下让任务退出
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    
    // 清除记忆 (memory only) — from doc table
    if clear_all || req.clear_memory.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM doc WHERE type = 'memory'").execute(&db.pool).await;
        match r {
            Ok(result) => {
                cleared.push(format!("memory from doc: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear memory: {}", e)})),
        }
        // 清除 memory_ext
        let r_ext = sqlx::query("DELETE FROM memory_ext").execute(&db.pool).await;
        if let Ok(result) = r_ext {
            cleared.push(format!("memory_ext: {} rows", result.rows_affected()));
        }
    }
    
    // 清除会话 (sessions) — from doc table
    if clear_all || req.clear_sessions.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM doc WHERE type = 'session'").execute(&db.pool).await;
        match r {
            Ok(result) => {
                cleared.push(format!("sessions from doc: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear sessions: {}", e)})),
        }
        // 清除 session_ext 表
        let r_ext = sqlx::query("DELETE FROM session_ext").execute(&db.pool).await;
        match r_ext {
            Ok(result) => {
                cleared.push(format!("session_ext: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear session_ext: {}", e)})),
        }
    }
    
    // 清除知识库 (knowledge) — from doc table
    if clear_all || req.clear_knowledge.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM doc WHERE type = 'knowledge'").execute(&db.pool).await;
        match r {
            Ok(result) => {
                cleared.push(format!("knowledge from doc: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear knowledge: {}", e)})),
        }
        // 清除 knowledge_ext 表
        let r_ext = sqlx::query("DELETE FROM knowledge_ext").execute(&db.pool).await;
        match r_ext {
            Ok(result) => {
                cleared.push(format!("knowledge_ext: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear knowledge_ext: {}", e)})),
        }
        // 清除 knowledge_base_config 表
        let r_config = sqlx::query("DELETE FROM knowledge_base_config").execute(&db.pool).await;
        match r_config {
            Ok(result) => {
                cleared.push(format!("knowledge_base_config: {} rows", result.rows_affected()));
            },
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear knowledge_base_config: {}", e)})),
        }
        // 清除索引器状态文件
        let state_file = std::path::Path::new(&data_dir).join("embeddings").join("index_state.json");
        if state_file.exists() {
            let _ = std::fs::remove_file(&state_file);
            cleared.push("indexer state cleared".to_string());
        }
    }
    
    // 清除 artifacts
    if clear_all || req.clear_artifacts.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM artifact").execute(&db.pool).await;
        match r {
            Ok(result) => cleared.push(format!("artifact: {} rows", result.rows_affected())),
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear artifacts: {}", e)})),
        }
    }
    
    // 清除 documents
    if clear_all || req.clear_documents.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM document").execute(&db.pool).await;
        match r {
            Ok(result) => cleared.push(format!("document: {} rows", result.rows_affected())),
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear documents: {}", e)})),
        }
    }
    
    // 清除知识库配置
    if clear_all || req.clear_configs.unwrap_or(false) {
        let r = sqlx::query("DELETE FROM knowledge_base_config").execute(&db.pool).await;
        match r {
            Ok(result) => cleared.push(format!("config: {} rows", result.rows_affected())),
            Err(e) => return Json(json!({"success": false, "error": format!("Failed to clear configs: {}", e)})),
        }
    }
    
    // VACUUM 回收空间 — 在任何数据清除操作后执行
    if clear_all || req.clear_memory.unwrap_or(false) || req.clear_sessions.unwrap_or(false) 
        || req.clear_knowledge.unwrap_or(false) || req.clear_embeddings.unwrap_or(false)
        || req.clear_deleted.unwrap_or(false) {
        let _ = sqlx::query("VACUUM").execute(&db.pool).await;
    }
    
    // 清除缓存目录
    if clear_all || req.clear_cache.unwrap_or(false) {
        let cache_dir = std::path::Path::new(&data_dir).join("cache");
        if cache_dir.exists() {
            match std::fs::remove_dir_all(&cache_dir) {
                Ok(_) => cleared.push("cache directory cleared".to_string()),
                Err(e) => cleared.push(format!("cache clear failed: {}", e)),
            }
        } else {
            cleared.push("cache directory not found".to_string());
        }
    }
    
    // 清除 embedding 目录
    if clear_all || req.clear_embeddings.unwrap_or(false) {
        let embedding_dir = std::path::Path::new(&data_dir).join("embeddings");
        if embedding_dir.exists() {
            match std::fs::remove_dir_all(&embedding_dir) {
                Ok(_) => cleared.push("embedding directory cleared".to_string()),
                Err(e) => cleared.push(format!("embedding clear failed: {}", e)),
            }
        } else {
            cleared.push("embedding directory not found".to_string());
        }
        // 同时清除 doc 表中的 embedding 字段和 metadata，使向量化进度清零
        let r = sqlx::query("UPDATE doc SET embedding = NULL, metadata = NULL")
            .execute(&db.pool).await;
        match r {
            Ok(result) => {
                cleared.push(format!("doc embedding cleared: {} rows", result.rows_affected()));
            },
            Err(e) => cleared.push(format!("doc embedding clear failed: {}", e)),
        }
    }
    
    if cleared.is_empty() {
        return Json(json!({"success": false, "error": "No data type selected for clearing"}));
    }
    
    // clear_all 时额外清理所有相关表 + VACUUM
    if clear_all {
        let _ = sqlx::query("DELETE FROM memory_candidate").execute(&db.pool).await;
        let _ = sqlx::query("DELETE FROM event_log").execute(&db.pool).await;
        let _ = sqlx::query("DELETE FROM source_file").execute(&db.pool).await;
        let _ = sqlx::query("DELETE FROM workspace").execute(&db.pool).await;
        // Ensure doc + knowledge_ext are also cleared in clear_all mode
        let _ = sqlx::query("DELETE FROM doc").execute(&db.pool).await;
        let _ = sqlx::query("DELETE FROM knowledge_ext").execute(&db.pool).await;
        // VACUUM 回收空间
        let _ = sqlx::query("VACUUM").execute(&db.pool).await;
        cleared.push("all auxiliary tables cleared + VACUUM".to_string());
    }
    
    Json(json!({
        "success": true,
        "cleared": cleared,
        "message": format!("Cleared {} data types", cleared.len())
    }))
}

/// 计算目录大小
fn dir_size(path: &std::path::Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                size += dir_size(&p);
            } else {
                size += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    size
}
