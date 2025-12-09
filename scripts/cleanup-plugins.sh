#!/bin/bash
# 清理所有插件进程和 socket 文件 / Cleanup all plugin processes and socket files
# 
# 用法 / Usage:
#   ./scripts/cleanup-plugins.sh

set -e

# 颜色定义 / Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

info "🧹 开始清理插件进程和 socket 文件 / Starting cleanup of plugin processes and socket files"

# 1. 查找并杀死所有插件相关进程 / Find and kill all plugin-related processes
info "🔍 查找插件进程 / Finding plugin processes"

# 查找所有包含 "plugin" 的 cargo run 进程 / Find all cargo run processes containing "plugin"
PLUGIN_PIDS=$(pgrep -f "cargo.*plugin" || true)

if [ -n "$PLUGIN_PIDS" ]; then
    warn "发现以下插件进程 / Found plugin processes:"
    echo "$PLUGIN_PIDS" | while read pid; do
        ps -p $pid -o pid,command | tail -n +2
    done
    
    echo "$PLUGIN_PIDS" | while read pid; do
        info "终止进程 / Killing process: $pid"
        kill -TERM $pid 2>/dev/null || true
    done
    
    # 等待进程退出 / Wait for processes to exit
    sleep 2
    
    # 检查是否还有残留 / Check for remaining processes
    REMAINING=$(pgrep -f "cargo.*plugin" || true)
    if [ -n "$REMAINING" ]; then
        warn "强制终止残留进程 / Force killing remaining processes"
        echo "$REMAINING" | while read pid; do
            kill -9 $pid 2>/dev/null || true
        done
    fi
    
    success "插件进程已清理 / Plugin processes cleaned"
else
    info "未发现插件进程 / No plugin processes found"
fi

# 2. 查找并杀死 v-connect-im 进程 / Find and kill v-connect-im processes
info "🔍 查找 v-connect-im 进程 / Finding v-connect-im processes"

IM_PIDS=$(pgrep -f "v-connect-im" | grep -v $$ || true)

if [ -n "$IM_PIDS" ]; then
    warn "发现以下 v-connect-im 进程 / Found v-connect-im processes:"
    echo "$IM_PIDS" | while read pid; do
        ps -p $pid -o pid,command | tail -n +2
    done
    
    echo "$IM_PIDS" | while read pid; do
        info "终止进程 / Killing process: $pid"
        kill -TERM $pid 2>/dev/null || true
    done
    
    sleep 2
    
    # 检查是否还有残留 / Check for remaining processes
    REMAINING=$(pgrep -f "v-connect-im" | grep -v $$ || true)
    if [ -n "$REMAINING" ]; then
        warn "强制终止残留进程 / Force killing remaining processes"
        echo "$REMAINING" | while read pid; do
            kill -9 $pid 2>/dev/null || true
        done
    fi
    
    success "v-connect-im 进程已清理 / v-connect-im processes cleaned"
else
    info "未发现 v-connect-im 进程 / No v-connect-im processes found"
fi

# 3. 清理 socket 文件 / Cleanup socket files
info "🧹 清理 socket 文件 / Cleaning up socket files"

# 清理项目目录下的 socket 文件 / Clean socket files in project directory
SOCKET_DIRS=(
    "$HOME/vp/sockets"
    "./v-connect-im/plugins/sockets"
    "./plugins/sockets"
    "./dist/v-connect-im/plugins/sockets"
)

for dir in "${SOCKET_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        SOCK_FILES=$(find "$dir" -name "*.sock" 2>/dev/null || true)
        if [ -n "$SOCK_FILES" ]; then
            warn "清理 socket 文件 / Cleaning socket files in: $dir"
            find "$dir" -name "*.sock" -delete 2>/dev/null || true
            success "已清理 / Cleaned: $dir"
        fi
    fi
done

# 4. 清理临时文件 / Cleanup temporary files
info "🧹 清理临时文件 / Cleaning up temporary files"

# 清理 cargo 的临时构建文件 / Clean cargo temporary build files
if [ -d "target/debug" ]; then
    find target/debug -name "*plugin*" -type f -delete 2>/dev/null || true
fi

success "✨ 清理完成 / Cleanup completed!"
echo ""
info "📋 建议的后续步骤 / Recommended next steps:"
echo "  1. 检查配置文件,确保只有需要的插件 / Check config files, ensure only needed plugins"
echo "  2. 重新启动 v-connect-im / Restart v-connect-im"
echo "  3. 验证只有配置的插件在运行 / Verify only configured plugins are running"
echo ""
info "💡 提示 / Tips:"
echo "  - 查看运行的进程: ps aux | grep -E 'plugin|v-connect-im'"
echo "  - 查看 socket 文件: ls -la ~/vp/sockets/"
