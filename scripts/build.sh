#!/bin/bash
# Knowledge Hub Desktop - 构建脚本

set -e

echo "=========================================="
echo "Knowledge Hub Desktop - Build Script"
echo "=========================================="

# 检查依赖
echo "Checking dependencies..."

if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed"
    exit 1
fi

echo "Dependencies OK"

# 安装npm依赖
echo ""
echo "Installing npm dependencies..."
cd apps/desktop
npm install
cd ../..

# 构建Rust服务
echo ""
echo "Building Rust services..."

echo "  Building hub-service..."
cd apps/hub-service
cargo build --release
cd ../..

echo "  Building collector..."
cd apps/collector
cargo build --release
cd ../..

echo "  Building mcp-server..."
cd apps/mcp-server
cargo build --release
cd ../..

echo "  Building crypto..."
cd packages/crypto
cargo build --release
cd ../..

# 构建Tauri应用
echo ""
echo "Building Tauri desktop app..."
cd apps/desktop
npm run tauri build
cd ../..

echo ""
echo "=========================================="
echo "Build completed!"
echo "=========================================="
echo ""
echo "Output files:"
echo "  - apps/desktop/src-tauri/target/release/knowledge-hub-desktop.exe"
echo "  - apps/hub-service/target/release/hub-service.exe"
echo "  - apps/collector/target/release/collector.exe"
echo "  - apps/mcp-server/target/release/mcp-server.exe"
