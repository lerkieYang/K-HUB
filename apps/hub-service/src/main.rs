#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use axum::{routing::get, Router, Json, middleware, http::StatusCode};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};

mod config;
mod db;
mod models;
mod api;
mod services;

// 全局存储实际使用的端口
static ACTUAL_PORT: AtomicU16 = AtomicU16::new(8443);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 支持 RUST_LOG 环境变量控制日志级别，默认 info
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let config = config::Config::from_env()?;
    let db = db::Database::new(&config).await?;

    // 启动时检查并下载 Tesseract 中文语言包（后台执行，不阻塞启动）
    tokio::spawn(async {
        if let Err(e) = ensure_tesseract_langpack().await {
            tracing::warn!("Tesseract language pack check failed: {}", e);
        }
    });

    // 启动文件系统监控 — 自动在文件变化时触发重新索引
    let mut file_watcher = services::file_watcher::FileWatcher::new(db.clone());
    if let Err(e) = file_watcher.start().await {
        tracing::warn!("Failed to start file watcher (non-fatal): {}", e);
    }
    
    // CORS 配置 — 允许所有来源（开发/本地网络场景）
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/discovery/info", get(discovery_info))
        .merge(api::routes())
        .layer(cors)
        .layer(middleware::from_fn(auth_middleware))
        .with_state(db);
    
    // 单机模式绑定 localhost，多设备模式绑定 0.0.0.0
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let base_port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8443".to_string())
        .parse()
        .unwrap_or(8443);
    
    // 端口自动切换：如果端口被占用，自动尝试下一个端口
    let mut port = base_port;
    let max_attempts = 10;
    let listener = loop {
        let addr = SocketAddr::new(bind_addr.parse()?, port);
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                tracing::info!("Hub Service listening on {}", addr);
                ACTUAL_PORT.store(port, Ordering::Relaxed);
                break listener;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse && port < base_port + max_attempts {
                    tracing::warn!("Port {} in use, trying {}...", port, port + 1);
                    port += 1;
                } else {
                    return Err(e.into());
                }
            }
        }
    };
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn discovery_info() -> Json<Value> {
    let local_ip = get_local_ip();
    let port = ACTUAL_PORT.load(Ordering::Relaxed);
    Json(json!({
        "hub_name": "K-HUB",
        "hub_id": "default",
        "hub_url": format!("http://{}:{}", local_ip, port),
        "mcp_url": format!("http://{}:{}/mcp", local_ip, port),
        "local_ip": local_ip,
        "port": port,
        "requires_invite": true
    }))
}

fn get_local_ip() -> String {
    use std::net::UdpSocket;
    
    let socket = UdpSocket::bind("0.0.0.0:0").ok();
    if let Some(socket) = socket {
        socket.connect("8.8.8.8:80").ok();
        if let Ok(addr) = socket.local_addr() {
            return addr.ip().to_string();
        }
    }
    
    "127.0.0.1".to_string()
}

// 鉴权中间件
async fn auth_middleware(
    req: axum::extract::Request,
    next: middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let path = req.uri().path();
    
    // 健康检查和发现端点不需要鉴权
    if path == "/health" || path == "/api/discovery/info" {
        return Ok(next.run(req).await);
    }
    
    // MCP 端点检查 token
    if path.starts_with("/mcp") {
        // 从 header 或 query 获取 token
        let token = req.headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());
        
        // 检查 MCP token（从环境变量读取，未设置则放行本地请求）
        let mcp_token = std::env::var("MCP_TOKEN").unwrap_or_default();
        
        if !mcp_token.is_empty() {
            // 有配置 token 时必须验证
            match token {
                Some(t) if t == mcp_token => {},
                _ => {
                    tracing::warn!("MCP request rejected: invalid or missing token");
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        } else {
            // 无配置 token 时只允许本地访问
            let remote_addr = req.extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0);
            
            if let Some(addr) = remote_addr {
                if !addr.ip().is_loopback() {
                    tracing::warn!("MCP request rejected: non-local access without MCP_TOKEN");
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }
    
    // 管理 API 检查
    if path.starts_with("/api/") {
        // 本地访问不需要额外验证
        let remote_addr = req.extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0);

        if let Some(addr) = remote_addr {
            // 只允许本地访问管理 API
            if !addr.ip().is_loopback() {
                tracing::warn!("Blocked non-local access to {} from {}", path, addr);
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            // ConnectInfo 缺失 — 通常意味着前面有反向代理
            let bind_addr = std::env::var("BIND_ADDR").unwrap_or_default();
            if bind_addr == "0.0.0.0" {
                tracing::warn!(
                    "API request to {} has no ConnectInfo and BIND_ADDR=0.0.0.0 — \
                     this means the request came through a reverse proxy. \
                     Configure proxy to forward X-Forwarded-For, or restrict access at proxy level.",
                    path
                );
            }
        }
    }
    
    Ok(next.run(req).await)
}

/// 检查并下载 Tesseract 中文语言包
/// 如果 Tesseract 已安装但缺少中文语言包，自动下载
async fn ensure_tesseract_langpack() -> Result<(), String> {
    // 查找 Tesseract 安装路径
    let tesseract_dir = find_tesseract_dir()?;
    let tessdata_dir = tesseract_dir.join("tessdata");

    // 检查是否已有中文语言包
    let chi_sim = tessdata_dir.join("chi_sim.traineddata");
    if chi_sim.exists() {
        return Ok(()); // 已有，不需要下载
    }

    // 检查 tessdata 目录是否存在
    if !tessdata_dir.exists() {
        return Err("tessdata directory not found".to_string());
    }

    tracing::info!("Tesseract Chinese language pack not found, downloading...");

    // 下载中文语言包
    let url = "https://github.com/tesseract-ocr/tessdata/raw/main/chi_sim.traineddata";
    let client = reqwest::Client::new();
    let response = client.get(url)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // 写入文件
    tokio::fs::write(&chi_sim, &bytes).await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    tracing::info!("Tesseract Chinese language pack downloaded successfully ({} bytes)", bytes.len());

    Ok(())
}

/// 查找 Tesseract 安装目录
fn find_tesseract_dir() -> Result<std::path::PathBuf, String> {
    // 优先：应用目录内的 tesseract
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bundled = exe_dir.join("resources").join("tesseract");
            if bundled.exists() {
                return Ok(bundled);
            }
        }
    }

    // 其次：用户本地目录
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let local_path = std::path::Path::new(&local_app_data).join("Tesseract-OCR");
        if local_path.exists() {
            return Ok(local_path);
        }
    }

    // 最后：Program Files
    let program_files = std::path::Path::new(r"C:\Program Files\Tesseract-OCR");
    if program_files.exists() {
        return Ok(program_files.to_path_buf());
    }

    Err("Tesseract not found".to_string())
}


