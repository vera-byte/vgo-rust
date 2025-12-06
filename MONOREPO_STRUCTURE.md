# VGO-Rust Monorepo 项目结构 / Project Structure

## 概述 / Overview

本项目采用 **Monorepo（大仓库）** 模式管理多语言、多服务的微服务架构。
This project uses **Monorepo** pattern to manage multi-language, multi-service microservice architecture.

## 项目架构 / Architecture

```
vgo-rust/
├── v/                          # 公共工具库 / Common library (Rust)
├── v-auth-center/              # 认证中心服务 / Auth service (Rust)
├── v-admin/                    # 管理后台后端 / Admin backend (Rust)
├── v-admin-vue/                # 管理后台前端 / Admin frontend (Vue3)
├── v-connect-im/               # 即时通讯服务 / IM service (Rust)
├── v-plugins-hub/              # 插件中心 / Plugin hub (Rust)
│   ├── v-connect-im-plugin-storage-sled/  # Sled存储插件
│   └── ...                     # 更多插件 / More plugins
├── docs/                       # 项目文档 / Documentation (Mintlify)
├── examples/                   # 示例代码 / Examples
├── config/                     # 配置文件 / Configuration
└── sql/                        # 数据库脚本 / SQL scripts
```

## Rust 工作空间结构 / Rust Workspace Structure

### 根工作空间 / Root Workspace

`/Cargo.toml` 管理所有 Rust 项目：
- **公共库**: `v`
- **核心服务**: `v-auth-center`, `v-admin`, `v-connect-im`
- **示例项目**: `examples`
- **插件项目**: `v-plugins-hub/*` (通配符包含所有插件)

### 插件工作空间 / Plugin Workspace

`/v-plugins-hub/Cargo.toml` 管理所有插件项目：
- 使用工作空间级别的依赖配置
- 统一版本管理
- 相对路径引用公共库 `v`

## 依赖关系 / Dependencies

```
v (公共库 / Common Library)
  ↑
  ├── v-auth-center
  ├── v-admin
  ├── v-connect-im
  └── v-plugins-hub/*
```

所有服务和插件都依赖于 `v` 公共库，避免代码重复。
All services and plugins depend on the `v` common library to avoid code duplication.

## 技术栈 / Tech Stack

### 后端 / Backend (Rust)
- **框架**: actix-web
- **异步运行时**: tokio
- **数据库**: PostgreSQL (sqlx)
- **缓存**: Redis
- **日志**: tracing

### 前端 / Frontend (Vue3)
- **框架**: Vue 3 + TypeScript
- **UI库**: Element Plus
- **构建工具**: Vite

### 文档 / Documentation
- **工具**: Mintlify

## 开发规范 / Development Guidelines

### 1. 代码组织 / Code Organization
- 所有 Rust 服务必须依赖 `v` 公共库
- 使用工作空间依赖，避免绝对路径
- 插件项目统一放在 `v-plugins-hub/` 下

### 2. API 规范 / API Standards
- 所有接口使用 `actix-web` 实现
- 响应使用 `v::response::respond_any` 方法
- 接口路径: `src/api/<版本>/<分组>/<接口名>.rs`

### 3. 命名规范 / Naming Conventions
- **Rust**: 蛇形命名法 (snake_case) 用于变量和函数
- **Rust**: 帕斯卡命名法 (PascalCase) 用于类型和结构体
- **Vue**: 组件使用 PascalCase，文件使用 kebab-case

### 4. 注释规范 / Comment Standards
- 所有代码必须包含双语注释（中文 + 英文）
- 格式: `// 中文说明 / English description`

## Git 管理策略 / Git Strategy

### Monorepo 优势 / Advantages
✅ **统一版本管理**: 所有服务版本同步
✅ **代码共享**: 公共库 `v` 易于维护和更新
✅ **原子提交**: 跨服务修改可以在一个 commit 中完成
✅ **简化 CI/CD**: 统一的构建和部署流程
✅ **依赖管理**: Cargo 工作空间自动处理依赖版本

### .gitignore 策略 / .gitignore Strategy
- 根目录统一管理忽略规则
- 分类清晰：Rust、前端、IDE、系统文件
- 插件编译产物单独配置

## 构建和部署 / Build & Deployment

### 本地开发 / Local Development
```bash
# 构建所有 Rust 项目 / Build all Rust projects
cargo build --workspace

# 构建特定服务 / Build specific service
cargo build -p v-auth-center

# 运行测试 / Run tests
cargo test --workspace
```

### 前端开发 / Frontend Development
```bash
cd v-admin-vue
npm install
npm run dev
```

### 插件开发 / Plugin Development
```bash
cd v-plugins-hub
cargo build --workspace
```

## 扩展指南 / Extension Guide

### 添加新的 Rust 服务 / Add New Rust Service
1. 在根目录创建新服务目录
2. 在根 `Cargo.toml` 的 `members` 中添加服务路径
3. 在服务的 `Cargo.toml` 中添加 `v = { path = "../v" }` 依赖

### 添加新的插件 / Add New Plugin
1. 在 `v-plugins-hub/` 下创建插件目录
2. 插件会自动被 `v-plugins-hub/*` 通配符包含
3. 使用工作空间依赖: `v = { workspace = true }`

### 添加新的前端项目 / Add New Frontend Project
1. 在根目录创建前端项目目录
2. 在根 `Cargo.toml` 的 `exclude` 中添加项目路径

## 最佳实践 / Best Practices

### 1. 模块化设计 / Modular Design
- 公共功能抽取到 `v` 库
- 服务间通过 API 通信，避免直接依赖

### 2. 异步编程 / Async Programming
- 使用 `tokio` 处理并发
- 避免阻塞操作
- 合理使用通道 (channel) 进行任务通信

### 3. 错误处理 / Error Handling
- 使用 `Result` 和 `Option` 类型
- 自定义错误类型使用 `thiserror`
- 应用层错误使用 `anyhow`

### 4. 测试策略 / Testing Strategy
- 单元测试: 每个模块独立测试
- 集成测试: 测试服务间交互
- 插件测试: 验证插件与主服务通信

## 性能优化 / Performance Optimization

### 1. 编译优化 / Compilation
- 使用 `cargo build --release` 生产构建
- 配置 LTO (Link Time Optimization)
- 使用 `cargo-chef` 优化 Docker 构建缓存

### 2. 运行时优化 / Runtime
- 合理配置 tokio 线程池
- 使用连接池管理数据库连接
- 实现缓存策略减少数据库查询

## 文档维护 / Documentation Maintenance

- **API 文档**: `/docs/api-reference/`
- **架构文档**: `/docs/concepts/`
- **部署文档**: `/docs/deployment/`
- **插件文档**: `/docs/plugin/`

所有文档使用 Mintlify 编写，支持交互式示例和代码高亮。
All documentation is written in Mintlify with interactive examples and code highlighting.

---

## 联系方式 / Contact

如有问题或建议，请提交 Issue 或 Pull Request。
For questions or suggestions, please submit an Issue or Pull Request.
