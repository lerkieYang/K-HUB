use axum::{routing::get, Router, Json, extract::Query};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use crate::db::Database;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/filesystem/list", get(list_directory))
        .route("/api/filesystem/roots", get(list_roots))
}

#[derive(Debug, Deserialize)]
struct ListDirRequest {
    path: Option<String>,
}

/// 列出系统根目录
async fn list_roots() -> Json<Value> {
    let mut roots = Vec::new();
    
    // Windows盘符
    for letter in 'A'..='Z' {
        let drive = format!("{}:\\", letter);
        let path = Path::new(&drive);
        if path.exists() {
            roots.push(json!({
                "name": format!("{}:", letter),
                "path": drive,
                "type": "drive"
            }));
        }
    }
    
    // WSL home
    let wsl_home = "/home";
    if Path::new(wsl_home).exists() {
        roots.push(json!({
            "name": "WSL /home",
            "path": wsl_home,
            "type": "directory"
        }));
    }
    
    // Windows 特殊文件夹 (Quick Access)
    let special_folders: Vec<(&str, &str)> = vec![
        ("Desktop", "C:\\Users\\lerki\\Desktop"),
        ("Documents", "C:\\Users\\lerki\\Documents"),
        ("Downloads", "C:\\Users\\lerki\\Downloads"),
        ("OneDrive", "C:\\Users\\lerki\\OneDrive"),
    ];
    for (name, path_str) in special_folders {
        if Path::new(path_str).exists() {
            roots.push(json!({
                "name": name,
                "path": path_str,
                "type": "special_folder"
            }));
        }
    }
    
    // 云盘文件夹 (Cloud Storage)
    let cloud_folders: Vec<(&str, &str)> = vec![
        ("WPS Cloud", "C:\\Users\\lerki\\WPS Cloud Files"),
        ("Xiaomi Cloud", "C:\\Users\\lerki\\Xiaomi Cloud"),
    ];
    for (name, path_str) in cloud_folders {
        if Path::new(path_str).exists() {
            roots.push(json!({
                "name": name,
                "path": path_str,
                "type": "cloud_folder"
            }));
        }
    }
    
    // UNC 网络路径支持 (Network Paths)
    // 常见的 UNC 路径可以通过 \\\\server\\share 格式访问
    // 这里提供一个网络入口，用户可以手动输入 UNC 路径
    let network_paths: Vec<(&str, &str)> = vec![
        ("Network", "\\\\"),
    ];
    for (name, path_str) in network_paths {
        // 检查是否可以访问网络根目录
        if Path::new(path_str).exists() {
            roots.push(json!({
                "name": name,
                "path": path_str,
                "type": "network"
            }));
        }
    }
    
    Json(json!({
        "roots": roots
    }))
}

/// 列出目录内容
async fn list_directory(
    Query(req): Query<ListDirRequest>,
) -> Json<Value> {
    let dir_path = req.path.unwrap_or_else(|| {
        // 默认返回用户目录
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "C:\\".to_string())
    });
    
    let path = Path::new(&dir_path);
    
    if !path.exists() {
        return Json(json!({
            "error": "Directory not found",
            "path": dir_path
        }));
    }
    
    if !path.is_dir() {
        return Json(json!({
            "error": "Not a directory",
            "path": dir_path
        }));
    }
    
    // 判断是否为 UNC 路径 (\\server\share)
    let is_unc = dir_path.starts_with("\\\\") || dir_path.starts_with("//");
    
    let mut entries = Vec::new();
    
    // 读取目录内容
    if let Ok(dir) = std::fs::read_dir(path) {
        for entry in dir.flatten() {
            let entry_path = entry.path();
            let name = entry_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            // 跳过隐藏文件和系统目录（UNC 路径下不过滤）
            if !is_unc && (name.starts_with('.') || name == "node_modules" || name == "__pycache__") {
                continue;
            }
            
            let is_dir = entry_path.is_dir();
            let metadata = entry.metadata().ok();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata.as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            entries.push(json!({
                "name": name,
                "path": entry_path.to_string_lossy(),
                "isDirectory": is_dir,
                "size": size,
                "modified": modified
            }));
        }
    }
    
    // 排序：目录在前，文件在后
    entries.sort_by(|a, b| {
        let a_is_dir = a["isDirectory"].as_bool().unwrap_or(false);
        let b_is_dir = b["isDirectory"].as_bool().unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                a_name.cmp(b_name)
            }
        }
    });
    
    // 计算父目录
    let parent = path.parent().map(|p| p.to_string_lossy().to_string());
    
    Json(json!({
        "path": dir_path,
        "parent": parent,
        "entries": entries,
        "total": entries.len()
    }))
}
