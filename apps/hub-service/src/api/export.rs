use axum::{routing::post, Router, Json, extract::State};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::db::Database;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/export/memory", post(export_memory))
}

#[derive(Debug, Deserialize)]
struct ExportRequest {
    format: String,
    source_type: Option<String>,
    status: Option<String>,
    ids: Option<Vec<String>>,
}

async fn export_memory(
    State(db): State<Database>,
    Json(req): Json<ExportRequest>,
) -> Json<Value> {
    let status_filter = req.status.as_deref().unwrap_or("active");
    
    let memories = if let Some(ids) = &req.ids {
        if !ids.is_empty() {
            // 按ID列表过滤
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                r#"SELECT d.id, d.title, d.content, COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), d.metadata,
                   d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'shared'), d.status, d.created_at, d.updated_at
                   FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = ? AND d.id IN ({}) ORDER BY d.created_at DESC"#,
                placeholders
            );
            let mut query = sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(&sql)
                .bind(status_filter);
            for id in ids {
                query = query.bind(id);
            }
            query.fetch_all(&db.pool).await
        } else {
            // 空ID列表，按原来的逻辑（根据source_type）
            if let Some(st) = &req.source_type {
                sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(
                    r#"SELECT d.id, d.title, d.content, COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), d.metadata,
                       d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'shared'), d.status, d.created_at, d.updated_at
                       FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = ? AND d.type = ? ORDER BY d.created_at DESC"#
                )
                .bind(status_filter)
                .bind(st)
                .fetch_all(&db.pool)
                .await
            } else {
                sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(
                    r#"SELECT d.id, d.title, d.content, COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), d.metadata,
                       d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'shared'), d.status, d.created_at, d.updated_at
                       FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = ? ORDER BY d.created_at DESC"#
                )
                .bind(status_filter)
                .fetch_all(&db.pool)
                .await
            }
        }
    } else {
        // 没有ids参数，按原来的逻辑
        if let Some(st) = &req.source_type {
            sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(
                r#"SELECT d.id, d.title, d.content, COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), d.metadata,
                   d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'shared'), d.status, d.created_at, d.updated_at
                   FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = ? AND d.type = ? ORDER BY d.created_at DESC"#
            )
            .bind(status_filter)
            .bind(st)
            .fetch_all(&db.pool)
            .await
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(
                r#"SELECT d.id, d.title, d.content, COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), d.metadata,
                   d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'shared'), d.status, d.created_at, d.updated_at
                   FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = ? ORDER BY d.created_at DESC"#
            )
            .bind(status_filter)
            .fetch_all(&db.pool)
            .await
        }
    };
    
    match memories {
        Ok(rows) => {
            let items: Vec<Value> = rows.into_iter().map(|(id, title, content, memory_type, scope, tags, source_type, confidence, visibility, status, created_at, updated_at)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                
                json!({
                    "id": id,
                    "title": title,
                    "content": content,
                    "memory_type": memory_type,
                    "scope": scope,
                    "tags": tags_vec,
                    "source_type": source_type,
                    "confidence": confidence,
                    "visibility": visibility,
                    "status": status,
                    "created_at": created_at,
                    "updated_at": updated_at
                })
            }).collect();
            
            let count = items.len();
            
            if req.format == "csv" {
                // CSV export: convert to CSV string
                let mut csv = String::from("id,title,content,memory_type,scope,source_type,confidence,visibility,status,created_at,updated_at\n");
                for item in &items {
                    csv.push_str(&format!(
                        "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        item["id"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["title"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["content"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["memory_type"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["scope"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["source_type"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["confidence"].as_f64().unwrap_or(0.0),
                        item["visibility"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["status"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["created_at"].as_str().unwrap_or_default().replace('"', "\"\""),
                        item["updated_at"].as_str().unwrap_or_default().replace('"', "\"\""),
                    ));
                }
                Json(json!({
                    "success": true,
                    "format": "csv",
                    "count": count,
                    "data": csv
                }))
            } else {
                // Default: JSON export
                Json(json!({
                    "success": true,
                    "format": "json",
                    "count": count,
                    "exported_at": chrono::Utc::now().to_rfc3339(),
                    "data": items
                }))
            }
        },
        Err(e) => Json(json!({"success": false, "error": e.to_string()})),
    }
}
