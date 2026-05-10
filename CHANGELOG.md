# Changelog

## v0.1.0 (2026-05-03)

### 初始版本 - 核心功能完成

#### 架构
- ✅ 创建monorepo项目结构
- ✅ 实现Tauri + React前端框架
- ✅ 实现Rust核心服务 (Hub Service)
- ✅ 实现Collector收集分发服务
- ✅ 实现MCP Server
- ✅ 实现安全模块 (crypto)
- ✅ 支持SQLite和PostgreSQL双数据库

#### Hub Service (Rust)
- ✅ Workspace API
- ✅ Device API (设备注册、心跳、审批)
- ✅ DataSource API (数据源管理)
- ✅ Memory API (CRUD、搜索、候选审核)
- ✅ Artifact API
- ✅ Ingest API (文件事件接收)
- ✅ Context API (上下文请求)
- ✅ Agent API (Agent发现与配置)
- ✅ Sync API (多设备同步)

#### Collector (Rust)
- ✅ 文件监听 (notify)
- ✅ 防抖机制
- ✅ 本地SQLite队列
- ✅ 上传重试
- ✅ Agent产出监听
- ✅ memory-export.json解析
- ✅ MCP代理

#### MCP Server (Rust)
- ✅ 标准MCP协议支持
- ✅ context.request工具
- ✅ memory.submit_candidate工具
- ✅ artifact.submit工具
- ✅ policy.check工具
- ✅ Token认证

#### 前端 (Tauri + React)
- ✅ Dashboard (统计、状态、快捷操作)
- ✅ SetupWizard (首次启动向导)
- ✅ DataSources (数据源管理)
- ✅ Memory (Memory管理、审核队列)
- ✅ Agents (Agent管理)
- ✅ AgentDiscovery (Agent发现与配置)
- ✅ Devices (设备管理，Hub模式)
- ✅ Settings (设置、模式切换)

#### 安全模块 (crypto)
- ✅ 密钥生成
- ✅ Token生成和验证
- ✅ 请求签名 (HMAC-SHA256)
- ✅ Nonce防重放

#### 共享类型 (TypeScript)
- ✅ workspace, device, data-source
- ✅ memory, artifact, agent
- ✅ config, api

#### 测试
- ✅ 单元测试框架
- ✅ E2E测试框架

#### 文档
- ✅ README.md
- ✅ CHANGELOG.md
- ✅ DEVELOPMENT_PLAN.md

---

### 待完成 (v0.2.0)

- [ ] Worker文件解析 (md/txt/pdf/docx)
- [ ] Context Pack Builder优化 (向量检索)
- [ ] 模式切换数据迁移
- [ ] Windows安装包打包
- [ ] 完整E2E测试
- [ ] 性能优化
- [ ] 用户手册
