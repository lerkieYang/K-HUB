use axum::{routing::{get, post}, Router, Json, extract::State};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::data_source::CreateDataSourceRequest;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/data-sources", get(list_data_sources).post(create_data_source))
        .route("/api/data-sources/:id", get(get_data_source).put(update_data_source).delete(delete_data_source))
        .route("/api/data-sources/:id/scan", post(scan_data_source))
}

async fn list_data_sources(
    State(db): State<Database>,
) -> Json<Value> {
    // Schema: id, workspace_id, name, source_type, path, url, config, include_globs, exclude_globs, auto_index, last_synced_at, status, created_at, updated_at
    let sources =
        sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
            "SELECT id, name, source_type, path, status, include_globs, created_at FROM data_source ORDER BY created_at DESC"
        )
        .fetch_all(&db.pool)
        .await;
    
    match sources {
        Ok(rows) => {
            let sources: Vec<Value> = rows.into_iter().map(|(id, name, source_type, path, status, include_globs, created_at)| {
                json!({
                    "id": id,
                    "name": name,
                    "source_type": source_type,
                    "path": path,
                    "status": status,
                    "include_globs": include_globs,
                    "created_at": created_at
                })
            }).collect();
            Json(json!({"data_sources": sources, "total": sources.len()}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn create_data_source(
    State(db): State<Database>,
    Json(req): Json<CreateDataSourceRequest>,
) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let include_globs = serde_json::to_string(&req.include_globs.unwrap_or_default()).unwrap_or_else(|_| "[\"*\"]".to_string());
    let exclude_globs = serde_json::to_string(&req.exclude_globs.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
    
    // Schema: id, workspace_id, name, source_type, path, url, config, include_globs, exclude_globs, auto_index, last_synced_at, status, created_at, updated_at
    let result =
        sqlx::query("INSERT INTO data_source (id, workspace_id, name, path, source_type, include_globs, exclude_globs, auto_index, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, ?, ?, 1, 'active', ?, ?)")
            .bind(&id)
            .bind(&req.name)
            .bind(&req.path)
            .bind(&req.source_type)
            .bind(&include_globs)
            .bind(&exclude_globs)
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({
            "id": id,
            "name": req.name,
            "path": req.path,
            "source_type": req.source_type
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_data_source(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let source =
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id, name, path, source_type, include_globs, exclude_globs, status, created_at FROM data_source WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&db.pool)
        .await;
    
    match source {
        Ok((id, name, path, source_type, include_globs, exclude_globs, status, created_at)) => Json(json!({
            "id": id,
            "name": name,
            "path": path,
            "source_type": source_type,
            "include_globs": serde_json::from_str::<Value>(&include_globs).unwrap_or_default(),
            "exclude_globs": serde_json::from_str::<Value>(&exclude_globs).unwrap_or_default(),
            "status": status,
            "created_at": created_at
        })),
        Err(_) => Json(json!({"error": "Data source not found"})),
    }
}

async fn update_data_source(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<CreateDataSourceRequest>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let include_globs = serde_json::to_string(&req.include_globs.unwrap_or_default()).unwrap_or_else(|_| "[\"*\"]".to_string());
    let exclude_globs = serde_json::to_string(&req.exclude_globs.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
    
    let result =
        sqlx::query("UPDATE data_source SET name = ?, path = ?, source_type = ?, include_globs = ?, exclude_globs = ?, updated_at = ? WHERE id = ?")
            .bind(&req.name)
            .bind(&req.path)
            .bind(&req.source_type)
            .bind(&include_globs)
            .bind(&exclude_globs)
            .bind(&now)
            .bind(&id)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({"success": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn delete_data_source(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let result =
        sqlx::query("DELETE FROM data_source WHERE id = ?")
            .bind(&id)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({"success": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn scan_data_source(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let source =
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, name, path, source_type FROM data_source WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&db.pool)
        .await;
    
    match source {
        Ok((source_id, name, path, source_type)) => {
            let event_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            
            // 记录扫描事件
            let _ = sqlx::query("INSERT INTO event_log (id, workspace_id, actor_type, event_type, entity_type, entity_id, metadata, created_at) VALUES (?, 'default', 'system', 'scan_triggered', 'data_source', ?, ?, ?)")
                .bind(&event_id)
                .bind(&source_id)
                .bind(serde_json::json!({"path": path, "source_type": source_type}).to_string())
                .bind(&now)
                .execute(&db.pool)
                .await;
            
            // 实际扫描目录
            let scan_path = std::path::PathBuf::from(&path);
            let supported_exts: std::collections::HashSet<&str> = [
                "md", "txt", "markdown", "rst", "json", "yaml", "yml", "toml", "csv", "xml",
                "html", "rtf",
            ].iter().cloned().collect();
            
            let mut scanned = 0i64;
            let mut inserted = 0i64;
            
            if scan_path.exists() && scan_path.is_dir() {
                if let Ok(entries) = walk_dir_recursive(&scan_path) {
                    for entry_path in entries {
                        if !entry_path.is_file() { continue; }
                        let ext = entry_path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if !supported_exts.contains(ext.as_str()) { continue; }
                        scanned += 1;
                        
                        // 检查是否已存在
                        let file_path_str = entry_path.to_string_lossy().to_string();
                        let exists: (i64,) = sqlx::query_as(
                            "SELECT COUNT(*) FROM doc WHERE source_path = ? AND type = 'knowledge'"
                        )
                        .bind(&file_path_str)
                        .fetch_one(&db.pool)
                        .await
                        .unwrap_or((0,));
                        
                        if exists.0 > 0 { continue; }
                        
                        // 读取文件内容（仅文本格式）
                        let content = match std::fs::read_to_string(&entry_path) {
                            Ok(text) => text,
                            Err(_) => continue,
                        };
                        
                        if content.trim().is_empty() { continue; }
                        
                        let file_name = entry_path.file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        let title = file_name.to_string();
                        let mem_id = uuid::Uuid::new_v4().to_string();
                        let content_preview = if content.len() > 5000 {
                            format!("{}...", &content[..5000])
                        } else {
                            content
                        };
                        
                        let result = sqlx::query(
                            "INSERT INTO doc (id, workspace_id, title, content, type, source_type, metadata, source_path, status, created_at, updated_at) VALUES (?, 'default', ?, ?, 'knowledge', 'knowledge', '[]', ?, 'active', ?, ?)"
                        )
                        .bind(&mem_id)
                        .bind(&title)
                        .bind(&content_preview)
                        .bind(&file_path_str)
                        .bind(&now)
                        .bind(&now)
                        .execute(&db.pool)
                        .await;
                        
                        if result.is_ok() { inserted += 1; }
                    }
                }
            }
            
            tracing::info!("Scan completed for source {} ({}): {} scanned, {} inserted", name, source_id, scanned, inserted);
            
            Json(json!({
                "message": "Scan completed",
                "data_source_id": source_id,
                "name": name,
                "path": path,
                "source_type": source_type,
                "scanned": scanned,
                "inserted": inserted,
                "event_id": event_id
            }))
        },
        Err(_) => Json(json!({"error": "Data source not found"})),
    }
}

fn walk_dir_recursive(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                results.extend(walk_dir_recursive(&path)?);
            } else {
                results.push(path);
            }
        }
    }
    Ok(results)
}
