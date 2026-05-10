use axum::{routing::{get, post}, Router, Json, extract::{State, Path, Query}};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::db::Database;
use crate::services::agent_discovery::AgentDiscovery;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/agents/scan", get(scan_agents).post(scan_agents))
        .route("/api/agents/configure", post(configure_agent))
        .route("/api/agents/unconfigure", post(unconfigure_agent))
        .route("/api/agents/:id/status", get(agent_status))
        .route("/api/agents/:id/config-prompt", get(get_config_prompt))
        .route("/api/agents/test-connection", post(test_connection))
}

/// 扫描Agent
async fn scan_agents(
    State(_db): State<Database>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.scan_agents();
    
    Json(json!({
        "agents": agents,
        "total": agents.len()
    }))
}

/// 配置Agent请求
#[derive(Debug, Deserialize)]
struct ConfigureAgentRequest {
    agent_id: String,
    hub_url: Option<String>,
}

/// 获取默认 HUB URL（优先使用环境变量）
fn default_hub_url() -> String {
    std::env::var("KH_HUB_URL").unwrap_or_else(|_| "http://127.0.0.1:8443".to_string())
}

/// 配置Agent
async fn configure_agent(
    State(_db): State<Database>,
    Json(req): Json<ConfigureAgentRequest>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();
    let hub_url = req.hub_url.unwrap_or_else(default_hub_url);
    
    match discovery.configure_agent(&req.agent_id, &hub_url) {
        Ok(message) => Json(json!({
            "success": true,
            "message": message
        })),
        Err(error) => Json(json!({
            "success": false,
            "error": error
        })),
    }
}

/// Agent状态
async fn agent_status(
    State(_db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.scan_agents();
    
    match agents.iter().find(|a| a.id == id) {
        Some(agent) => Json(json!(agent)),
        None => Json(json!({"error": "Agent not found"})),
    }
}

/// 取消配置Agent
async fn unconfigure_agent(
    State(_db): State<Database>,
    Json(req): Json<ConfigureAgentRequest>,
) -> Json<Value> {
    let discovery = AgentDiscovery::new();

    match discovery.unconfigure_agent(&req.agent_id) {
        Ok(message) => Json(json!({
            "success": true,
            "message": message
        })),
        Err(error) => Json(json!({
            "success": false,
            "error": error
        })),
    }
}

/// 获取 Agent 自配置提示词
async fn get_config_prompt(
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    // 优先用前端传来的 hub_url，否则用环境变量
    let hub_url = params.get("hub_url")
        .cloned()
        .unwrap_or_else(default_hub_url);
    let mcp_url = format!("{}/mcp", hub_url);
    
    let prompt = match id.as_str() {
        "hermes" => format!(r#"# K-HUB 连通任务 (Hermes)

你的任务是连接到 K-HUB MCP 服务。

## 第一步：检查是否已配置

检查 `~/.hermes/mcp.yaml` 文件中是否已有 `k-hub` 服务器配置：
```bash
cat ~/.hermes/mcp.yaml 2>/dev/null | grep -q "k-hub" && echo "已配置" || echo "未配置"
```

## 第二步：写入配置

如果未配置，编辑 `~/.hermes/mcp.yaml`，添加 K-HUB 服务器：

```yaml
mcp:
  servers:
    k-hub:
      url: {mcp_url}
      transport: http
```

如果文件已存在其他 MCP 服务器，在 `servers` 下追加 `k-hub` 即可，不要覆盖已有配置。

## 第三步：测试连通性

调用 MCP 工具验证：
1. 调用 `knowledge.search`，参数 `query` 设为 `"test"`
2. 如果返回结果（即使为空数组），说明连通成功

## 第四步：报告结果

- 成功：`K-HUB 已连通 ✅`
- 失败：`K-HUB 连通失败 ❌ （错误原因：xxx）`

## 记忆提交指南

### 应该提交的情况
- 用户明确要求保存（"记住这个"、"保存到知识库"）
- 发现重要配置信息（端口、路径、密钥位置、服务地址）
- 解决了复杂问题（调试过程、根本原因、解决方案）
- 用户偏好（代码风格、工具选择、工作习惯）
- 项目架构决策（技术选型、设计模式、目录结构）
- 重要的环境信息（OS、Docker配置、网络拓扑）

### 不应该提交的情况
- 临时性操作（正在调试、测试中的中间状态）
- 通用常识（不需要记忆的公共知识）
- 敏感信息原文（密码、密钥本身——可以记录位置但不记录内容）
- 一次性任务的执行细节

### 提交格式
调用 `memory.submit_candidate` 工具：
- `title`: 简短描述（10字以内）
- `content`: 详细内容（包含上下文和关键信息）
- `memory_type`: semantic（知识）/ episodic（事件）/ procedural（流程）
- `tags`: 分类标签数组
- `agent_id`: 你的agent标识（hermes）

请现在开始执行。"#, mcp_url = mcp_url),

        "codex" => format!(r#"# K-HUB 连通任务 (Codex)

你的任务是连接到 K-HUB MCP 服务。

## 第一步：检查是否已配置

检查 `~/.codex/mcp.yaml` 文件中是否已有 `k-hub` 服务器配置：
```bash
cat ~/.codex/mcp.yaml 2>/dev/null | grep -q "k-hub" && echo "已配置" || echo "未配置"
```

## 第二步：写入配置

如果未配置，创建或编辑 `~/.codex/mcp.yaml`：

```yaml
mcp:
  servers:
    k-hub:
      url: {mcp_url}
      transport: http
```

如果文件已存在其他 MCP 服务器，在 `servers` 下追加 `k-hub`。

## 第三步：测试连通性

调用 MCP 工具验证：
1. 调用 `knowledge.search`，参数 `query` 设为 `"test"`
2. 如果返回结果，说明连通成功

## 第四步：报告结果

- 成功：`K-HUB 已连通 ✅`
- 失败：`K-HUB 连通失败 ❌ （错误原因：xxx）`

## 记忆提交指南

### 应该提交的情况
- 用户明确要求保存（"记住这个"、"保存到知识库"）
- 发现重要配置信息（端口、路径、密钥位置、服务地址）
- 解决了复杂问题（调试过程、根本原因、解决方案）
- 用户偏好（代码风格、工具选择、工作习惯）
- 项目架构决策（技术选型、设计模式、目录结构）
- 重要的环境信息（OS、Docker配置、网络拓扑）

### 不应该提交的情况
- 临时性操作（正在调试、测试中的中间状态）
- 通用常识（不需要记忆的公共知识）
- 敏感信息原文（密码、密钥本身——可以记录位置但不记录内容）
- 一次性任务的执行细节

### 提交格式
调用 `memory.submit_candidate` 工具：
- `title`: 简短描述（10字以内）
- `content`: 详细内容（包含上下文和关键信息）
- `memory_type`: semantic（知识）/ episodic（事件）/ procedural（流程）
- `tags`: 分类标签数组
- `agent_id`: 你的agent标识（codex）

请现在开始执行。"#, mcp_url = mcp_url),
        
        "gemini" => format!(r#"# K-HUB 连通任务 (Gemini)

你的任务是连接到 K-HUB MCP 服务。

## 第一步：检查是否已配置

检查 `~/.gemini/settings.json` 中是否已有 K-HUB 配置：
```bash
cat ~/.gemini/settings.json 2>/dev/null | grep -q "k-hub" && echo "已配置" || echo "未配置"
```

## 第二步：写入配置

Gemini CLI 使用 JSON 格式的 settings.json。编辑 `~/.gemini/settings.json`，在 `mcpServers` 中添加：

```json
{{
  "mcpServers": {{
    "k-hub": {{
      "url": "{mcp_url}",
      "transport": "http"
    }}
  }}
}}
```

⚠️ 注意：如果文件已有其他配置，只在 `mcpServers` 对象中追加 `k-hub` 字段，不要覆盖整个文件。

## 第三步：测试连通性

调用 MCP 工具验证：
1. 调用 `knowledge.search`，参数 `query` 设为 `"test"`
2. 如果返回结果，说明连通成功

## 第四步：报告结果

- 成功：`K-HUB 已连通 ✅`
- 失败：`K-HUB 连通失败 ❌ （错误原因：xxx）`

## 记忆提交指南

### 应该提交的情况
- 用户明确要求保存（"记住这个"、"保存到知识库"）
- 发现重要配置信息（端口、路径、密钥位置、服务地址）
- 解决了复杂问题（调试过程、根本原因、解决方案）
- 用户偏好（代码风格、工具选择、工作习惯）
- 项目架构决策（技术选型、设计模式、目录结构）
- 重要的环境信息（OS、Docker配置、网络拓扑）

### 不应该提交的情况
- 临时性操作（正在调试、测试中的中间状态）
- 通用常识（不需要记忆的公共知识）
- 敏感信息原文（密码、密钥本身——可以记录位置但不记录内容）
- 一次性任务的执行细节

### 提交格式
调用 `memory.submit_candidate` 工具：
- `title`: 简短描述（10字以内）
- `content`: 详细内容（包含上下文和关键信息）
- `memory_type`: semantic（知识）/ episodic（事件）/ procedural（流程）
- `tags`: 分类标签数组
- `agent_id`: 你的agent标识（gemini）

请现在开始执行。"#, mcp_url = mcp_url),
        
        "openclaw" => format!(r#"# K-HUB 连通任务 (OpenClaw)

你的任务是连接到 K-HUB MCP 服务。

## 第一步：检查是否已配置

检查 `~/.openclaw/openclaw.json` 中是否已有 K-HUB 配置：
```bash
cat ~/.openclaw/openclaw.json 2>/dev/null | grep -q "k-hub" && echo "已配置" || echo "未配置"
```

## 第二步：写入配置

OpenClaw 的 MCP 配置嵌入在 `openclaw.json` 中。编辑 `~/.openclaw/openclaw.json`，添加 `mcp.servers.k-hub`：

```json
{{
  "mcp": {{
    "servers": {{
      "k-hub": {{
        "url": "{mcp_url}",
        "transport": "http"
      }}
    }}
  }}
}}
```

⚠️ 注意：OpenClaw 的配置文件可能包含其他字段（如 agents、settings 等）。只在 `mcp.servers` 中追加 `k-hub`，不要覆盖其他配置。

## 第三步：测试连通性

调用 MCP 工具验证：
1. 调用 `knowledge.search`，参数 `query` 设为 `"test"`
2. 如果返回结果，说明连通成功

## 第四步：报告结果

- 成功：`K-HUB 已连通 ✅`
- 失败：`K-HUB 连通失败 ❌ （错误原因：xxx）`

## 记忆提交指南

### 应该提交的情况
- 用户明确要求保存（"记住这个"、"保存到知识库"）
- 发现重要配置信息（端口、路径、密钥位置、服务地址）
- 解决了复杂问题（调试过程、根本原因、解决方案）
- 用户偏好（代码风格、工具选择、工作习惯）
- 项目架构决策（技术选型、设计模式、目录结构）
- 重要的环境信息（OS、Docker配置、网络拓扑）

### 不应该提交的情况
- 临时性操作（正在调试、测试中的中间状态）
- 通用常识（不需要记忆的公共知识）
- 敏感信息原文（密码、密钥本身——可以记录位置但不记录内容）
- 一次性任务的执行细节

### 提交格式
调用 `memory.submit_candidate` 工具：
- `title`: 简短描述（10字以内）
- `content`: 详细内容（包含上下文和关键信息）
- `memory_type`: semantic（知识）/ episodic（事件）/ procedural（流程）
- `tags`: 分类标签数组
- `agent_id`: 你的agent标识（openclaw）

请现在开始执行。"#, mcp_url = mcp_url),
        
        _ => format!(r#"# K-HUB 连通任务

你的任务是连接到 K-HUB MCP 服务。

## 第一步：检查是否已配置

检查你的 MCP 配置文件中是否已有 `k-hub` 服务器：
- 搜索关键词 `k-hub`

## 第二步：写入配置

在你的 MCP 配置文件中添加 K-HUB 服务器。根据你的 Agent 类型，配置文件可能是：
- YAML 格式 (`mcp.yaml`):
  ```yaml
  mcp:
    servers:
      k-hub:
        url: {mcp_url}
        transport: http
  ```
- JSON 格式 (`settings.json` 或 `openclaw.json`):
  ```json
  {{"mcp": {{"servers": {{"k-hub": {{"url": "{mcp_url}", "transport": "http"}}}}}}}}
  ```

如果已有其他 MCP 服务器配置，追加 `k-hub` 即可，不要覆盖已有配置。

## 第三步：测试连通性

调用 MCP 工具验证：
1. 调用 `knowledge.search`，参数 `query` 设为 `"test"`
2. 如果返回结果，说明连通成功

## 第四步：报告结果

- 成功：`K-HUB 已连通 ✅`
- 失败：`K-HUB 连通失败 ❌ （错误原因：xxx）`

## 记忆提交指南

### 应该提交的情况
- 用户明确要求保存（"记住这个"、"保存到知识库"）
- 发现重要配置信息（端口、路径、密钥位置、服务地址）
- 解决了复杂问题（调试过程、根本原因、解决方案）
- 用户偏好（代码风格、工具选择、工作习惯）
- 项目架构决策（技术选型、设计模式、目录结构）
- 重要的环境信息（OS、Docker配置、网络拓扑）

### 不应该提交的情况
- 临时性操作（正在调试、测试中的中间状态）
- 通用常识（不需要记忆的公共知识）
- 敏感信息原文（密码、密钥本身——可以记录位置但不记录内容）
- 一次性任务的执行细节

### 提交格式
调用 `memory.submit_candidate` 工具：
- `title`: 简短描述（10字以内）
- `content`: 详细内容（包含上下文和关键信息）
- `memory_type`: semantic（知识）/ episodic（事件）/ procedural（流程）
- `tags`: 分类标签数组
- `agent_id`: 你的agent标识（hermes/codex/gemini/openclaw）

请现在开始执行。"#, mcp_url = mcp_url),
    };
    
    Json(json!({
        "agent_id": id,
        "hub_url": hub_url,
        "mcp_url": mcp_url,
        "prompt": prompt
    }))
}

/// 测试 MCP 连通性
#[derive(Debug, Deserialize)]
struct TestConnectionRequest {
    hub_url: Option<String>,
}

async fn test_connection(
    Json(req): Json<TestConnectionRequest>,
) -> Json<Value> {
    let hub_url = req.hub_url.unwrap_or_else(default_hub_url);
    let mcp_url = format!("{}/mcp", hub_url);
    let health_url = format!("{}/health", hub_url);
    
    let client = reqwest::Client::new();
    let mut results = json!({
        "hub_url": hub_url,
        "mcp_url": mcp_url,
    });
    
    // 1. 测试健康检查
    match client.get(&health_url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(resp) => {
            results["health"] = json!({
                "status": resp.status().as_u16(),
                "ok": resp.status().is_success()
            });
        },
        Err(e) => {
            results["health"] = json!({
                "status": 0,
                "ok": false,
                "error": e.to_string()
            });
        }
    }
    
    // 2. 测试 MCP initialize
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });
    
    match client.post(&mcp_url)
        .timeout(std::time::Duration::from_secs(5))
        .json(&mcp_request)
        .send().await 
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body: Value = resp.json().await.unwrap_or(json!({}));
            results["mcp"] = json!({
                "status": status,
                "ok": status == 200 && body.get("result").is_some(),
                "server": body.pointer("/result/serverInfo/name").cloned().unwrap_or(json!(null)),
                "version": body.pointer("/result/serverInfo/version").cloned().unwrap_or(json!(null))
            });
        },
        Err(e) => {
            results["mcp"] = json!({
                "status": 0,
                "ok": false,
                "error": e.to_string()
            });
        }
    }
    
    // 3. 测试 tools/list
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    
    match client.post(&mcp_url)
        .timeout(std::time::Duration::from_secs(5))
        .json(&tools_request)
        .send().await 
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body: Value = resp.json().await.unwrap_or(json!({}));
            let tools = body.pointer("/result/tools")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect::<Vec<_>>())
                .unwrap_or_default();
            results["tools"] = json!({
                "status": status,
                "ok": !tools.is_empty(),
                "count": tools.len(),
                "names": tools
            });
        },
        Err(e) => {
            results["tools"] = json!({
                "status": 0,
                "ok": false,
                "error": e.to_string()
            });
        }
    }
    
    let all_ok = results["health"]["ok"].as_bool().unwrap_or(false)
        && results["mcp"]["ok"].as_bool().unwrap_or(false)
        && results["tools"]["ok"].as_bool().unwrap_or(false);
    
    results["all_ok"] = json!(all_ok);
    
    Json(results)
}
