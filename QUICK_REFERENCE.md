# 快速参考 / Quick Reference

## 最常用命令 / Most Common Commands

```bash
# 查看所有任务 / List all tasks
task

# 构建所有插件 / Build all plugins
task build:plugins

# 构建生产版本 / Build release
task build:release

# 启动开发服务器 / Start dev server
task dev:im

# 清理插件 / Cleanup plugins
task cleanup:plugins

# 运行测试 / Run tests
task test

# 格式化代码 / Format code
task fmt
```

## 构建命令 / Build Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task build:plugins` | 构建所有插件 / Build all plugins |
| `task build:plugin PLUGIN=<name>` | 构建指定插件 / Build specific plugin |
| `task build:release` | 构建生产版本 / Build release version |
| `task build:all` | 构建所有（插件+主程序）/ Build all |

## 开发命令 / Development Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task dev:im` | 启动 IM 服务 / Start IM service |
| `task dev:admin` | 启动管理后台 / Start admin service |
| `task dev:auth` | 启动认证中心 / Start auth service |
| `task watch:im` | 监听文件变化 / Watch file changes |

## 测试命令 / Test Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task test` | 运行所有测试 / Run all tests |
| `task test:v` | 测试公共库 / Test common library |
| `task test:im` | 测试 IM 服务 / Test IM service |

## 代码质量 / Code Quality

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task fmt` | 格式化代码 / Format code |
| `task fmt:check` | 检查格式 / Check format |
| `task clippy` | 运行 Clippy / Run Clippy |
| `task lint` | 运行所有检查 / Run all linters |

## 清理命令 / Cleanup Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task cleanup:plugins` | 清理插件进程 / Cleanup plugin processes |
| `task cleanup:dist` | 清理构建产物 / Cleanup build artifacts |
| `task cleanup:target` | 清理 Cargo 缓存 / Cleanup Cargo cache |
| `task cleanup:all` | 清理所有 / Cleanup all |

## 文档命令 / Documentation Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task docs:build` | 构建 Rust 文档 / Build Rust docs |
| `task docs:open` | 打开 Rust 文档 / Open Rust docs |
| `task docs:serve` | 启动文档服务器 / Start docs server |

## 工具命令 / Utility Commands

| 命令 / Command | 说明 / Description |
|---------------|-------------------|
| `task info` | 显示项目信息 / Show project info |
| `task version` | 显示版本信息 / Show version info |
| `task install:tools` | 安装开发工具 / Install dev tools |

## 常用工作流 / Common Workflows

### 开始开发 / Start Development

```bash
task install:tools  # 安装工具 / Install tools
task dev:im         # 启动服务 / Start service
```

### 提交代码前 / Before Commit

```bash
task fmt            # 格式化 / Format
task lint           # 检查 / Lint
task test           # 测试 / Test
```

### 构建发布 / Build Release

```bash
task cleanup:all    # 清理 / Cleanup
task lint           # 检查 / Lint
task test           # 测试 / Test
task build:all      # 构建 / Build
```

### 调试插件问题 / Debug Plugin Issues

```bash
task check:plugins     # 检查状态 / Check status
task cleanup:plugins   # 清理插件 / Cleanup plugins
task build:plugins     # 重新构建 / Rebuild
```

## 脚本对照表 / Script Mapping

| 原脚本 / Original | Task 命令 / Task Command |
|------------------|-------------------------|
| `./scripts/build-plugins.sh` | `task build:plugins` |
| `./scripts/build-release.sh` | `task build:release` |
| `./scripts/check-plugins.sh` | `task check:plugins` |
| `./scripts/cleanup-plugins.sh` | `task cleanup:plugins` |
| `./scripts/validate-workflows.sh` | `task check:workflows` |

## 高级用法 / Advanced Usage

```bash
# 并行执行 / Parallel execution
task --parallel test fmt

# 查看执行的命令 / Show commands
task --dry build:plugins

# 详细输出 / Verbose output
task --verbose build:plugins

# 静默模式 / Silent mode
task --silent build:plugins

# 查看任务详情 / Show task details
task --summary build:plugins
```

## 环境变量 / Environment Variables

```bash
# 传递变量 / Pass variables
task build:plugin PLUGIN=my-plugin
task build:release:custom OUTPUT=/path/to/output

# 设置 Rust 日志级别 / Set Rust log level
RUST_LOG=debug task dev:im

# 启用详细输出 / Enable verbose
VERBOSE=1 task build:plugins
```

## 提示 / Tips

1. **自动补全** / **Auto-completion**: 运行 `./.taskfile/install-completion.sh` 安装 shell 补全
2. **帮助信息** / **Help**: 使用 `task --summary <task-name>` 查看任务详情
3. **列出所有任务** / **List all**: 使用 `task --list-all` 查看包括内部任务的所有任务
4. **任务依赖** / **Dependencies**: 某些任务会自动执行依赖的任务
5. **增量构建** / **Incremental**: Task 会根据文件变化智能决定是否需要重新构建

## 故障排查 / Troubleshooting

### Task 未找到 / Task not found

```bash
# 检查安装 / Check installation
which task
task --version

# macOS 安装 / Install on macOS
brew install go-task/tap/go-task
```

### 权限错误 / Permission error

```bash
# 添加执行权限 / Add execute permission
chmod +x scripts/*.sh
```

### 构建失败 / Build failed

```bash
# 清理后重试 / Cleanup and retry
task cleanup:all
task build:all
```

### 插件问题 / Plugin issues

```bash
# 检查插件状态 / Check plugin status
task check:plugins

# 清理插件 / Cleanup plugins
task cleanup:plugins
```

## 更多信息 / More Information

- 详细文档: [TASKFILE_USAGE.md](./TASKFILE_USAGE.md)
- 项目文档: [docs/](./docs/)
- Task 官网: https://taskfile.dev
