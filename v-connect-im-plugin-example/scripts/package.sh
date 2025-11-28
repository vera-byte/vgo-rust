#!/bin/bash
# 插件打包脚本 / Plugin packaging script
# 将插件打包成 .wkp 文件（tar.gz 格式）
# Package plugin into .wkp file (tar.gz format)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_NAME="example"
OS="${OS:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
ARCH="${ARCH:-$(uname -m)}"

# 处理架构名称 / Handle architecture names
case "$ARCH" in
    x86_64)
        ARCH="amd64"
        ;;
    arm64|aarch64)
        ARCH="arm64"
        ;;
esac

# 处理操作系统名称 / Handle OS names
case "$OS" in
    darwin)
        OS="darwin"
        ;;
    linux)
        OS="linux"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "📦 Packaging plugin: $PLUGIN_NAME"
echo "   OS: $OS"
echo "   Arch: $ARCH"

# 构建插件 / Build plugin
echo "🔨 Building plugin..."
cd "$PROJECT_DIR"
cargo build --release

# 创建临时目录 / Create temporary directory
TEMP_DIR=$(mktemp -d)
PLUGIN_DIR="$TEMP_DIR/$PLUGIN_NAME"
mkdir -p "$PLUGIN_DIR"

# 复制文件 / Copy files
echo "📋 Copying files..."

# 复制二进制文件 / Copy binary
BINARY_NAME="$PLUGIN_NAME"
if [ "$OS" = "windows" ]; then
    BINARY_NAME="${PLUGIN_NAME}.exe"
fi
cp "$PROJECT_DIR/target/release/$PLUGIN_NAME" "$PLUGIN_DIR/$BINARY_NAME"
chmod +x "$PLUGIN_DIR/$BINARY_NAME"

# 复制配置文件 / Copy config file
if [ -f "$PROJECT_DIR/plugin.json" ]; then
    cp "$PROJECT_DIR/plugin.json" "$PLUGIN_DIR/"
elif [ -f "$PROJECT_DIR/plugin.yaml" ]; then
    cp "$PROJECT_DIR/plugin.yaml" "$PLUGIN_DIR/"
elif [ -f "$PROJECT_DIR/plugin.yml" ]; then
    cp "$PROJECT_DIR/plugin.yml" "$PLUGIN_DIR/"
fi

# 创建 tar.gz 文件 / Create tar.gz file
OUTPUT_FILE="wk.plugin.${PLUGIN_NAME}-${OS}-${ARCH}.vp"
echo "📦 Creating package: $OUTPUT_FILE"
cd "$TEMP_DIR"
tar -czf "$PROJECT_DIR/$OUTPUT_FILE" "$PLUGIN_NAME"

# 清理临时目录 / Cleanup
rm -rf "$TEMP_DIR"

echo "✅ Package created: $OUTPUT_FILE"
echo "   Location: $PROJECT_DIR/$OUTPUT_FILE"
