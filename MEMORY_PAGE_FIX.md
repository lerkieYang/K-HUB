# Memory 页面空白问题诊断

## 问题原因

Memory 页面需要连接 hub-service（后端 API），但 hub-service 可能没有运行。

## 诊断步骤

### 1. 检查 hub-service 是否运行

```bash
# 在 Windows PowerShell 中执行
netstat -ano | findstr 8443

# 或者在 WSL 中执行
cmd.exe /c "netstat -ano | findstr 8443"
```

如果没有输出，说明 hub-service 没有运行。

### 2. 启动 hub-service

```bash
# 进入 hub-service 目录
cd /mnt/c/Users/user/OneDrive/Desktop/knowledgeHUB/apps/hub-service

# 编译（如果还没有编译）
/mnt/c/Users/user/.cargo/bin/cargo.exe build

# 运行
DATABASE_URL="sqlite:./data/knowledge-hub.db" ./target/debug/hub-service.exe
```

### 3. 验证 hub-service 运行

```bash
# 测试健康检查
curl http://127.0.0.1:8443/health

# 应该返回 "ok"
```

### 4. 测试 Memory API

```bash
# 测试 Memory 列表 API
curl http://127.0.0.1:8443/api/memory?limit=5

# 应该返回 JSON 格式的 memory 列表
```

## 快速修复

### 方案 1：启动 hub-service（推荐）

1. 打开一个新的 PowerShell 或终端窗口
2. 运行 hub-service：
   ```bash
   cd C:\Users\user\OneDrive\Desktop\knowledgeHUB\apps\hub-service
   set DATABASE_URL=sqlite:./data/knowledge-hub.db
   .\target\debug\hub-service.exe
   ```
3. 保持窗口打开，不要关闭
4. 刷新前端页面

### 方案 2：检查前端连接

如果 hub-service 已经运行，但 Memory 页面仍然空白：

1. 打开浏览器开发者工具（F12）
2. 查看 Console 标签页的错误信息
3. 查看 Network 标签页，检查 `/api/memory` 请求是否成功

## 数据库状态

数据库已有 200 条 memory 记录，数据是完整的。

问题只是 hub-service 没有运行，导致前端无法获取数据。
