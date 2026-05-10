# Knowledge Hub Desktop - AI工程师开发计划

**版本**: V1.0
**日期**: 2026-05-03
**预估总工时**: 16小时

---

## 项目概览

### 目标
构建一个完整桌面客户端应用，将用户多台设备上的本地文件、SMB挂载目录、同步目录、各类Agent产出和候选memory统一归集到一个受控、可加密、可审计、可同步的个人AI知识中枢。

### 技术栈
| 组件 | 技术 | 说明 |
|------|------|------|
| 桌面壳 | Tauri 2.0 | 比Electron轻量，Rust后端 |
| UI框架 | React + TypeScript | 前端标准 |
| 本地API | FastAPI (Python) | 异步、高性能 |
| 数据库 | PostgreSQL + pgvector | 向量检索 |
| 队列 | Redis | 任务队列 |
| 文件监听 | Python watchdog | 跨平台 |
| MCP | Python mcp库 | Agent标准接口 |

### 进程架构
```
┌─────────────────────────────────────────────────────────┐
│                  Windows 桌面客户端                        │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Tauri 应用 (用户看到的界面)                          │ │
│  │  ┌───────────────┐  ┌───────────────────────────┐   │ │
│  │  │  React UI      │  │  Service Manager          │   │ │
│  │  │  (Dashboard,   │  │  (启动/停止各服务)          │   │ │
│  │  │   设置, 管理)   │  │                           │   │ │
│  │  └───────┬───────┘  └───────────┬───────────────┘   │ │
│  └──────────┼──────────────────────┼───────────────────┘ │
│             │ HTTP (127.0.0.1)     │ 启动/停止            │
│  ┌──────────▼──────────────────────▼───────────────────┐ │
│  │  后台服务进程 (用户无感，自动管理)                      │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │ │
│  │  │ hub-service  │ │  collector  │ │ mcp-server  │   │ │
│  │  │ (FastAPI)    │ │ (Python)    │ │ (MCP)       │   │ │
│  │  └──────┬──────┘ └─────────────┘ └─────────────┘   │ │
│  │         │                                           │ │
│  │  ┌──────▼──────┐ ┌─────────────┐                   │ │
│  │  │ PostgreSQL  │ │   Redis     │                   │ │
│  │  └─────────────┘ └─────────────┘                   │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## 版本路线图

| 版本 | 功能 | 预估工时 |
|------|------|---------|
| V0.1 | 单机模式、本地目录扫描、基础context.request | 6小时 |
| V0.2 | 多设备模式、设备注册、心跳、文件上传 | 3小时 |
| V0.3 | MCP Server、Agent回流、Review Queue | 3小时 |
| V0.4 | 安全模块、加密、签名、权限 | 2小时 |
| V0.5 | 产品化、打包、文档 | 2小时 |

---

## 详细任务拆分

### 第1小时：项目骨架生成

**并行任务组A** (delegate_task, 3个subagent)

#### 任务A1: 初始化 monorepo 结构
```
输入: 无
输出: 完整目录结构 + README.md + docker-compose.dev.yml
时间: 20分钟

执行:
1. 创建 knowledge-hub-desktop/
2. 创建 apps/desktop (Tauri + React)
3. 创建 apps/hub-service (FastAPI)
4. 创建 apps/worker (Python)
5. 创建 apps/mcp-server
6. 创建 apps/local-collector (Python)
7. 创建 packages/schemas, packages/crypto, packages/shared-types
8. 创建 docker-compose.dev.yml (postgres + pgvector + redis)

验收:
- 目录结构完整
- docker-compose up -d 可以启动数据库
```

#### 任务A2: 初始化 Tauri + React 前端
```
输入: 无
输出: 可运行的 Tauri 应用骨架
时间: 20分钟

执行:
1. cd apps/desktop
2. npx create-tauri-app
3. 配置 React + TypeScript + Vite
4. 安装依赖: react-router, zustand, tailwindcss
5. 创建基础路由结构

验收:
- npm run tauri dev 可以启动
- 显示空白页面
```

#### 任务A3: 初始化 FastAPI 后端
```
输入: 无
输出: 可运行的 FastAPI 应用骨架
时间: 20分钟

执行:
1. cd apps/hub-service
2. 创建 requirements.txt (fastapi, uvicorn, sqlalchemy, pgvector, redis, etc.)
3. 创建 app/main.py (FastAPI app)
4. 创建 app/config.py (配置管理)
5. 创建 app/models/ (SQLAlchemy模型目录)
6. 创建 app/api/ (API路由目录)
7. 创建 app/services/ (业务逻辑目录)

验收:
- uvicorn app.main:app 可以启动
- /docs 可以访问Swagger文档
```

---

### 第2小时：数据库和配置

**并行任务组B** (delegate_task, 2个subagent)

#### 任务B1: 生成数据库迁移脚本
```
输入: 无
输出: 完整的 models.py + migrations/
时间: 30分钟

执行:
1. 创建 apps/hub-service/app/models/base.py (Base model)
2. 创建 apps/hub-service/app/models/workspace.py
3. 创建 apps/hub-service/app/models/device.py
4. 创建 apps/hub-service/app/models/data_source.py
5. 创建 apps/hub-service/app/models/source_file.py
6. 创建 apps/hub-service/app/models/document.py
7. 创建 apps/hub-service/app/models/document_chunk.py
8. 创建 apps/hub-service/app/models/memory.py
9. 创建 apps/hub-service/app/models/memory_candidate.py
10. 创建 apps/hub-service/app/models/artifact.py
11. 创建 apps/hub-service/app/models/event_log.py
12. 创建 Alembic 迁移脚本

验收:
- alembic upgrade head 成功
- 10个表创建成功
- 索引和约束正确
```

#### 任务B2: 生成配置文件和类型定义
```
输入: 无
输出: 共享类型定义 + 配置模板
时间: 30分钟

执行:
1. 创建 packages/schemas/src/ (Pydantic models)
   - workspace.py
   - device.py
   - data_source.py
   - memory.py
   - artifact.py
2. 创建 packages/shared-types/src/ (TypeScript types)
   - workspace.ts
   - device.ts
   - data_source.ts
   - memory.ts
3. 生成配置文件示例
   - desktop-config.json
   - collector-config.json
   - agent-config.json

验收:
- Pydantic models可以导入
- TypeScript types可以编译
- 配置文件格式正确
```

---

### 第3-4小时：核心后端API

**并行任务组C** (delegate_task, 3个subagent)

#### 任务C1: 实现 Workspace + Device API
```
输入: models.py, schemas/
输出: 完整的设备管理API + 测试
时间: 60分钟

执行:
1. 创建 app/api/workspace.py
   - POST /api/workspace (创建)
   - GET /api/workspace/{id} (查询)
2. 创建 app/api/device.py
   - POST /api/invites (邀请码)
   - POST /api/devices/register (注册)
   - POST /api/devices/{id}/approve (审批)
   - POST /api/devices/heartbeat (心跳)
   - GET /api/devices (列表)
   - DELETE /api/devices/{id} (撤销)
3. 创建 app/services/device_service.py
4. 创建 tests/test_device_api.py

验收:
- 所有API端点可调用
- 测试通过
- OpenAPI文档完整
```

#### 任务C2: 实现 DataSource + FileEvent API
```
输入: models.py, schemas/
输出: 数据源管理API + 测试
时间: 60分钟

执行:
1. 创建 app/api/data_source.py
   - CRUD /api/data-sources
2. 创建 app/api/ingest.py
   - POST /api/ingest/file-events
3. 创建 app/api/source_file.py
   - GET /api/source-files
   - GET /api/source-files/{id}
4. 创建 app/services/ingest_service.py
5. 创建 tests/test_ingest_api.py

验收:
- 所有API端点可调用
- 文件事件可以写入数据库
- 测试通过
```

#### 任务C3: 实现 Memory + Artifact API
```
输入: models.py, schemas/
输出: Memory管理API + 测试
时间: 60分钟

执行:
1. 创建 app/api/memory.py
   - CRUD /api/memory
   - GET /api/memory/search (搜索)
2. 创建 app/api/memory_candidate.py
   - POST /api/memory/candidates
   - GET /api/memory/candidates (Review Queue)
   - POST /api/memory/candidates/{id}/approve
   - POST /api/memory/candidates/{id}/reject
3. 创建 app/api/artifact.py
   - POST /api/artifacts
   - GET /api/artifacts
4. 创建 app/services/memory_service.py
5. 创建 tests/test_memory_api.py

验收:
- 所有API端点可调用
- Memory候选可以进入Review Queue
- 测试通过
```

---

### 第5-6小时：Collector + Worker

**并行任务组D** (delegate_task, 2个subagent)

#### 任务D1: 实现 Local Collector
```
输入: 无
输出: 完整的 Collector + 测试
时间: 60分钟

执行:
1. 创建 apps/local-collector/collector/__init__.py
2. 创建 apps/local-collector/collector/main.py (入口)
3. 创建 apps/local-collector/collector/watcher.py (watchdog监听)
4. 创建 apps/local-collector/collector/debouncer.py (防抖)
5. 创建 apps/local-collector/collector/queue.py (SQLite pending queue)
6. 创建 apps/local-collector/collector/hasher.py (SHA256)
7. 创建 apps/local-collector/collector/scanner.py (增量/全量扫描)
8. 创建 apps/local-collector/collector/uploader.py (上传重试)
9. 创建 apps/local-collector/collector/config.py (配置)
10. 创建 apps/local-collector/tests/test_collector.py

功能:
- watchdog 目录监听
- 文件事件防抖 (本地5秒, 同步盘30秒, SMB 60秒)
- SQLite pending queue
- SHA256 hash计算
- include/exclude globs过滤
- 增量扫描 (每6小时) + 全量扫描 (每周)
- 上传重试机制 (指数退避)

验收:
- 可以监听目录变化
- 文件事件写入SQLite
- Hub恢复后可以补传
- 测试通过
```

#### 任务D2: 实现 Worker Pipeline
```
输入: 无
输出: 完整的 Worker + 测试
时间: 60分钟

执行:
1. 创建 apps/worker/app/__init__.py
2. 创建 apps/worker/app/main.py (入口)
3. 创建 apps/worker/app/pipeline.py (处理流水线)
4. 创建 apps/worker/app/parsers/__init__.py
5. 创建 apps/worker/app/parsers/md_parser.py
6. 创建 apps/worker/app/parsers/txt_parser.py
7. 创建 apps/worker/app/parsers/pdf_parser.py (pymupdf)
8. 创建 apps/worker/app/parsers/docx_parser.py (python-docx)
9. 创建 apps/worker/app/chunker.py (文本切块)
10. 创建 apps/worker/app/summarizer.py (摘要生成)
11. 创建 apps/worker/app/embedder.py (Embedding)
12. 创建 apps/worker/app/health_scorer.py (健康度评分)
13. 创建 apps/worker/tests/test_worker.py

功能:
- md/txt/pdf/docx 解析
- 文本切块 (按段落/按token)
- 摘要生成 (调用LLM API)
- Embedding写入pgvector
- 文档健康度评分 (0-100)

验收:
- 可以解析各种格式文件
- chunk写入数据库
- embedding写入pgvector
- 测试通过
```

---

### 第7-8小时：MCP Server + Context Builder + Agent发现配置

**并行任务组E** (delegate_task, 3个subagent)

#### 任务E1: 实现 MCP Server
```
输入: 无
输出: 完整的 MCP Server + 测试
时间: 60分钟

执行:
1. 创建 apps/mcp-server/src/__init__.py
2. 创建 apps/mcp-server/src/main.py (入口)
3. 创建 apps/mcp-server/src/server.py (MCP Server)
4. 创建 apps/mcp-server/src/tools/context_request.py
5. 创建 apps/mcp-server/src/tools/memory_submit.py
6. 创建 apps/mcp-server/src/tools/artifact_submit.py
7. 创建 apps/mcp-server/src/tools/policy_check.py
8. 创建 apps/mcp-server/src/auth.py (Agent token认证)
9. 创建 apps/mcp-server/src/logger.py (调用日志)
10. 创建 apps/mcp-server/tests/test_mcp.py

MCP工具:
- context.request: 请求上下文
- memory.submit_candidate: 提交候选memory
- artifact.submit: 提交产出
- policy.check: 策略检查

验收:
- MCP Server可以启动
- 工具可以调用
- Agent token认证有效
- 测试通过
```

#### 任务E2: 实现 Context Pack Builder
```
输入: 无
输出: Context Pack Builder + 测试
时间: 60分钟

执行:
1. 创建 packages/context-builder/src/__init__.py
2. 创建 packages/context-builder/src/builder.py (主构建器)
3. 创建 packages/context-builder/src/intent.py (意图识别)
4. 创建 packages/context-builder/src/retriever.py (检索器)
5. 创建 packages/context-builder/src/memory_search.py (Memory检索)
6. 创建 packages/context-builder/src/doc_search.py (文档检索)
7. 创建 packages/context-builder/src/template_search.py (模板检索)
8. 创建 packages/context-builder/src/ranker.py (排序)
9. 创建 packages/context-builder/src/compressor.py (压缩)
10. 创建 packages/context-builder/src/filter.py (过滤)
11. 创建 packages/context-builder/tests/test_builder.py

功能:
- 意图识别
- 知识库检索 (关键词+向量)
- Memory检索 (scope+type+embedding+recency)
- 模板/SOP/规则检索
- 去重、排序、压缩、来源标注
- 敏感信息和权限过滤

验收:
- 可以生成Context Pack
- 包含relevant_memory, relevant_docs, templates, rules, warnings
- 测试通过
```

#### 任务E3: 实现 Agent发现与配置
```
输入: 无
输出: Agent发现与配置功能 + 测试
时间: 60分钟

执行:
1. 创建 apps/hub-service/app/services/agent_detector.py
   - 检测已安装的Agent (Hermes, Codex, Gemini, OpenClaw, Cursor, Windsurf)
   - 扫描特征目录和配置文件
   - 分析Agent类型和状态
2. 创建 apps/hub-service/app/services/agent_configurator.py
   - 注入MCP配置到Agent
   - 安装Skill文件
   - 创建输出目录
   - 生成访问Token
   - 注册Agent到Hub
3. 创建 apps/hub-service/app/api/agent_discovery.py
   - GET /api/agents/scan (扫描Agent)
   - POST /api/agents/configure (配置Agent)
   - POST /api/agents/configure-batch (批量配置)
   - GET /api/agents/{id}/status (状态)
   - DELETE /api/agents/{id} (移除配置)
4. 创建 apps/desktop/src/pages/AgentDiscovery.tsx
   - Agent列表展示
   - 一键配置按钮
   - 配置详情弹窗
   - 批量配置支持
5. 创建 apps/hub-service/tests/test_agent_discovery.py

功能:
- 自动检测常见Agent目录
- 显示检测结果和配置状态
- 一键注入MCP配置
- 自动安装Skill
- 生成访问Token
- 创建输出目录

验收:
- 可以检测到已安装的Agent
- 可以一键配置Agent
- 配置后Agent可以调用MCP
- 测试通过
```

---

### 第9-10小时：前端页面

**并行任务组F** (delegate_task, 3个subagent)

#### 任务F1: 实现 Dashboard + 首次启动向导
```
输入: API endpoints
输出: Dashboard + SetupWizard 组件
时间: 60分钟

执行:
1. 创建 apps/desktop/src/pages/Dashboard.tsx
   - 显示: 模式、Hub状态、Collector状态、MCP状态
   - 显示: 文档数、Memory数、设备数、待审核数
   - 显示: 最近错误
   - 操作: 启动/停止Hub、添加数据源、生成邀请码
2. 创建 apps/desktop/src/pages/SetupWizard.tsx
   - 步骤1: 选择模式 (单机/多设备)
   - 步骤2: 配置数据目录
   - 步骤3: 选择要监听的目录
   - 步骤4: 完成
3. 创建 apps/desktop/src/components/StatusCard.tsx
4. 创建 apps/desktop/src/components/StatsGrid.tsx
5. 创建 apps/desktop/src/hooks/useDashboard.ts

验收:
- Dashboard正确显示状态
- SetupWizard可以完成配置
- 适配1366x768和1920x1080
```

#### 任务F2: 实现数据源 + 设备管理页
```
输入: API endpoints
输出: 数据源 + 设备管理组件
时间: 60分钟

执行:
1. 创建 apps/desktop/src/pages/DataSources.tsx
   - 数据源列表
   - 添加/编辑/删除数据源
   - 立即扫描
   - 查看错误
2. 创建 apps/desktop/src/pages/Devices.tsx
   - 设备列表
   - 生成邀请码
   - 审批设备
   - 暂停/恢复/撤销设备
3. 创建 apps/desktop/src/components/DataSourceForm.tsx
4. 创建 apps/desktop/src/components/DeviceCard.tsx
5. 创建 apps/desktop/src/components/InviteCodeModal.tsx

验收:
- 数据源CRUD正常
- 设备管理正常
- 邀请码可以生成和复制
```

#### 任务F3: 实现 Memory + Agent管理页
```
输入: API endpoints
输出: Memory + Agent管理组件
时间: 60分钟

执行:
1. 创建 apps/desktop/src/pages/Memory.tsx
   - Active Memory列表
   - Candidate Memory列表 (Review Queue)
   - 过滤: scope, type, agent_id, status
   - 操作: approve, reject, merge, deprecate
2. 创建 apps/desktop/src/pages/Agents.tsx
   - Agent列表
   - 添加Agent
   - 配置输出目录
   - 复制MCP配置
   - 查看调用日志
3. 创建 apps/desktop/src/pages/AgentDiscovery.tsx
   - 扫描Agent
   - 一键配置
   - 配置详情
4. 创建 apps/desktop/src/components/MemoryCard.tsx
5. 创建 apps/desktop/src/components/ReviewQueue.tsx

验收:
- Memory列表和操作正常
- Agent管理正常
- Agent发现与配置正常
```

---

### 第11-12小时：安全模块

**并行任务组G** (delegate_task, 2个subagent)

#### 任务G1: 实现加密和签名
```
输入: 无
输出: 加密库 + 测试
时间: 60分钟

执行:
1. 创建 packages/crypto/src/__init__.py
2. 创建 packages/crypto/src/keys.py (密钥生成)
   - Ed25519密钥对生成
   - 密钥存储和加载
3. 创建 packages/crypto/src/signing.py (请求签名)
   - 签名生成 (HMAC-SHA256)
   - 签名验证
   - Nonce防重放 (10分钟内不重复)
4. 创建 packages/crypto/src/encryption.py (加密)
   - AES-256-GCM加密
   - AES-256-GCM解密
   - 密钥派生 (HKDF)
5. 创建 packages/crypto/src/token.py (Token管理)
   - Token生成
   - Token验证
   - Token撤销
6. 创建 packages/crypto/tests/test_crypto.py

验收:
- 密钥生成正常
- 签名和验证正常
- 加密和解密正常
- 测试通过
```

#### 任务G2: 实现权限和审计
```
输入: 无
输出: 权限系统 + 测试
时间: 60分钟

执行:
1. 创建 apps/hub-service/app/auth/__init__.py
2. 创建 apps/hub-service/app/auth/middleware.py (认证中间件)
   - Token验证
   - 签名验证
   - 设备状态检查
3. 创建 apps/hub-service/app/auth/permissions.py (权限管理)
   - Device权限
   - Agent权限
   - Scope规则
4. 创建 apps/hub-service/app/auth/audit.py (审计日志)
   - 记录所有写操作
   - 记录权限拒绝
   - 记录安全事件
5. 创建 apps/hub-service/tests/test_auth.py

验收:
- 未签名请求被拒绝
- 重放请求被拒绝
- revoked device被拒绝
- 审计日志完整
- 测试通过
```

---

### 第13-14小时：集成测试 + Docker

**并行任务组H** (delegate_task, 2个subagent)

#### 任务H1: 端到端测试
```
输入: 所有模块
输出: 完整的E2E测试套件
时间: 60分钟

执行:
1. 创建 tests/e2e/conftest.py (fixtures)
2. 创建 tests/e2e/test_standalone.py (单机模式)
   - 创建workspace
   - 添加数据源
   - 触发扫描
   - 验证文档索引
   - 请求context
   - 提交memory
3. 创建 tests/e2e/test_multi_device.py (多设备)
   - 生成邀请码
   - 注册设备
   - 审批设备
   - 上传文件事件
   - 验证跨设备访问
4. 创建 tests/e2e/test_agent_flow.py (Agent回流)
   - 提交candidate memory
   - 验证进入Review Queue
   - 批准memory
   - 验证context可以检索到
5. 创建 tests/e2e/test_security.py (安全)
   - 签名验证
   - 重放检测
   - 设备撤销

验收:
- 所有E2E测试通过
- 覆盖happy path和failure path
```

#### 任务H2: Docker部署配置
```
输入: 无
输出: 完整的Docker部署方案
时间: 60分钟

执行:
1. 创建 apps/hub-service/Dockerfile
2. 创建 apps/worker/Dockerfile
3. 创建 apps/mcp-server/Dockerfile
4. 创建 apps/local-collector/Dockerfile
5. 创建 docker-compose.prod.yml
6. 创建 scripts/start.sh (启动脚本)
7. 创建 scripts/stop.sh (停止脚本)
8. 创建 scripts/backup.sh (备份脚本)

验收:
- docker-compose up -d 启动所有服务
- 所有服务健康检查通过
- 备份脚本可以正常工作
```

---

### 第15-16小时：文档 + 打包

**并行任务组I** (delegate_task, 2个subagent)

#### 任务I1: 生成API文档
```
输入: 所有API endpoints
输出: 完整文档
时间: 60分钟

执行:
1. 自动生成OpenAPI spec
2. 创建 docs/api/README.md
3. 创建 docs/user-guide.md (用户手册)
4. 创建 docs/dev-guide.md (开发者手册)
5. 更新 README.md

验收:
- OpenAPI文档完整
- 用户手册清晰
- 开发者手册可用
```

#### 任务I2: 生成安装包
```
输入: 所有模块
输出: 可分发的安装包
时间: 60分钟

执行:
1. 配置 Tauri build
2. 创建 NSIS安装脚本
3. 打包hub-service, worker, mcp-server, collector
4. 打包PostgreSQL和Redis (便携版)
5. 生成 KnowledgeHub-Setup.exe

验收:
- 安装包可以正常安装
- 所有服务自动启动
- Dashboard可以访问
```

---

## 任务依赖图

```
第1小时: [A1, A2, A3] 并行
    ↓
第2小时: [B1, B2] 并行 (依赖A1)
    ↓
第3-4小时: [C1, C2, C3] 并行 (依赖B1)
    ↓
第5-6小时: [D1, D2] 并行 (依赖C1,C2)
           [E1, E2, E3] 并行 (依赖C1,C3)
    ↓
第7-8小时: [F1, F2, F3] 并行 (依赖C1,C2,C3)
    ↓
第9-10小时: [G1, G2] 并行 (依赖C1)
    ↓
第11-12小时: [H1, H2] 并行 (依赖所有)
    ↓
第13-14小时: [I1, I2] 并行 (依赖H1,H2)
```

---

## 关键配置

### hub-service/config.py
```python
DATABASE_URL = "postgresql+asyncpg://user:pass@localhost:5432/knowledge_hub"
REDIS_URL = "redis://localhost:6379"
EMBEDDING_API = "https://api.openai.com/v1/embeddings"
EMBEDDING_MODEL = "text-embedding-3-small"
```

### desktop-config.json
```json
{
  "app_version": "0.1.0",
  "mode": "standalone",
  "hub_url": "https://127.0.0.1:8443",
  "mcp_url": "https://127.0.0.1:8444/mcp",
  "data_dir": "~/.knowledge-hub"
}
```

### agent-config.json
```json
{
  "agent_id": "hermes",
  "display_name": "Hermes Agent",
  "permissions": ["context_read", "memory_candidate_write", "artifact_write"],
  "output_dirs": ["C:/Users/user/.knowledge-hub/outputs/hermes"],
  "memory_policy": {
    "allow_candidate_submit": true,
    "default_scope": "project",
    "require_review": true
  }
}
```

---

## Agent发现与配置功能

### 支持的Agent
| Agent | 检测特征 | 配置方式 |
|-------|---------|---------|
| Hermes | ~/.hermes/ | MCP + Skill |
| Codex | ~/.codex/ | MCP + Skill |
| Gemini CLI | ~/.gemini/ | MCP + Skill |
| OpenClaw | ~/.openclaw/ | MCP + Skill |
| Cursor | ~/.cursor/ | MCP (无Skill) |
| Windsurf | ~/.windsurf/ | MCP (无Skill) |

### 配置流程
```
1. 用户打开 "Agent发现与配置" 页面
2. 点击 "扫描Agent"
3. 系统检测已安装的Agent
4. 显示检测结果和配置状态
5. 用户选择要配置的Agent
6. 点击 "一键配置"
7. 系统执行:
   - 生成访问Token
   - 注入MCP配置
   - 安装Skill文件
   - 创建输出目录
   - 注册Agent到Hub
8. 配置完成，Agent可以使用Knowledge Hub
```

---

## 多设备协作架构

### 设备角色
| 角色 | 说明 | 进程 |
|------|------|------|
| Hub Device | 中心节点，权威数据源 | hub-service, postgres, redis, worker, mcp-server |
| Client Device | 客户端，只缓存和上传 | desktop, collector |
| Standalone Device | 单机模式，所有进程本地运行 | 全部 |

### 数据流
```
分布式采集:
Client A Collector ──┐
Client B Collector ──┼──▶ Hub Service ──▶ PostgreSQL
Client C Collector ──┘

集中式查询:
Agent A ──┐
Agent B ──┼──▶ MCP Server ──▶ Hub Service ──▶ PostgreSQL
Agent C ──┘

Memory回流:
Agent A ──┐
Agent B ──┼──▶ Hub Service ──▶ Review Queue ──▶ 用户审核 ──▶ Memory
Agent C ──┘
```

### 离线支持
```
离线时:
- Collector继续监听本地文件
- 事件写入SQLite pending queue
- Agent请求context使用本地缓存 (标记not_fresh)

恢复连接时:
- 检测Hub可达
- 按顺序补传pending events
- 清空pending queue
```

---

## 验收标准

### V0.1 MVP (单机模式)
- [ ] 用户安装并打开客户端
- [ ] 选择本机模式
- [ ] 添加一个本地目录
- [ ] 系统扫描并索引 md/txt/pdf/docx
- [ ] 用户能在Dashboard看到文档数量
- [ ] Agent能通过MCP调用context.request
- [ ] 用户能手动添加memory
- [ ] 用户能查看active memory

### V0.2 多设备模式
- [ ] 迷你主机客户端选择中心模式
- [ ] 笔记本客户端通过邀请码申请加入
- [ ] 中心客户端批准设备
- [ ] 笔记本配置一个本地目录
- [ ] 笔记本Collector上传文件事件
- [ ] 中心能看到来自该设备的文件
- [ ] 笔记本Agent能请求中心context
- [ ] 中心撤销设备后，该设备不能继续请求context

### V0.3 Agent回流
- [ ] Codex或Gemini CLI写入memory-export.json
- [ ] Collector捕获文件
- [ ] Hub生成candidate memory
- [ ] Dashboard显示待审核
- [ ] 用户批准后成为active memory
- [ ] 下一次context.request能检索到该memory

### V0.4 安全
- [ ] 多设备请求必须签名
- [ ] revoked device请求失败
- [ ] 重放请求失败
- [ ] memory.content加密存储
- [ ] Dashboard能显示安全状态

### V0.5 产品化
- [ ] 完整Dashboard
- [ ] Windows安装包
- [ ] 托盘常驻
- [ ] 备份恢复
- [ ] 日志和诊断

---

## 风险和注意事项

1. **SMB文件监听可能漏报** - 必须使用定时扫描兜底
2. **向量检索性能** - 需要合理设计索引和查询策略
3. **大文件处理** - 需要流式处理和分块上传
4. **跨平台兼容** - Windows/Mac/Linux路径差异
5. **加密密钥管理** - 需要安全的密钥存储方案
6. **Agent兼容性** - 不同Agent的配置方式可能不同

---

## 总结

| 阶段 | 任务数 | 预估工时 | 并行度 |
|------|--------|---------|--------|
| 项目骨架 | 3 | 1小时 | 3 |
| 数据库+配置 | 2 | 1小时 | 2 |
| 后端API | 3 | 2小时 | 3 |
| Collector+Worker | 2 | 2小时 | 2 |
| MCP+Context+Agent配置 | 3 | 2小时 | 3 |
| 前端页面 | 3 | 2小时 | 3 |
| 安全模块 | 2 | 2小时 | 2 |
| 测试+部署 | 2 | 2小时 | 2 |
| 文档+打包 | 2 | 2小时 | 2 |
| **总计** | **22** | **16小时** | **平均2.4** |

---

## 附录：目录结构

```
knowledge-hub-desktop/
├── apps/
│   ├── desktop/                    # Tauri + React 前端
│   │   ├── src-tauri/              # Tauri Rust代码
│   │   ├── src/
│   │   │   ├── pages/              # 页面组件
│   │   │   ├── components/         # 通用组件
│   │   │   ├── hooks/              # React Hooks
│   │   │   ├── stores/             # Zustand状态
│   │   │   └── utils/              # 工具函数
│   │   └── package.json
│   │
│   ├── hub-service/                # FastAPI 后端
│   │   ├── app/
│   │   │   ├── api/                # API路由
│   │   │   ├── models/             # SQLAlchemy模型
│   │   │   ├── services/           # 业务逻辑
│   │   │   ├── auth/               # 认证和权限
│   │   │   └── main.py             # 入口
│   │   ├── migrations/             # Alembic迁移
│   │   ├── tests/                  # 测试
│   │   ├── requirements.txt
│   │   └── Dockerfile
│   │
│   ├── worker/                     # 文件解析Worker
│   │   ├── app/
│   │   │   ├── parsers/            # 文件解析器
│   │   │   ├── pipeline.py         # 处理流水线
│   │   │   └── main.py             # 入口
│   │   └── Dockerfile
│   │
│   ├── mcp-server/                 # MCP Server
│   │   ├── src/
│   │   │   ├── tools/              # MCP工具
│   │   │   ├── server.py           # Server实现
│   │   │   └── main.py             # 入口
│   │   └── Dockerfile
│   │
│   └── local-collector/            # 文件监听Collector
│       ├── collector/
│       │   ├── watcher.py          # watchdog监听
│       │   ├── queue.py            # SQLite队列
│       │   └── main.py             # 入口
│       └── tests/
│
├── packages/
│   ├── schemas/                    # Pydantic schemas
│   ├── crypto/                     # 加密库
│   ├── context-builder/            # Context Pack Builder
│   └── shared-types/               # TypeScript类型
│
├── docs/
│   ├── api/                        # API文档
│   └── architecture/               # 架构文档
│
├── docker-compose.dev.yml          # 开发环境
├── docker-compose.prod.yml         # 生产环境
├── README.md
└── DEVELOPMENT_PLAN.md             # 本文件
```
