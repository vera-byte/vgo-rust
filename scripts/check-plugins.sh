#!/bin/bash
# 检查插件状态 / Check plugin status
# 
# 用法 / Usage:
#   ./scripts/check-plugins.sh

# 颜色定义 / Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warn() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

section() {
    echo -e "\n${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

section "🔍 插件状态检查 / Plugin Status Check"

# 1. 检查运行的进程 / Check running processes
section "1️⃣  运行中的进程 / Running Processes"

info "v-connect-im 进程 / v-connect-im processes:"
V_PIDS=$(pgrep -f "v-connect-im" || true)
if [ -n "$V_PIDS" ]; then
    ps aux | grep -E "v-connect-im" | grep -v grep | grep -v "check-plugins"
    success "发现 $(echo "$V_PIDS" | wc -l) 个进程 / Found $(echo "$V_PIDS" | wc -l) process(es)"
else
    warn "未发现 v-connect-im 进程 / No v-connect-im processes found"
fi

echo ""
info "插件进程 / Plugin processes:"
PLUGIN_PIDS=$(pgrep -f "plugin" | grep -v $$ || true)
if [ -n "$PLUGIN_PIDS" ]; then
    ps aux | grep -E "plugin" | grep -v grep | grep -v "check-plugins"
    warn "发现 $(echo "$PLUGIN_PIDS" | wc -l) 个插件进程 / Found $(echo "$PLUGIN_PIDS" | wc -l) plugin process(es)"
else
    success "未发现插件进程 / No plugin processes found"
fi

# 2. 检查 socket 文件 / Check socket files
section "2️⃣  Socket 文件 / Socket Files"

SOCKET_DIRS=(
    "$HOME/vp/sockets"
    "./v-connect-im/plugins/sockets"
    "./plugins/sockets"
    "./dist/v-connect-im/plugins/sockets"
)

FOUND_SOCKETS=0
for dir in "${SOCKET_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        SOCK_FILES=$(find "$dir" -name "*.sock" 2>/dev/null || true)
        if [ -n "$SOCK_FILES" ]; then
            info "发现 socket 文件在 / Found socket files in: $dir"
            ls -lh "$dir"/*.sock 2>/dev/null || true
            FOUND_SOCKETS=$((FOUND_SOCKETS + 1))
        fi
    fi
done

if [ $FOUND_SOCKETS -eq 0 ]; then
    success "未发现 socket 文件 / No socket files found"
fi

# 3. 检查配置文件 / Check configuration files
section "3️⃣  配置文件 / Configuration Files"

CONFIG_FILES=(
    "./v-connect-im/config/default.toml"
    "./config/production.toml"
    "./dist/v-connect-im/config/production.toml"
)

for config in "${CONFIG_FILES[@]}"; do
    if [ -f "$config" ]; then
        info "配置文件 / Config file: $config"
        echo ""
        echo "  dev_plugins:"
        grep -A 5 "dev_plugins" "$config" 2>/dev/null | sed 's/^/    /' || echo "    (未找到 / not found)"
        echo ""
        echo "  install:"
        grep -A 5 "^install" "$config" 2>/dev/null | sed 's/^/    /' || echo "    (未找到 / not found)"
        echo ""
    fi
done

# 4. 检查端口占用 / Check port usage
section "4️⃣  端口占用 / Port Usage"

PORTS=(8080 8081)
for port in "${PORTS[@]}"; do
    info "检查端口 / Checking port: $port"
    lsof -i :$port 2>/dev/null || echo "  端口未被占用 / Port not in use"
done

# 5. 提供建议 / Provide recommendations
section "💡 建议 / Recommendations"

if [ -n "$PLUGIN_PIDS" ]; then
    warn "发现未预期的插件进程 / Found unexpected plugin processes"
    echo "  建议执行 / Recommended action:"
    echo "    ./scripts/cleanup-plugins.sh"
    echo ""
fi

if [ $FOUND_SOCKETS -gt 0 ]; then
    info "发现 socket 文件 / Found socket files"
    echo "  这些文件会在服务启动时被使用 / These files will be used when service starts"
    echo "  如需清理 / To cleanup:"
    echo "    ./scripts/cleanup-plugins.sh"
    echo ""
fi

section "✨ 检查完成 / Check Completed"

echo "运行以下命令进行操作 / Run these commands for actions:"
echo ""
echo "  清理所有插件 / Cleanup all plugins:"
echo "    ./scripts/cleanup-plugins.sh"
echo ""
echo "  启动服务 / Start service:"
echo "    cd v-connect-im && cargo run"
echo ""
echo "  查看实时日志 / View live logs:"
echo "    tail -f v-connect-im/logs/*.log"
