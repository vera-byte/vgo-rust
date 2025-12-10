#!/bin/bash
# 插件打包脚本 / Plugin Build Script
# 
# 用法 / Usage:
#   ./scripts/build-plugins.sh [plugin_name] [output_dir]
#   
# 参数 / Parameters:
#   plugin_name  - 插件名称，留空则打包所有插件 / Plugin name, leave empty to build all plugins
#   output_dir   - 输出目录，默认为 ./dist/plugins / Output directory, default: ./dist/plugins
#
# 示例 / Examples:
#   ./scripts/build-plugins.sh                                    # 打包所有插件为 .vp 文件 / Build all plugins as .vp files
#   ./scripts/build-plugins.sh v-connect-im-plugin-storage-sled  # 只打包指定插件 / Build only specified plugin
#   ./scripts/build-plugins.sh "" ~/deploy                        # 打包所有插件到指定目录 / Build all to specific directory
#
# 输出格式 / Output Format:
#   插件将被打包为 .vp 文件（tar.gz 格式）/ Plugins will be packaged as .vp files (tar.gz format)
#   例如 / Example: storage-sled-0.1.0.vp

set -e  # 遇到错误立即退出 / Exit on error

# 颜色定义 / Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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

step() {
    echo -e "${CYAN}▶️  $1${NC}"
}

debug() {
    if [ "${VERBOSE:-0}" = "1" ]; then
        echo -e "${CYAN}🔍 $1${NC}"
    fi
}

# 获取脚本所在目录 / Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 解析参数 / Parse arguments
PLUGIN_NAME="${1:-}"
OUTPUT_DIR="${2:-$PROJECT_ROOT/dist/plugins}"

# 插件源码目录 / Plugin source directory
PLUGINS_DIR="$PROJECT_ROOT/v-plugins-hub"

# 显示配置信息 / Show configuration
info "插件打包脚本 / Plugin Build Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "项目根目录 / Project root: $PROJECT_ROOT"
echo "插件源码目录 / Plugins source: $PLUGINS_DIR"
echo "输出目录 / Output directory: $OUTPUT_DIR"
if [ -n "$PLUGIN_NAME" ]; then
    echo "目标插件 / Target plugin: $PLUGIN_NAME"
else
    echo "目标插件 / Target plugin: 全部 / All"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查插件目录是否存在 / Check if plugins directory exists
if [ ! -d "$PLUGINS_DIR" ]; then
    error "插件目录不存在 / Plugins directory not found: $PLUGINS_DIR"
    exit 1
fi

# 创建输出目录 / Create output directory
mkdir -p "$OUTPUT_DIR"

# 获取要打包的插件列表 / Get list of plugins to build
if [ -n "$PLUGIN_NAME" ]; then
    # 打包指定插件 / Build specific plugin
    if [ ! -d "$PLUGINS_DIR/$PLUGIN_NAME" ]; then
        error "插件不存在 / Plugin not found: $PLUGIN_NAME"
        exit 1
    fi
    PLUGINS=("$PLUGIN_NAME")
else
    # 打包所有插件 / Build all plugins
    PLUGINS=()
    for dir in "$PLUGINS_DIR"/*; do
        if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ]; then
            plugin_name=$(basename "$dir")
            PLUGINS+=("$plugin_name")
        fi
    done
fi

# 显示插件列表 / Show plugin list
info "发现 ${#PLUGINS[@]} 个插件 / Found ${#PLUGINS[@]} plugin(s):"
for plugin in "${PLUGINS[@]}"; do
    echo "  - $plugin"
done
echo ""

# 编译计数器 / Build counters
SUCCESS_COUNT=0
FAILED_COUNT=0
FAILED_PLUGINS=()

# 开始编译 / Start building
step "开始编译插件 / Starting plugin compilation..."
echo ""

for plugin in "${PLUGINS[@]}"; do
    step "正在编译插件 / Building plugin: $plugin"
    
    PLUGIN_DIR="$PLUGINS_DIR/$plugin"
    TEMP_DIR="$OUTPUT_DIR/.tmp/$plugin"
    
    # 创建临时目录 / Create temporary directory
    rm -rf "$TEMP_DIR"
    mkdir -p "$TEMP_DIR"
    
    # 编译插件 / Compile plugin
    if cargo build --release --manifest-path "$PLUGIN_DIR/Cargo.toml" 2>&1 | grep -E "(Compiling|Finished|error)"; then
        # 查找编译产物 / Find build artifacts
        # 检查是否使用工作区 / Check if using workspace
        if [ -f "$PLUGINS_DIR/Cargo.toml" ] && grep -q "\[workspace\]" "$PLUGINS_DIR/Cargo.toml"; then
            TARGET_DIR="$PLUGINS_DIR/target/release"
            info "检测到工作区配置，使用工作区 target 目录 / Workspace detected, using workspace target directory"
        else
            TARGET_DIR="$PLUGIN_DIR/target/release"
        fi
        
        # 从 Cargo.toml 获取实际的二进制名称 / Get actual binary name from Cargo.toml
        # 使用 jq 正确解析当前插件的 target name
        BINARY_NAME=$(cargo metadata --manifest-path "$PLUGIN_DIR/Cargo.toml" --format-version 1 --no-deps 2>/dev/null | \
                      jq -r '.packages[] | select(.manifest_path | contains("'"$plugin"'")) | .targets[] | select(.kind[] == "bin") | .name' | head -1)
        
        if [ -z "$BINARY_NAME" ]; then
            # 如果 jq 不可用或解析失败，使用插件目录名作为后备
            BINARY_NAME="$plugin"
        fi
        
        info "二进制名称 / Binary name: $BINARY_NAME"
        
        # 获取版本信息 / Get version info
        VERSION=$(cargo metadata --manifest-path "$PLUGIN_DIR/Cargo.toml" --format-version 1 --no-deps 2>/dev/null | \
                  jq -r '.packages[] | select(.manifest_path | contains("'"$plugin"'")) | .version' | head -1)
        if [ -z "$VERSION" ]; then
            VERSION="0.0.0"
        fi
        info "插件版本 / Plugin version: $VERSION"
        
        # 复制二进制文件 / Copy binary
        if [ -f "$TARGET_DIR/$BINARY_NAME" ]; then
            cp "$TARGET_DIR/$BINARY_NAME" "$TEMP_DIR/"
            chmod +x "$TEMP_DIR/$BINARY_NAME"
            success "已复制二进制文件 / Binary copied: $BINARY_NAME"
        else
            error "未找到二进制文件 / Binary not found: $BINARY_NAME"
            error "请检查编译是否成功 / Please check if compilation succeeded"
            FAILED_PLUGINS+=("$plugin")
            ((FAILED_COUNT++))
            continue
        fi
        
        # 复制插件配置文件（必需）/ Copy plugin config (required)
        if [ -f "$PLUGIN_DIR/plugin.json" ]; then
            cp "$PLUGIN_DIR/plugin.json" "$TEMP_DIR/"
            info "已复制配置文件 / Config copied: plugin.json"
        else
            error "未找到 plugin.json 配置文件 / plugin.json not found"
            error "请在插件目录创建 plugin.json 文件 / Please create plugin.json in plugin directory"
            FAILED_PLUGINS+=("$plugin")
            ((FAILED_COUNT++))
            continue
        fi
        
        # 复制 README（可选）/ Copy README (optional)
        if [ -f "$PLUGIN_DIR/README.md" ]; then
            cp "$PLUGIN_DIR/README.md" "$TEMP_DIR/"
        fi
        
        # 创建版本信息文件 / Create version info file
        echo "$VERSION" > "$TEMP_DIR/VERSION"
        
        # 检测操作系统和架构 / Detect OS and architecture
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        ARCH=$(uname -m)
        
        # 标准化架构名称 / Normalize architecture name
        case "$ARCH" in
            x86_64)
                ARCH="amd64"
                ;;
            aarch64|arm64)
                ARCH="arm64"
                ;;
            armv7l)
                ARCH="armv7"
                ;;
        esac
        
        # 打包成 .vp 文件 / Package as .vp file
        VP_FILE="$(cd "$OUTPUT_DIR" && pwd)/$plugin-$VERSION-$OS-$ARCH.vp"
        step "打包插件 / Packaging plugin: $plugin-$VERSION-$OS-$ARCH.vp"
        
        cd "$TEMP_DIR"
        tar -czf "$VP_FILE" *
        cd - > /dev/null
        
        # 计算文件大小和校验和 / Calculate file size and checksum
        VP_SIZE=$(du -h "$VP_FILE" | cut -f1)
        VP_SHA256=$(shasum -a 256 "$VP_FILE" | cut -d' ' -f1)
        
        success "插件打包成功 / Plugin packaged successfully: $plugin-$VERSION-$OS-$ARCH.vp ($VP_SIZE)"
        info "SHA256: $VP_SHA256"
        
        # 创建校验和文件 / Create checksum file
        echo "$VP_SHA256  $plugin-$VERSION-$OS-$ARCH.vp" > "$VP_FILE.sha256"
        
        ((SUCCESS_COUNT++))
    else
        error "插件编译失败 / Plugin build failed: $plugin"
        FAILED_PLUGINS+=("$plugin")
        ((FAILED_COUNT++))
    fi
    
    echo ""
done

# 显示编译结果摘要 / Show build summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
info "编译完成 / Build completed"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "成功 / Success: $SUCCESS_COUNT"
echo "失败 / Failed: $FAILED_COUNT"

if [ $FAILED_COUNT -gt 0 ]; then
    echo ""
    warn "失败的插件 / Failed plugins:"
    for plugin in "${FAILED_PLUGINS[@]}"; do
        echo "  - $plugin"
    done
fi

echo ""
echo "输出目录 / Output directory: $OUTPUT_DIR"
echo ""

# 清理临时目录 / Cleanup temporary directory
rm -rf "$OUTPUT_DIR/.tmp"

# 显示打包文件列表 / Show packaged files
if [ $SUCCESS_COUNT -gt 0 ]; then
    echo ""
    info "打包的插件文件 / Packaged plugin files:"
    for vp_file in "$OUTPUT_DIR"/*.vp; do
        if [ -f "$vp_file" ]; then
            filename=$(basename "$vp_file")
            size=$(du -h "$vp_file" | cut -f1)
            echo "  📦 $filename ($size)"
            if [ -f "$vp_file.sha256" ]; then
                sha256=$(cat "$vp_file.sha256" | cut -d' ' -f1)
                echo "     SHA256: $sha256"
            fi
        fi
    done
    echo ""
fi

# 退出码 / Exit code
if [ $FAILED_COUNT -gt 0 ]; then
    exit 1
else
    success "所有插件编译成功！/ All plugins built successfully!"
    exit 0
fi
