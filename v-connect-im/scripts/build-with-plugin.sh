#!/bin/bash
# 构建 v-connect-im 并打包插件 / Build v-connect-im and package plugin
# 此脚本会先构建插件，然后构建服务器，最后配置插件自动加载
# This script builds the plugin first, then the server, and configures plugin auto-loading

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_DIR="$PROJECT_ROOT/../v-connect-im-plugin-example"
PLUGIN_NAME="example"

echo "🔨 Building plugin..."
cd "$PLUGIN_DIR"
cargo build --release

echo "📦 Packaging plugin..."
./scripts/package.sh

# 查找生成的 .wkp 文件 / Find generated .wkp file
WKP_FILE=$(find "$PLUGIN_DIR" -name "wk.plugin.${PLUGIN_NAME}-*.wkp" | head -1)
if [ -z "$WKP_FILE" ]; then
    echo "❌ Plugin package not found"
    exit 1
fi

echo "✅ Plugin packaged: $WKP_FILE"

# 复制插件到 v-connect-im 的插件目录 / Copy plugin to v-connect-im plugin directory
PLUGIN_DEST_DIR="$PROJECT_ROOT/plugins"
mkdir -p "$PLUGIN_DEST_DIR"
cp "$WKP_FILE" "$PLUGIN_DEST_DIR/"

echo "📋 Plugin copied to: $PLUGIN_DEST_DIR"

# 构建 v-connect-im / Build v-connect-im
echo "🔨 Building v-connect-im..."
cd "$PROJECT_ROOT"
cargo build --release

echo "✅ Build complete!"
echo ""
echo "To run v-connect-im with the plugin:"
echo "  1. Update config/default.toml:"
echo "     [plugins]"
echo "     install = [\"file://$(realpath $PLUGIN_DEST_DIR)/$(basename $WKP_FILE)\"]"
echo "  2. Run: ./target/release/v-connect-im"

