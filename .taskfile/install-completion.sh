#!/bin/bash
# Task Shell 自动补全安装脚本 / Task Shell Completion Installation Script

set -e

# 颜色定义 / Color definitions
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
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

info "安装 Task Shell 自动补全 / Installing Task Shell Completion"

# 检测 shell 类型 / Detect shell type
SHELL_TYPE=$(basename "$SHELL")

case "$SHELL_TYPE" in
    bash)
        info "检测到 Bash / Detected Bash"
        COMPLETION_DIR="$HOME/.local/share/bash-completion/completions"
        mkdir -p "$COMPLETION_DIR"
        
        # 生成补全脚本 / Generate completion script
        task --completion bash > "$COMPLETION_DIR/task"
        
        # 添加到 .bashrc / Add to .bashrc
        if ! grep -q "bash-completion" "$HOME/.bashrc" 2>/dev/null; then
            echo "" >> "$HOME/.bashrc"
            echo "# Task completion" >> "$HOME/.bashrc"
            echo "[[ -r $COMPLETION_DIR/task ]] && . $COMPLETION_DIR/task" >> "$HOME/.bashrc"
        fi
        
        success "Bash 补全已安装 / Bash completion installed"
        info "请运行以下命令使其生效 / Run the following to activate:"
        echo "  source ~/.bashrc"
        ;;
        
    zsh)
        info "检测到 Zsh / Detected Zsh"
        COMPLETION_DIR="$HOME/.zsh/completion"
        mkdir -p "$COMPLETION_DIR"
        
        # 生成补全脚本 / Generate completion script
        task --completion zsh > "$COMPLETION_DIR/_task"
        
        # 添加到 .zshrc / Add to .zshrc
        if ! grep -q "fpath.*completion" "$HOME/.zshrc" 2>/dev/null; then
            echo "" >> "$HOME/.zshrc"
            echo "# Task completion" >> "$HOME/.zshrc"
            echo "fpath=($COMPLETION_DIR \$fpath)" >> "$HOME/.zshrc"
            echo "autoload -Uz compinit && compinit" >> "$HOME/.zshrc"
        fi
        
        success "Zsh 补全已安装 / Zsh completion installed"
        info "请运行以下命令使其生效 / Run the following to activate:"
        echo "  source ~/.zshrc"
        ;;
        
    fish)
        info "检测到 Fish / Detected Fish"
        COMPLETION_DIR="$HOME/.config/fish/completions"
        mkdir -p "$COMPLETION_DIR"
        
        # 生成补全脚本 / Generate completion script
        task --completion fish > "$COMPLETION_DIR/task.fish"
        
        success "Fish 补全已安装 / Fish completion installed"
        info "Fish 会自动加载补全 / Fish will auto-load completions"
        ;;
        
    *)
        warn "不支持的 shell: $SHELL_TYPE / Unsupported shell: $SHELL_TYPE"
        info "支持的 shell / Supported shells: bash, zsh, fish"
        exit 1
        ;;
esac

echo ""
success "✨ 安装完成 / Installation completed!"
echo ""
info "现在你可以使用 Tab 键自动补全 Task 命令了"
info "Now you can use Tab key to auto-complete Task commands"
echo ""
echo "示例 / Examples:"
echo "  task bu<Tab>      # 补全为 build:"
echo "  task build:p<Tab> # 补全为 build:plugins"
