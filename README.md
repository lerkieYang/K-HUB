# K-HUB (Knowledge Hub)

个人AI知识中枢 — 统一管理本地文件、Agent输出和候选记忆，提供MCP接口供第三方Agent检索。

## 功能概览

### 核心功能
- **知识库管理**：索引本地文件夹，支持PDF/DOC/DOCX/XLS/XLSX/PPT/PPTX等格式
- **记忆管理**：收集和管理Agent产生的记忆数据
- **会话管理**：存储和检索Agent会话记录
- **向量化**：使用AI提取关键词标签，支持语义搜索
- **Agent集成**：自动发现和配置AI Agent的MCP连接
- **任务管理**：后台索引、向量化任务的进度追踪

### Dashboard功能
- 知识库/记忆/会话的数量统计
- 索引进度和向量化进度实时显示
- Agent活跃度和向量化覆盖率可视化
- 一键索引/向量化/拉取操作

### 资源管理
- 知识库文件浏览和搜索
- 记忆数据的导入/导出/编辑
- 会话记录的查看和管理
- 批量操作支持

### 设置功能
- 知识库目录配置
- AI模型配置（embedding和chat）
- 定时任务调度
- 数据管理（清除/恢复/导出）

## 端口信息

| 服务 | 默认端口 | 说明 |
|------|----------|------|
| Hub Service (后端) | 8443 | API服务 + MCP服务 |
| Vite Dev Server (前端) | 1420 | 开发服务器 |

**端口自动切换**：如果默认端口被占用，后端会自动尝试下一个端口（8444, 8445...），最多尝试10次。通过 `/api/discovery/info` 可查询实际使用的端口。

## 技术实现

### 架构

```
┌─────────────────┐     ┌─────────────────┐
│   浏览器前端     │────>│   Hub Service   │
│  (Vite + React) │     │   (Rust/Axum)   │
└─────────────────┘     └────────┬────────┘
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
              ┌─────┴─────┐ ┌───┴───┐ ┌─────┴─────┐
              │  SQLite   │ │ AI    │ │ 文件系统  │
              │  数据库   │ │ 服务  │ │ 监控     │
              └───────────┘ └───────┘ └───────────┘
```

### 技术栈

**后端 (Rust)**
- Web框架：Axum
- 数据库：SQLite (sqlx)
- 异步运行时：Tokio
- 序列化：Serde

**前端 (TypeScript)**
- 框架：React 18
- 状态管理：Zustand
- 路由：React Router
- 样式：Tailwind CSS
- 构建：Vite

### 数据库设计

```sql
-- 文档表（统一存储知识/记忆/会话）
CREATE TABLE doc (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,           -- 'knowledge' | 'memory' | 'session'
    title TEXT,
    content TEXT,
    status TEXT DEFAULT 'active', -- 'active' | 'deleted' | 'archived'
    source_type TEXT,             -- 'knowledge' | 'hermes' | 'openclaw' | ...
    source_path TEXT,
    metadata TEXT,                -- JSON格式的标签/关键词
    created_at TEXT,
    updated_at TEXT
);

-- 知识库扩展表
CREATE TABLE knowledge_ext (
    id TEXT PRIMARY KEY,
    file_path TEXT,
    version INTEGER DEFAULT 1,
    is_current INTEGER DEFAULT 1,
    FOREIGN KEY (id) REFERENCES doc(id)
);

-- 记忆扩展表
CREATE TABLE memory_ext (
    id TEXT PRIMARY KEY,
    agent_id TEXT,
    category TEXT,
    owner TEXT,
    visibility TEXT DEFAULT 'private',
    memory_type TEXT DEFAULT 'semantic',
    scope TEXT DEFAULT 'project',
    confidence REAL DEFAULT 0.8,
    FOREIGN KEY (id) REFERENCES doc(id)
);

-- 会话扩展表
CREATE TABLE session_ext (
    id TEXT PRIMARY KEY,
    agent_id TEXT,
    session_id TEXT,
    FOREIGN KEY (id) REFERENCES doc(id)
);

-- 知识库配置表
CREATE TABLE knowledge_base_config (
    id TEXT PRIMARY KEY,
    name TEXT,
    path TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    created_at TEXT
);
```

### API端点

**健康检查**
- `GET /health` - 服务健康状态

**发现服务**
- `GET /api/discovery/info` - 服务信息（包含实际端口）

**知识库**
- `GET /api/knowledge/config` - 列出配置
- `POST /api/knowledge/config` - 创建配置
- `DELETE /api/knowledge/config/:id` - 删除配置
- `POST /api/knowledge/reindex` - 重新索引所有
- `POST /api/knowledge/config/:id/reindex` - 索引单个配置
- `GET /api/knowledge/progress` - 索引进度
- `GET /api/knowledge/stats` - 知识库统计
- `POST /api/knowledge/light-embed` - 向量化
- `GET /api/knowledge/tasks` - 任务列表
- `POST /api/knowledge/tasks/stop-all` - 停止所有任务

**记忆**
- `GET /api/memory/search` - 搜索记忆
- `GET /api/memory/:id` - 获取单条记忆
- `PUT /api/memory/:id` - 更新记忆
- `DELETE /api/memory/:id` - 删除记忆
- `POST /api/memory/batch-delete` - 批量删除
- `GET /api/memory/stats` - 记忆统计

**数据统计**
- `GET /api/data/stats` - 综合数据统计

**Agent**
- `POST /api/agents/scan` - 扫描Agent
- `POST /api/agents/configure` - 配置Agent
- `POST /api/agents/unconfigure` - 取消配置
- `GET /api/agents/:id/config-prompt` - 获取配置提示词
- `POST /api/agents/test-connection` - 测试连接

**MCP**
- `POST /mcp` - MCP JSON-RPC端点

### MCP工具

**knowledge.search** - 搜索知识库
```json
{
  "query": "搜索关键词",
  "limit": 5
}
```

**context.request** - 请求上下文
```json
{
  "query": "上下文查询",
  "max_tokens": 2000
}
```

**memory.submit_candidate** - 提交候选记忆
```json
{
  "title": "记忆标题",
  "content": "记忆内容",
  "tags": ["标签1", "标签2"]
}
```

**artifact.submit** - 提交制品
```json
{
  "title": "制品标题",
  "content": "制品内容",
  "type": "code"
}
```

**policy.check** - 检查策略
```json
{
  "action": "read",
  "resource": "knowledge"
}
```

## 使用方式

### 环境要求

- Windows 10/11
- Rust 1.70+
- Node.js 18+
- npm 或 yarn

### 安装步骤

1. **克隆项目**
```bash
git clone <repository-url>
cd knowledgeHUB
```

2. **编译后端**
```bash
cd apps/hub-service
cargo build --release
```

3. **安装前端依赖**
```bash
cd apps/desktop
npm install
```

4. **启动服务**
```bash
# 启动后端
cd apps/hub-service
./target/release/hub-service.exe

# 启动前端（开发模式）
cd apps/desktop
npm run dev
```

5. **访问应用**
- 前端：http://localhost:1420
- 后端API：http://localhost:8443

### 配置

**环境变量**
- `BIND_ADDR` - 绑定地址（默认：0.0.0.0）
- `PORT` - 端口号（默认：8443）
- `RUST_LOG` - 日志级别（默认：info）
- `MCP_TOKEN` - MCP认证token（可选）

**数据目录**
- 默认：`./data`
- 包含：SQLite数据库、embedding文件、缓存

### 打包发布

**后端**
```bash
cd apps/hub-service
cargo build --release
# 输出：target/release/hub-service.exe
```

**前端**
```bash
cd apps/desktop
npm run build
# 输出：dist/ 目录
```

## 开发指南

### 项目结构

```
knowledgeHUB/
├── apps/
│   ├── hub-service/          # 后端服务
│   │   ├── src/
│   │   │   ├── api/          # API路由
│   │   │   ├── db/           # 数据库
│   │   │   ├── models/       # 数据模型
│   │   │   ├── services/     # 业务逻辑
│   │   │   └── main.rs       # 入口
│   │   └── Cargo.toml
│   └── desktop/              # 前端应用
│       ├── src/
│       │   ├── pages/        # 页面组件
│       │   ├── stores/       # 状态管理
│       │   ├── i18n/         # 国际化
│       │   └── App.tsx       # 入口
│       └── package.json
└── README.md
```

### 添加新功能

1. **后端**
   - 在 `api/` 添加路由
   - 在 `services/` 实现业务逻辑
   - 在 `db/` 添加数据库操作

2. **前端**
   - 在 `pages/` 添加页面组件
   - 在 `stores/` 添加状态管理
   - 在 `i18n/` 添加翻译

### 调试

**后端日志**
```bash
RUST_LOG=debug cargo run
```

**前端开发**
```bash
npm run dev
# 访问 http://localhost:1420
```

## 常见问题

### 端口被占用
后端会自动尝试下一个端口。查看实际端口：
```bash
curl http://localhost:8443/api/discovery/info
```

### 连接失败
- 检查后端是否运行
- 检查防火墙设置
- 确认端口配置

### 索引失败
- 检查文件路径是否正确
- 检查文件权限
- 查看后端日志

### 向量化失败
- 检查AI配置是否正确
- 检查API密钥是否有效
- 查看任务管理页面的错误信息

## 更新日志

### 2026-05-08
- 知识库索引版本逻辑修复：内容不变时不创建新版本
- Agents页面布局优化：备注名和地址对齐
- Dashboard统计卡片括号格式统一
- Tasks页面向量化进度显示修复
- 端口自动切换功能
- 按钮颜色一致性修复

## 许可证

[待定]

## 联系方式

[待定]
