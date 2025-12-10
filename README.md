# VGO-Rust

高性能即时通讯系统 / High-performance Instant Messaging System

## 项目结构 / Project Structure

```
vgo-rust/
├── v/                          # 公共工具库 / Common library
├── v-connect-im/               # 即时通讯服务 / IM service
├── v-admin/                    # 管理后台后端 / Admin backend
├── v-admin-vue/                # 管理后台前端 / Admin frontend (Vue3)
├── v-auth-center/              # 认证中心 / Auth center
├── v-plugins-hub/              # 插件中心 / Plugin hub
│   ├── v-connect-im-plugin-storage-sled/
│   ├── v-connect-im-plugin-gateway/
│   └── ...
├── docs/                       # 项目文档 / Documentation (Mintlify)
├── scripts/                    # 构建和部署脚本 / Build and deploy scripts
├── config/                     # 配置文件 / Configuration files
├── sql/                        # 数据库迁移脚本 / Database migrations
└── Taskfile.yml               # 任务管理文件 / Task management
```

## 快速开始 / Quick Start

### 前置要求 / Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Redis 6+
- Task (可选，用于任务管理 / Optional, for task management)

### 安装 Task / Install Task

```bash
# macOS
brew install go-task/tap/go-task

# Linux
sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b ~/.local/bin

# Windows
choco install go-task
```

### 使用 Task 构建和运行 / Build and Run with Task

```bash
# 查看所有可用任务 / List all available tasks
task --list

# 构建所有插件 / Build all plugins
task build:plugins

# 构建生产版本 / Build release version
task build:release

# 启动开发服务器 / Start dev server
task dev:im

# 运行测试 / Run tests
task test

# 代码格式化和检查 / Format and lint
task fmt
task lint
```

详细的 Task 使用说明请查看 [TASKFILE_USAGE.md](./TASKFILE_USAGE.md)

For detailed Task usage, see [TASKFILE_USAGE.md](./TASKFILE_USAGE.md)

### 传统方式 / Traditional Way

如果不使用 Task，也可以直接运行脚本：

If not using Task, you can run scripts directly:

```bash
# 构建插件 / Build plugins
./scripts/build-plugins.sh

# 构建生产版本 / Build release
./scripts/build-release.sh

# 检查插件状态 / Check plugin status
./scripts/check-plugins.sh

# 清理插件 / Cleanup plugins
./scripts/cleanup-plugins.sh
```

## 开发 / Development

### 启动开发服务器 / Start Development Server

```bash
# 使用 Task / Using Task
task dev:im

# 或直接使用 Cargo / Or directly with Cargo
cd v-connect-im
cargo run
```

### 代码格式化 / Code Formatting

```bash
# 使用 Task / Using Task
task fmt

# 或直接使用 Cargo / Or directly with Cargo
cargo fmt --all
```

### 代码检查 / Linting

```bash
# 使用 Task / Using Task
task lint

# 或直接使用 Cargo / Or directly with Cargo
cargo clippy --workspace --all-targets -- -D warnings
```

### 运行测试 / Run Tests

```bash
# 使用 Task / Using Task
task test

# 或直接使用 Cargo / Or directly with Cargo
cargo test --workspace
```

## 构建 / Build

### 构建插件 / Build Plugins

```bash
# 使用 Task / Using Task
task build:plugins

# 构建指定插件 / Build specific plugin
task build:plugin PLUGIN=v-connect-im-plugin-storage-sled

# 或使用脚本 / Or using script
./scripts/build-plugins.sh
./scripts/build-plugins.sh v-connect-im-plugin-storage-sled
```

### 构建生产版本 / Build Release

```bash
# 使用 Task / Using Task
task build:release

# 构建到自定义目录 / Build to custom directory
task build:release:custom OUTPUT=~/deploy/v-connect-im

# 或使用脚本 / Or using script
./scripts/build-release.sh
./scripts/build-release.sh ~/deploy/v-connect-im
```

## 部署 / Deployment

构建完成后，在 `dist/v-connect-im/` 目录下会生成完整的部署包。

After building, a complete deployment package will be generated in `dist/v-connect-im/`.

详细部署说明请查看：

For detailed deployment instructions, see:

- [Docker 部署 / Docker Deployment](./docs/deployment/docker.mdx)
- [Kubernetes 部署 / Kubernetes Deployment](./docs/deployment/kubernetes.mdx)
- [生产环境部署 / Production Deployment](./docs/deployment/production.mdx)

## 文档 / Documentation

### 项目文档 / Project Documentation

项目使用 Mintlify 构建文档，位于 `docs/` 目录。

Project documentation is built with Mintlify, located in `docs/` directory.

```bash
# 启动文档服务器 / Start docs server
task docs:serve

# 或 / Or
cd docs
mintlify dev
```

### API 文档 / API Documentation

```bash
# 生成并打开 Rust API 文档 / Generate and open Rust API docs
task docs:open

# 或 / Or
cargo doc --workspace --no-deps --open
```

## 插件开发 / Plugin Development

插件开发指南请查看：

For plugin development guide, see:

- [插件架构 / Plugin Architecture](./docs/plugin/architecture.mdx)
- [插件开发最佳实践 / Plugin Best Practices](./docs/plugin/best-practices.mdx)
- [插件示例 / Plugin Examples](./examples/PLUGIN_EXAMPLES.md)

## 技术栈 / Tech Stack

### 后端 / Backend

- **Rust** - 系统编程语言 / Systems programming language
- **Tokio** - 异步运行时 / Async runtime
- **Actix-web** - Web 框架 / Web framework
- **SQLx** - 数据库访问 / Database access
- **Redis** - 缓存和消息队列 / Cache and message queue
- **Protocol Buffers** - 数据序列化 / Data serialization

### 前端 / Frontend

- **Vue 3** - 前端框架 / Frontend framework
- **TypeScript** - 类型安全 / Type safety
- **Element Plus** - UI 组件库 / UI component library

### 工具 / Tools

- **Task** - 任务管理 / Task management
- **Mintlify** - 文档生成 / Documentation
- **Docker** - 容器化 / Containerization
- **GitHub Actions** - CI/CD

## 贡献 / Contributing

欢迎贡献！请遵循以下步骤：

Contributions are welcome! Please follow these steps:

1. Fork 本仓库 / Fork the repository
2. 创建特性分支 / Create a feature branch (`git checkout -b feature/amazing-feature`)
3. 提交更改 / Commit your changes (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 / Push to the branch (`git push origin feature/amazing-feature`)
5. 创建 Pull Request / Create a Pull Request

### 代码规范 / Code Standards

- 遵循 Rust 官方代码风格 / Follow official Rust style guide
- 使用 `cargo fmt` 格式化代码 / Format code with `cargo fmt`
- 使用 `cargo clippy` 检查代码 / Lint code with `cargo clippy`
- 编写测试覆盖新功能 / Write tests for new features
- 更新相关文档 / Update relevant documentation

## 许可证 / License

[MIT License](./LICENSE)

## 联系方式 / Contact

- 项目主页 / Project Home: [GitHub Repository](https://github.com/vera-byte/vgo-rust)
- 问题反馈 / Issue Tracker: [GitHub Issues](https://github.com/vera-byte/vgo-rust/issues)

## 致谢 / Acknowledgments

感谢所有贡献者和开源项目的支持！

Thanks to all contributors and open source projects!
