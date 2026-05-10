# Knowledge Hub Desktop - 项目总结

## 项目概述

Knowledge Hub Desktop 是一个个人AI知识中枢桌面应用，用于统一管理多设备文件、Agent产出和Memory。

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                  Windows 桌面客户端                        │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Tauri Shell (Rust + React)                         │ │
│  │  - Dashboard, Settings, Agent Discovery             │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Hub Service (Rust)                                 │ │
│  │  - REST API (axum)                                  │ │
│  │  - SQLite/PostgreSQL                                │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Collector (Rust)                                   │ │
│  │  - 文件监听                                          │ │
│  │  - Agent产出收集                                     │ │
│  │  - MCP代理                                          │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  MCP Server (Rust)                                  │ │
│  │  - 标准MCP协议                                       │ │
│  │  - Agent接口                                        │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## 文件结构

```
knowledgeHUB/
├── apps/
│   ├── desktop/                    # Tauri + React 前端
│   │   ├── src/                    # React源码
│   │   │   ├── pages/              # 页面组件 (8个)
│   │   │   ├── stores/             # Zustand状态管理
│   │   │   └── config/             # 配置
│   │   └── src-tauri/              # Tauri Rust后端
│   │
│   ├── hub-service/                # Hub Service (Rust)
│   │   └── src/
│   │       ├── api/                # API路由 (9个模块)
│   │       ├── models/             # 数据模型 (10个)
│   │       ├── services/           # 业务逻辑
│   │       └── db/                 # 数据库抽象层
│   │
│   ├── collector/                  # Collector (Rust)
│   │   └── src/
│   │       ├── watcher/            # 文件监听
│   │       ├── queue/              # 本地队列
│   │       ├── uploader/           # 上传模块
│   │       ├── agent_watcher/      # Agent产出监听
│   │       └── proxy/              # MCP代理
│   │
│   └── mcp-server/                 # MCP Server (Rust)
│       └── src/
│           ├── tools.rs            # MCP工具
│           └── auth.rs             # 认证
│
├── packages/
│   ├── shared-types/               # TypeScript类型定义
│   │   └── src/
│   │       ├── workspace.ts
│   │       ├── device.ts
│   │       ├── memory.ts
│   │       └── ...
│   │
│   └── crypto/                     # 加密库 (Rust)
│       └── src/
│           ├── keys.rs             # 密钥生成
│           ├── token.rs            # Token管理
│           └── signing.rs          # 请求签名
│
├── tests/                          # 测试
│   ├── unit/                       # 单元测试
│   └── e2e/                        # E2E测试
│
├── scripts/                        # 脚本
│   ├── build.sh                    # 构建脚本
│   └── start.sh                    # 启动脚本
│
├── docs/                           # 文档
├── README.md                       # 项目说明
├── CHANGELOG.md                    # 版本记录
├── DEVELOPMENT_PLAN.md             # 开发计划
└── .gitignore                      # Git忽略文件
```

## API接口

### Hub Service API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /health | 健康检查 |
| POST | /api/workspace | 创建workspace |
| GET | /api/workspace/:id | 获取workspace |
| POST | /api/invites | 创建邀请码 |
| GET | /api/devices | 设备列表 |
| POST | /api/devices/register | 注册设备 |
| POST | /api/devices/:id/approve | 批准设备 |
| POST | /api/devices/heartbeat | 心跳 |
| GET | /api/data-sources | 数据源列表 |
| POST | /api/data-sources | 创建数据源 |
| POST | /api/data-sources/:id/scan | 触发扫描 |
| GET | /api/memory | Memory列表 |
| POST | /api/memory | 创建Memory |
| GET | /api/memory/search | 搜索Memory |
| GET | /api/memory/candidates | 待审核列表 |
| POST | /api/memory/candidates | 提交候选 |
| POST | /api/memory/candidates/:id/approve | 批准候选 |
| POST | /api/memory/candidates/:id/reject | 拒绝候选 |
| GET | /api/artifacts | Artifact列表 |
| POST | /api/artifacts | 创建Artifact |
| POST | /api/ingest/file-events | 接收文件事件 |
| POST | /api/context/request | 请求上下文 |
| GET | /api/agents | Agent列表 |
| GET | /api/agents/scan | 扫描Agent |
| POST | /api/agents/configure | 配置Agent |
| POST | /api/sync/push | 推送同步 |
| POST | /api/sync/pull | 拉取同步 |

### MCP工具

| 工具 | 说明 |
|------|------|
| context.request | 请求上下文 |
| memory.submit_candidate | 提交候选memory |
| artifact.submit | 提交产出 |
| policy.check | 策略检查 |

## 运行模式

| 模式 | 说明 | 数据库 | 服务 |
|------|------|--------|------|
| Standalone | 单机模式 | SQLite | 全部本地 |
| Hub | 中心节点 | PostgreSQL | 完整服务 + 同步 |
| Client | 客户端 | 本地缓存 | Collector + 代理 |

## 下一步

1. 完善Worker文件解析模块
2. 优化Context Pack Builder
3. 实现模式切换数据迁移
4. 打包Windows安装程序
5. 编写用户手册
6. 性能测试和优化
