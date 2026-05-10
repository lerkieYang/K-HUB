#!/bin/bash
# Knowledge Hub Desktop - 启动脚本

set -e

echo "Starting Knowledge Hub Desktop..."

# 获取脚本所在目录
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/.."

# 检查可执行文件
if [ ! -f "apps/desktop/src-tauri/target/release/knowledge-hub-desktop.exe" ]; then
    echo "Error: Desktop app not found. Run scripts/build.sh first."
    exit 1
fi

# 启动应用
./apps/desktop/src-tauri/target/release/knowledge-hub-desktop.exe
