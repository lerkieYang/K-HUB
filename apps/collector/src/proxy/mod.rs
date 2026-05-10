pub mod context;
pub mod memory;

use crate::config::Config;
use axum::{routing::post, Router, Json, extract::State};
use serde_json::{json, Value};
use reqwest::Client;

#[derive(Clone)]
pub struct McpProxy {
    config: Config,
    client: Client,
}

impl McpProxy {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client = Client::new();
        Ok(Self { config, client })
    }
    
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting MCP proxy on port 8444");
        
        let app = Router::new()
            .route("/mcp/context.request", post(context_request))
            .route("/mcp/memory.submit_candidate", post(memory_submit))
            .route("/mcp/artifact.submit", post(artifact_submit))
            .with_state(self.clone());
        
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8444));
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        
        Ok(())
    }
    
    pub async fn proxy_context_request(&self, params: Value) -> anyhow::Result<Value> {
        let url = format!("{}/api/context/request", self.config.hub_url);
        let response = self.client.post(&url)
            .json(&params)
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
    
    pub async fn proxy_memory_submit(&self, params: Value) -> anyhow::Result<Value> {
        let url = format!("{}/api/memory/candidates", self.config.hub_url);
        let response = self.client.post(&url)
            .json(&params)
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
    
    pub async fn proxy_artifact_submit(&self, params: Value) -> anyhow::Result<Value> {
        let url = format!("{}/api/artifacts", self.config.hub_url);
        let response = self.client.post(&url)
            .json(&params)
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
}

async fn context_request(
    State(proxy): State<McpProxy>,
    Json(params): Json<Value>,
) -> Json<Value> {
    match proxy.proxy_context_request(params).await {
        Ok(result) => Json(result),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn memory_submit(
    State(proxy): State<McpProxy>,
    Json(params): Json<Value>,
) -> Json<Value> {
    match proxy.proxy_memory_submit(params).await {
        Ok(result) => Json(result),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn artifact_submit(
    State(proxy): State<McpProxy>,
    Json(params): Json<Value>,
) -> Json<Value> {
    match proxy.proxy_artifact_submit(params).await {
        Ok(result) => Json(result),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}
