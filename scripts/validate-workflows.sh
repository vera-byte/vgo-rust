#!/bin/bash
# GitHub Actions 工作流验证脚本 / GitHub Actions Workflow Validation Script
#
# 用法 / Usage:
#   ./scripts/validate-workflows.sh
#
# 此脚本检查 GitHub Actions 工作流配置的有效性
# This script checks the validity of GitHub Actions workflow configurations

set -e

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
WORKFLOWS_DIR="$PROJECT_ROOT/.github/workflows"

info "GitHub Actions 工作流验证 / GitHub Actions Workflow Validation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查工作流目录是否存在 / Check if workflows directory exists
if [ ! -d "$WORKFLOWS_DIR" ]; then
    error "工作流目录不存在 / Workflows directory not found: $WORKFLOWS_DIR"
    exit 1
fi

# 检查是否安装了 actionlint / Check if actionlint is installed
if ! command -v actionlint &> /dev/null; then
    warn "actionlint 未安装，将跳过 YAML 语法检查 / actionlint not installed, skipping YAML syntax check"
    warn "安装方法 / Installation: brew install actionlint (macOS) or go install github.com/rhysd/actionlint/cmd/actionlint@latest"
    ACTIONLINT_AVAILABLE=false
else
    ACTIONLINT_AVAILABLE=true
    success "actionlint 已安装 / actionlint is installed"
fi

echo ""

# 检查工作流文件 / Check workflow files
WORKFLOW_FILES=("$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml)
TOTAL_FILES=0
VALID_FILES=0
INVALID_FILES=0

info "检查工作流文件 / Checking workflow files..."
echo ""

for workflow in "${WORKFLOW_FILES[@]}"; do
    if [ ! -f "$workflow" ]; then
        continue
    fi
    
    TOTAL_FILES=$((TOTAL_FILES + 1))
    filename=$(basename "$workflow")
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    info "检查文件 / Checking file: $filename"
    
    # 基本 YAML 语法检查 / Basic YAML syntax check
    if command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$workflow'))" 2>/dev/null; then
            success "YAML 语法正确 / YAML syntax is valid"
        else
            error "YAML 语法错误 / YAML syntax error"
            INVALID_FILES=$((INVALID_FILES + 1))
            continue
        fi
    fi
    
    # 使用 actionlint 进行详细检查 / Detailed check with actionlint
    if [ "$ACTIONLINT_AVAILABLE" = true ]; then
        if actionlint "$workflow"; then
            success "actionlint 检查通过 / actionlint check passed"
            VALID_FILES=$((VALID_FILES + 1))
        else
            error "actionlint 检查失败 / actionlint check failed"
            INVALID_FILES=$((INVALID_FILES + 1))
        fi
    else
        # 如果没有 actionlint，只做基本检查 / Basic checks if actionlint not available
        VALID_FILES=$((VALID_FILES + 1))
    fi
    
    # 检查必需的字段 / Check required fields
    if ! grep -q "^name:" "$workflow"; then
        warn "缺少 'name' 字段 / Missing 'name' field"
    fi
    
    if ! grep -q "^on:" "$workflow"; then
        error "缺少 'on' 字段 / Missing 'on' field"
        INVALID_FILES=$((INVALID_FILES + 1))
        VALID_FILES=$((VALID_FILES - 1))
    fi
    
    if ! grep -q "^jobs:" "$workflow"; then
        error "缺少 'jobs' 字段 / Missing 'jobs' field"
        INVALID_FILES=$((INVALID_FILES + 1))
        VALID_FILES=$((VALID_FILES - 1))
    fi
    
    echo ""
done

# 显示摘要 / Show summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
info "验证完成 / Validation completed"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "总文件数 / Total files: $TOTAL_FILES"
echo "有效文件 / Valid files: $VALID_FILES"
echo "无效文件 / Invalid files: $INVALID_FILES"
echo ""

# 检查项目结构 / Check project structure
info "检查项目结构 / Checking project structure..."
echo ""

# 检查必需的目录 / Check required directories
REQUIRED_DIRS=(
    "v"
    "v-connect-im"
    "v-plugins-hub"
    "v-plugins-hub/v-connect-im-plugin-storage-sled"
    "v-plugins-hub/v-connect-im-plugin-gateway"
)

for dir in "${REQUIRED_DIRS[@]}"; do
    if [ -d "$PROJECT_ROOT/$dir" ]; then
        success "目录存在 / Directory exists: $dir"
    else
        error "目录不存在 / Directory not found: $dir"
    fi
done

echo ""

# 检查必需的文件 / Check required files
REQUIRED_FILES=(
    "Cargo.toml"
    "v/Cargo.toml"
    "v-connect-im/Cargo.toml"
    "v-plugins-hub/Cargo.toml"
    "v-plugins-hub/v-connect-im-plugin-storage-sled/Cargo.toml"
    "v-plugins-hub/v-connect-im-plugin-storage-sled/plugin.json"
    "v-plugins-hub/v-connect-im-plugin-gateway/Cargo.toml"
    "v-plugins-hub/v-connect-im-plugin-gateway/plugin.json"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$PROJECT_ROOT/$file" ]; then
        success "文件存在 / File exists: $file"
    else
        error "文件不存在 / File not found: $file"
    fi
done

echo ""

# 检查 Docker 配置 / Check Docker configuration
info "检查 Docker 配置 / Checking Docker configuration..."
echo ""

if [ -f "$PROJECT_ROOT/v-connect-im/Dockerfile" ]; then
    success "Dockerfile 存在 / Dockerfile exists"
    
    # 检查 Dockerfile 语法 / Check Dockerfile syntax
    if command -v docker &> /dev/null; then
        if docker build --dry-run -f "$PROJECT_ROOT/v-connect-im/Dockerfile" "$PROJECT_ROOT" &> /dev/null; then
            success "Dockerfile 语法正确 / Dockerfile syntax is valid"
        else
            warn "Dockerfile 可能存在问题 / Dockerfile may have issues"
        fi
    else
        warn "Docker 未安装，跳过 Dockerfile 验证 / Docker not installed, skipping Dockerfile validation"
    fi
else
    warn "Dockerfile 不存在 / Dockerfile not found"
fi

if [ -f "$PROJECT_ROOT/.dockerignore" ]; then
    success ".dockerignore 存在 / .dockerignore exists"
else
    warn ".dockerignore 不存在 / .dockerignore not found"
fi

echo ""

# 检查脚本权限 / Check script permissions
info "检查脚本权限 / Checking script permissions..."
echo ""

SCRIPTS=(
    "scripts/build-release.sh"
    "scripts/build-plugins.sh"
    "scripts/check-plugins.sh"
    "scripts/cleanup-plugins.sh"
)

for script in "${SCRIPTS[@]}"; do
    if [ -f "$PROJECT_ROOT/$script" ]; then
        if [ -x "$PROJECT_ROOT/$script" ]; then
            success "脚本可执行 / Script is executable: $script"
        else
            warn "脚本不可执行 / Script is not executable: $script"
            info "修复方法 / Fix: chmod +x $PROJECT_ROOT/$script"
        fi
    fi
done

echo ""

# 最终结果 / Final result
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $INVALID_FILES -eq 0 ]; then
    success "所有检查通过！/ All checks passed!"
    echo ""
    info "下一步 / Next steps:"
    echo "  1. 提交工作流文件 / Commit workflow files: git add .github/"
    echo "  2. 推送到 GitHub / Push to GitHub: git push"
    echo "  3. 查看 Actions 页面 / Check Actions page: https://github.com/{owner}/{repo}/actions"
    exit 0
else
    error "发现 $INVALID_FILES 个问题 / Found $INVALID_FILES issue(s)"
    echo ""
    info "请修复上述问题后重新运行此脚本 / Please fix the issues above and run this script again"
    exit 1
fi
