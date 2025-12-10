# Taskfile 使用指南 / Taskfile Usage Guide

本项目使用 [Task](https://taskfile.dev) 来管理常用的开发任务和脚本。

This project uses [Task](https://taskfile.dev) to manage common development tasks and scripts.

## 安装 Task / Install Task

### macOS

```bash
brew install go-task/tap/go-task
```

### Linux

```bash
sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b ~/.local/bin
```

### Windows

```powershell
choco install go-task
```

更多安装方式请参考: https://taskfile.dev/installation/

More installation methods: https://taskfile.dev/installation/

## 快速开始 / Quick Start

### 查看所有可用任务 / List all available tasks

```bash
task --list
# 或 / or
task
```

### 查看任务详细信息 / View task details

```bash
task --summary <task-name>
# 例如 / Example:
task --summary build:plugins
```

## 常用任务 / Common Tasks

### 🔨 构建任务 / Build Tasks

```bash
# 构建所有插件 / Build all plugins
task build:plugins

# 构建指定插件 / Build specific plugin
task build:plugin PLUGIN=v-connect-im-plugin-storage-sled

# 构建生产版本 / Build release version
task build:release

# 构建到自定义目录 / Build to custom directory
task build:release:custom OUTPUT=~/deploy/v-connect-im

# 构建所有（插件 + 主程序）/ Build all (plugins + main)
task build:all
```

### 🔍 检查任务 / Check Tasks

```bash
# 检查插件状态 / Check plugin status
task check:plugins

# 验证 GitHub Actions 工作流 / Validate GitHub Actions workflows
task check:workflows
```

### 🧹 清理任务 / Cleanup Tasks

```bash
# 清理插件进程和 socket 文件 / Cleanup plugin processes and socket files
task cleanup:plugins

# 清理构建产物 / Cleanup build artifacts
task cleanup:dist

# 清理 Cargo 缓存 / Cleanup Cargo cache
task cleanup:target

# 清理所有 / Cleanup all
task cleanup:all
```

### 💻 开发任务 / Development Tasks

```bash
# 启动 v-connect-im 开发服务器 / Start v-connect-im dev server
task dev:im

# 启动 v-admin 开发服务器 / Start v-admin dev server
task dev:admin

# 启动 v-auth-center 开发服务器 / Start v-auth-center dev server
task dev:auth

# 监听文件变化并自动重启 / Watch and auto-restart
task watch:im
```

### 🧪 测试任务 / Test Tasks

```bash
# 运行所有测试 / Run all tests
task test

# 测试公共库 v / Test common library v
task test:v

# 测试 v-connect-im / Test v-connect-im
task test:im
```

### 📝 代码质量任务 / Code Quality Tasks

```bash
# 格式化代码 / Format code
task fmt

# 检查代码格式 / Check code format
task fmt:check

# 运行 Clippy / Run Clippy
task clippy

# 运行所有代码检查 / Run all linters
task lint
```

### 📚 文档任务 / Documentation Tasks

```bash
# 构建 Rust 文档 / Build Rust documentation
task docs:build

# 打开 Rust 文档 / Open Rust documentation
task docs:open

# 启动 Mintlify 文档服务器 / Start Mintlify docs server
task docs:serve
```

### 🛠️ 工具任务 / Utility Tasks

```bash
# 安装开发工具 / Install development tools
task install:tools

# 显示项目信息 / Show project information
task info

# 显示版本信息 / Show version information
task version
```

## 任务组合示例 / Task Combination Examples

### 完整的构建和部署流程 / Complete build and deploy workflow

```bash
# 1. 清理旧的构建产物 / Cleanup old artifacts
task cleanup:all

# 2. 运行代码检查 / Run linters
task lint

# 3. 运行测试 / Run tests
task test

# 4. 构建所有组件 / Build all components
task build:all

# 5. 检查插件状态 / Check plugin status
task check:plugins
```

### 开发工作流 / Development workflow

```bash
# 1. 安装开发工具 / Install dev tools
task install:tools

# 2. 格式化代码 / Format code
task fmt

# 3. 启动开发服务器 / Start dev server
task dev:im
```

### 发布前检查 / Pre-release checklist

```bash
# 1. 代码格式检查 / Check code format
task fmt:check

# 2. 运行 Clippy / Run Clippy
task clippy

# 3. 运行所有测试 / Run all tests
task test

# 4. 构建生产版本 / Build release version
task build:release
```

## 自定义任务 / Custom Tasks

你可以在 `Taskfile.yml` 中添加自己的任务。任务格式如下：

You can add your own tasks in `Taskfile.yml`. Task format:

```yaml
tasks:
  my-task:
    desc: 任务简短描述 / Short description
    summary: |
      任务详细说明
      Detailed description
    cmds:
      - echo "执行命令 / Execute command"
```

## 高级用法 / Advanced Usage

### 传递变量 / Pass variables

```bash
task build:plugin PLUGIN=my-plugin
task build:release:custom OUTPUT=/path/to/output
```

### 并行执行任务 / Run tasks in parallel

```bash
task --parallel task1 task2 task3
```

### 查看任务执行的命令 / Show commands without executing

```bash
task --dry build:plugins
```

### 静默模式 / Silent mode

```bash
task --silent build:plugins
```

## 与原有脚本的对应关系 / Mapping to Original Scripts

| 原脚本 / Original Script | Task 命令 / Task Command |
|-------------------------|-------------------------|
| `./scripts/build-plugins.sh` | `task build:plugins` |
| `./scripts/build-plugins.sh my-plugin` | `task build:plugin PLUGIN=my-plugin` |
| `./scripts/build-release.sh` | `task build:release` |
| `./scripts/build-release.sh ~/output` | `task build:release:custom OUTPUT=~/output` |
| `./scripts/check-plugins.sh` | `task check:plugins` |
| `./scripts/cleanup-plugins.sh` | `task cleanup:plugins` |
| `./scripts/validate-workflows.sh` | `task check:workflows` |

## 优势 / Advantages

使用 Taskfile 的优势：

Advantages of using Taskfile:

1. **统一接口** / **Unified Interface**: 所有任务通过 `task` 命令访问
2. **自动补全** / **Auto-completion**: 支持 shell 自动补全
3. **依赖管理** / **Dependency Management**: 任务可以依赖其他任务
4. **增量构建** / **Incremental Builds**: 基于文件变化的智能构建
5. **跨平台** / **Cross-platform**: 在 Linux、macOS、Windows 上都能运行
6. **文档化** / **Documentation**: 任务自带描述和帮助信息
7. **变量支持** / **Variable Support**: 支持环境变量和任务变量
8. **并行执行** / **Parallel Execution**: 支持并行执行多个任务

## 故障排查 / Troubleshooting

### Task 命令未找到 / Task command not found

确保已正确安装 Task 并添加到 PATH。

Make sure Task is properly installed and added to PATH.

```bash
# 检查安装 / Check installation
which task

# 查看版本 / Check version
task --version
```

### 任务执行失败 / Task execution failed

使用 `--verbose` 查看详细输出：

Use `--verbose` to see detailed output:

```bash
task --verbose build:plugins
```

### 权限问题 / Permission issues

确保脚本有执行权限：

Make sure scripts have execute permission:

```bash
chmod +x scripts/*.sh
```

## 更多资源 / More Resources

- [Task 官方文档 / Official Documentation](https://taskfile.dev)
- [Task GitHub 仓库 / GitHub Repository](https://github.com/go-task/task)
- [项目脚本目录 / Project Scripts Directory](./scripts/)

## 贡献 / Contributing

如果你添加了新的脚本或任务，请：

If you add new scripts or tasks, please:

1. 在 `Taskfile.yml` 中添加对应的任务
2. 更新本文档
3. 添加适当的描述和示例

1. Add corresponding task in `Taskfile.yml`
2. Update this documentation
3. Add proper description and examples
