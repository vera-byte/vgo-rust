#!/bin/bash
# v-connect-im 生产环境打包脚本 / v-connect-im Production Build Script
# 
# 用法 / Usage:
#   ./scripts/build-release.sh [output_dir]
#   默认输出目录 / Default output: ./dist/v-connect-im
#
# 示例 / Example:
#   ./scripts/build-release.sh ~/deploy/v-connect-im

set -e  # 遇到错误立即退出 / Exit on error

# 颜色定义 / Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息 / Print colored messages
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

# 获取脚本所在目录 / Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 输出目录 / Output directory
OUTPUT_DIR="${1:-$PROJECT_ROOT/dist/v-connect-im}"
VERSION=$(grep '^version' "$PROJECT_ROOT/v-connect-im/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

info "🚀 开始打包 v-connect-im v$VERSION / Starting to build v-connect-im v$VERSION"
info "📁 输出目录 / Output directory: $OUTPUT_DIR"

# 清理旧的输出目录 / Clean old output directory
if [ -d "$OUTPUT_DIR" ]; then
    warn "清理旧的输出目录 / Cleaning old output directory"
    rm -rf "$OUTPUT_DIR"
fi

# 创建输出目录结构 / Create output directory structure
info "📂 创建目录结构 / Creating directory structure"
mkdir -p "$OUTPUT_DIR"/{bin,config,logs,plugins/sockets,data}

# 编译 release 版本 / Build release version
info "🔨 编译 release 版本 / Building release version"
cd "$PROJECT_ROOT"
cargo build --release --package v-connect-im

if [ $? -ne 0 ]; then
    error "编译失败 / Build failed"
    exit 1
fi

success "编译完成 / Build completed"

# 复制二进制文件 / Copy binary
info "📦 复制二进制文件 / Copying binary"
cp "$PROJECT_ROOT/target/release/v-connect-im" "$OUTPUT_DIR/bin/"
chmod +x "$OUTPUT_DIR/bin/v-connect-im"

# 复制配置文件 / Copy configuration files
info "📝 复制配置文件 / Copying configuration files"
cp "$PROJECT_ROOT/v-connect-im/config/default.toml" "$OUTPUT_DIR/config/"

# 创建生产环境配置模板 / Create production config template
cat > "$OUTPUT_DIR/config/production.toml" << 'EOF'
# v-connect-im 生产环境配置 / v-connect-im Production Configuration
# 复制此文件并根据实际环境修改 / Copy this file and modify according to your environment

[server]
host = "0.0.0.0"
port = 8080
ws_port = 8081

[database]
# 配置你的数据库连接 / Configure your database connection
# url = "postgres://user:password@localhost/v_connect_im"

[redis]
# 配置你的 Redis 连接 / Configure your Redis connection
# url = "redis://localhost:6379"

[plugins]
plugin_dir = "./plugins"
socket_path = "./plugins/sockets/runtime.sock"
debug = false
# log_level = "info"

# 生产环境不使用 dev_plugins / Don't use dev_plugins in production
dev_plugins = []

# 安装的插件 / Installed plugins
# install = [
#     "file://./plugins/v-connect-im-plugin-storage-sled.vp",
# ]
EOF

# 创建启动脚本 / Create startup script
info "🚀 创建启动脚本 / Creating startup script"
cat > "$OUTPUT_DIR/start.sh" << 'EOF'
#!/bin/bash
# v-connect-im 启动脚本 / v-connect-im Startup Script

set -e

# 获取脚本所在目录 / Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 检查配置文件 / Check configuration file
if [ ! -f "config/production.toml" ]; then
    echo "❌ 配置文件不存在，请先创建 config/production.toml"
    echo "❌ Configuration file not found, please create config/production.toml first"
    echo "💡 可以复制 config/default.toml 作为模板"
    echo "💡 You can copy config/default.toml as a template"
    exit 1
fi

# 创建必要的目录 / Create necessary directories
mkdir -p logs plugins/sockets data

# 设置环境变量 / Set environment variables
export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

echo "🚀 启动 v-connect-im / Starting v-connect-im"
echo "📁 工作目录 / Working directory: $SCRIPT_DIR"
echo "📊 日志级别 / Log level: $RUST_LOG"

# 启动服务 / Start service
exec ./bin/v-connect-im
EOF

chmod +x "$OUTPUT_DIR/start.sh"

# 创建停止脚本 / Create stop script
cat > "$OUTPUT_DIR/stop.sh" << 'EOF'
#!/bin/bash
# v-connect-im 停止脚本 / v-connect-im Stop Script

echo "🛑 停止 v-connect-im / Stopping v-connect-im"

# 查找进程 / Find process
PID=$(pgrep -f "bin/v-connect-im" || true)

if [ -z "$PID" ]; then
    echo "ℹ️  v-connect-im 未运行 / v-connect-im is not running"
    exit 0
fi

echo "📍 找到进程 PID: $PID / Found process PID: $PID"
kill -TERM "$PID"

# 等待进程退出 / Wait for process to exit
for i in {1..10}; do
    if ! kill -0 "$PID" 2>/dev/null; then
        echo "✅ v-connect-im 已停止 / v-connect-im stopped"
        exit 0
    fi
    sleep 1
done

# 强制杀死 / Force kill
echo "⚠️  强制停止进程 / Force killing process"
kill -9 "$PID"
echo "✅ v-connect-im 已强制停止 / v-connect-im force stopped"
EOF

chmod +x "$OUTPUT_DIR/stop.sh"

# 创建 systemd 服务文件模板 / Create systemd service template
info "🔧 创建 systemd 服务文件模板 / Creating systemd service template"
cat > "$OUTPUT_DIR/v-connect-im.service" << EOF
[Unit]
Description=v-connect-im Instant Messaging Server
After=network.target

[Service]
Type=simple
User=YOUR_USER
Group=YOUR_GROUP
WorkingDirectory=$OUTPUT_DIR
ExecStart=$OUTPUT_DIR/bin/v-connect-im
ExecStop=$OUTPUT_DIR/stop.sh
Restart=on-failure
RestartSec=5s

# 环境变量 / Environment variables
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

# 安全设置 / Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$OUTPUT_DIR/logs $OUTPUT_DIR/plugins $OUTPUT_DIR/data

# 资源限制 / Resource limits
LimitNOFILE=65535
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

# 创建 README / Create README
info "📄 创建 README / Creating README"
cat > "$OUTPUT_DIR/README.md" << EOF
# v-connect-im v$VERSION

高性能即时通讯服务器 / High-performance Instant Messaging Server

## 目录结构 / Directory Structure

\`\`\`
v-connect-im/
├── bin/                    # 二进制文件 / Binary files
│   └── v-connect-im       # 主程序 / Main program
├── config/                 # 配置文件 / Configuration files
│   ├── default.toml       # 默认配置 / Default config
│   └── production.toml    # 生产环境配置 / Production config
├── logs/                   # 日志目录 / Log directory
├── plugins/                # 插件目录 / Plugin directory
│   └── sockets/           # Socket 文件目录 / Socket files
├── data/                   # 数据目录 / Data directory
├── start.sh               # 启动脚本 / Startup script
├── stop.sh                # 停止脚本 / Stop script
├── v-connect-im.service   # systemd 服务文件 / systemd service file
└── README.md              # 本文件 / This file
\`\`\`

## 快速开始 / Quick Start

### 1. 配置 / Configuration

复制并编辑生产环境配置:

\`\`\`bash
cp config/default.toml config/production.toml
vim config/production.toml
\`\`\`

### 2. 启动服务 / Start Service

#### 方式一:直接启动 / Method 1: Direct Start

\`\`\`bash
./start.sh
\`\`\`

#### 方式二:使用 systemd / Method 2: Using systemd

\`\`\`bash
# 1. 编辑服务文件,修改 YOUR_USER 和 YOUR_GROUP
# Edit service file, change YOUR_USER and YOUR_GROUP
sudo vim v-connect-im.service

# 2. 复制服务文件
# Copy service file
sudo cp v-connect-im.service /etc/systemd/system/

# 3. 重载 systemd
# Reload systemd
sudo systemctl daemon-reload

# 4. 启动服务
# Start service
sudo systemctl start v-connect-im

# 5. 设置开机自启
# Enable auto-start
sudo systemctl enable v-connect-im

# 6. 查看状态
# Check status
sudo systemctl status v-connect-im
\`\`\`

### 3. 停止服务 / Stop Service

\`\`\`bash
# 直接停止 / Direct stop
./stop.sh

# 或使用 systemd / Or using systemd
sudo systemctl stop v-connect-im
\`\`\`

## 日志 / Logs

日志输出到标准输出,可以通过以下方式查看:

\`\`\`bash
# 直接运行时 / When running directly
# 日志会输出到终端 / Logs output to terminal

# 使用 systemd 时 / When using systemd
sudo journalctl -u v-connect-im -f
\`\`\`

## 环境变量 / Environment Variables

- \`RUST_LOG\`: 日志级别 (trace, debug, info, warn, error) / Log level
- \`RUST_BACKTRACE\`: 启用堆栈跟踪 / Enable backtrace (0 或 1)

## 端口 / Ports

- HTTP API: 8080 (默认 / default)
- WebSocket: 8081 (默认 / default)

## 插件 / Plugins

插件文件放在 \`plugins/\` 目录下。开发模式插件在生产环境中不可用。

Plugins are placed in the \`plugins/\` directory. Dev mode plugins are not available in production.

## 监控 / Monitoring

健康检查端点 / Health check endpoint:

\`\`\`bash
curl http://localhost:8080/health
\`\`\`

## 故障排查 / Troubleshooting

1. **服务无法启动 / Service won't start**
   - 检查配置文件是否正确 / Check configuration file
   - 检查端口是否被占用 / Check if ports are in use
   - 查看日志输出 / Check log output

2. **插件无法加载 / Plugins won't load**
   - 确保插件文件存在 / Ensure plugin files exist
   - 检查 socket 目录权限 / Check socket directory permissions
   - 查看插件日志 / Check plugin logs

## 更多信息 / More Information

- 项目文档 / Project Documentation: /Users/mac/workspace/vgo-rust/docs
- 版本 / Version: $VERSION
- 构建时间 / Build Time: $(date '+%Y-%m-%d %H:%M:%S')
EOF

# 生成版本信息文件 / Generate version info
cat > "$OUTPUT_DIR/VERSION" << EOF
VERSION=$VERSION
BUILD_DATE=$(date '+%Y-%m-%d %H:%M:%S')
BUILD_HOST=$(hostname)
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
EOF

# 打包完成 / Build completed
success "✨ 打包完成 / Build completed!"
echo ""
info "📦 输出目录 / Output directory: $OUTPUT_DIR"
info "📊 版本 / Version: $VERSION"
info "💾 二进制大小 / Binary size: $(du -h "$OUTPUT_DIR/bin/v-connect-im" | cut -f1)"
echo ""
info "🚀 下一步 / Next steps:"
echo "  1. cd $OUTPUT_DIR"
echo "  2. 编辑配置文件 / Edit config: vim config/production.toml"
echo "  3. 启动服务 / Start service: ./start.sh"
echo ""
success "🎉 完成 / Done!"
