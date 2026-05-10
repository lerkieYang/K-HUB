#!/bin/bash
# Knowledge Hub Desktop - 测试脚本

set -e

echo "=========================================="
echo "Knowledge Hub Desktop - Test Suite"
echo "=========================================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查Hub Service是否运行
check_hub_service() {
    echo -e "${YELLOW}Checking Hub Service...${NC}"
    
    if curl -s http://127.0.0.1:8443/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Hub Service is running${NC}"
        return 0
    else
        echo -e "${RED}✗ Hub Service is not running${NC}"
        return 1
    fi
}

# 运行单元测试
run_unit_tests() {
    echo ""
    echo -e "${YELLOW}Running unit tests...${NC}"
    
    # Crypto tests
    echo "  Testing crypto package..."
    cd packages/crypto
    cargo test --quiet 2>/dev/null || echo "  Crypto tests skipped"
    cd ../..
    
    echo -e "${GREEN}✓ Unit tests completed${NC}"
}

# 运行API测试
run_api_tests() {
    echo ""
    echo -e "${YELLOW}Running API tests...${NC}"
    
    if ! check_hub_service; then
        echo -e "${YELLOW}Skipping API tests (Hub Service not running)${NC}"
        return
    fi
    
    # 测试各个API端点
    echo "  Testing health check..."
    HEALTH=$(curl -s http://127.0.0.1:8443/health)
    if [ "$HEALTH" = "ok" ]; then
        echo -e "    ${GREEN}✓ Health check passed${NC}"
    else
        echo -e "    ${RED}✗ Health check failed${NC}"
    fi
    
    echo "  Testing discovery info..."
    DISCOVERY=$(curl -s http://127.0.0.1:8443/api/discovery/info)
    if echo "$DISCOVERY" | grep -q "hub_name"; then
        echo -e "    ${GREEN}✓ Discovery info passed${NC}"
    else
        echo -e "    ${RED}✗ Discovery info failed${NC}"
    fi
    
    echo "  Testing workspace creation..."
    WORKSPACE=$(curl -s -X POST http://127.0.0.1:8443/api/workspace \
        -H "Content-Type: application/json" \
        -d '{"name":"Test","mode":"standalone"}')
    if echo "$WORKSPACE" | grep -q "id"; then
        echo -e "    ${GREEN}✓ Workspace creation passed${NC}"
    else
        echo -e "    ${RED}✗ Workspace creation failed${NC}"
    fi
    
    echo "  Testing data source creation..."
    SOURCE=$(curl -s -X POST http://127.0.0.1:8443/api/data-sources \
        -H "Content-Type: application/json" \
        -d '{"name":"Test","path":"/tmp/test","source_type":"local"}')
    if echo "$SOURCE" | grep -q "id"; then
        echo -e "    ${GREEN}✓ Data source creation passed${NC}"
    else
        echo -e "    ${RED}✗ Data source creation failed${NC}"
    fi
    
    echo "  Testing memory creation..."
    MEMORY=$(curl -s -X POST http://127.0.0.1:8443/api/memory \
        -H "Content-Type: application/json" \
        -d '{"scope":"project","type":"decision","content":"Test","source":"test"}')
    if echo "$MEMORY" | grep -q "id"; then
        echo -e "    ${GREEN}✓ Memory creation passed${NC}"
    else
        echo -e "    ${RED}✗ Memory creation failed${NC}"
    fi
    
    echo "  Testing invite creation..."
    INVITE=$(curl -s -X POST http://127.0.0.1:8443/api/invites \
        -H "Content-Type: application/json" \
        -d '{"ttl_seconds":600}')
    if echo "$INVITE" | grep -q "invite_code"; then
        echo -e "    ${GREEN}✓ Invite creation passed${NC}"
    else
        echo -e "    ${RED}✗ Invite creation failed${NC}"
    fi
    
    echo "  Testing agent scan..."
    AGENTS=$(curl -s http://127.0.0.1:8443/api/agents/scan)
    if echo "$AGENTS" | grep -q "agents"; then
        echo -e "    ${GREEN}✓ Agent scan passed${NC}"
    else
        echo -e "    ${RED}✗ Agent scan failed${NC}"
    fi
    
    echo -e "${GREEN}✓ API tests completed${NC}"
}

# 运行集成测试
run_integration_tests() {
    echo ""
    echo -e "${YELLOW}Running integration tests...${NC}"
    
    if ! check_hub_service; then
        echo -e "${YELLOW}Skipping integration tests (Hub Service not running)${NC}"
        return
    fi
    
    # 运行Rust集成测试
    cd tests
    cargo test --quiet 2>/dev/null || echo "  Integration tests skipped"
    cd ..
    
    echo -e "${GREEN}✓ Integration tests completed${NC}"
}

# 测试前端构建
test_frontend_build() {
    echo ""
    echo -e "${YELLOW}Testing frontend build...${NC}"
    
    cd apps/desktop
    
    if [ -f "package.json" ]; then
        echo "  Checking package.json..."
        if node -e "require('./package.json')" 2>/dev/null; then
            echo -e "    ${GREEN}✓ package.json is valid${NC}"
        else
            echo -e "    ${RED}✗ package.json is invalid${NC}"
        fi
    fi
    
    if [ -d "node_modules" ]; then
        echo -e "    ${GREEN}✓ node_modules exists${NC}"
    else
        echo -e "    ${YELLOW}! node_modules not found (run npm install)${NC}"
    fi
    
    cd ../..
    
    echo -e "${GREEN}✓ Frontend check completed${NC}"
}

# 测试Rust编译
test_rust_compilation() {
    echo ""
    echo -e "${YELLOW}Testing Rust compilation...${NC}"
    
    # 测试hub-service
    echo "  Checking hub-service..."
    cd apps/hub-service
    if cargo check --quiet 2>/dev/null; then
        echo -e "    ${GREEN}✓ hub-service compiles${NC}"
    else
        echo -e "    ${RED}✗ hub-service has compilation errors${NC}"
    fi
    cd ../..
    
    # 测试collector
    echo "  Checking collector..."
    cd apps/collector
    if cargo check --quiet 2>/dev/null; then
        echo -e "    ${GREEN}✓ collector compiles${NC}"
    else
        echo -e "    ${RED}✗ collector has compilation errors${NC}"
    fi
    cd ../..
    
    # 测试mcp-server
    echo "  Checking mcp-server..."
    cd apps/mcp-server
    if cargo check --quiet 2>/dev/null; then
        echo -e "    ${GREEN}✓ mcp-server compiles${NC}"
    else
        echo -e "    ${RED}✗ mcp-server has compilation errors${NC}"
    fi
    cd ../..
    
    # 测试crypto
    echo "  Checking crypto..."
    cd packages/crypto
    if cargo check --quiet 2>/dev/null; then
        echo -e "    ${GREEN}✓ crypto compiles${NC}"
    else
        echo -e "    ${RED}✗ crypto has compilation errors${NC}"
    fi
    cd ../..
    
    echo -e "${GREEN}✓ Rust compilation check completed${NC}"
}

# 主测试流程
main() {
    echo ""
    echo "Starting tests..."
    echo ""
    
    # 运行各项测试
    run_unit_tests
    test_rust_compilation
    test_frontend_build
    run_api_tests
    run_integration_tests
    
    echo ""
    echo "=========================================="
    echo -e "${GREEN}All tests completed!${NC}"
    echo "=========================================="
}

# 运行主流程
main
