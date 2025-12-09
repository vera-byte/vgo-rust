# GitHub Actions 文件清单
# GitHub Actions Files Checklist

本文档列出了为项目创建的所有 GitHub Actions 相关文件。
This document lists all GitHub Actions related files created for the project.

---

## 📁 创建的文件 / Created Files

### 1. GitHub Actions 工作流 / Workflows

#### `.github/workflows/ci.yml`
**持续集成工作流 / Continuous Integration Workflow**

- ✅ 代码格式检查 (rustfmt)
- ✅ Clippy 代码质量检查
- ✅ 单元测试 (Linux & macOS)
- ✅ 文档测试
- ✅ 编译检查
- ✅ 依赖安全审计
- ✅ 代码覆盖率 (可选)

**触发条件**:
- 推送到 `main`, `develop`, `feature/**` 分支
- Pull Request 到 `main`, `develop` 分支

---

#### `.github/workflows/build-and-release.yml`
**构建和发布工作流 / Build and Release Workflow**

- ✅ 多平台构建 (Linux AMD64/ARM64, macOS Intel/Apple Silicon)
- ✅ 打包 v-connect-im 服务
- ✅ 打包插件 (storage-sled, gateway)
- ✅ 生成 SHA256 校验和
- ✅ 创建 GitHub Release
- ✅ 构建 Docker 镜像 (可选)

**触发条件**:
- 推送到 `main`, `develop` 分支
- 推送标签 `v*`
- 手动触发

**构建产物**:
- `v-connect-im-{version}-{os}-{arch}.tar.gz`
- `storage-sled-{version}-{os}-{arch}.vp`
- `gateway-{version}-{os}-{arch}.vp`
- 对应的 `.sha256` 文件

---

#### `.github/workflows/manual-build.yml`
**手动构建工作流 / Manual Build Workflow**

- ✅ 可选择构建组件 (all/v-connect-im/plugins)
- ✅ 可选择目标平台 (all/linux-amd64/linux-arm64/darwin-amd64/darwin-arm64)
- ✅ 可选择构建类型 (release/debug)
- ✅ 可选择是否创建 Release

**触发条件**:
- 仅手动触发

**用途**:
- 测试特定平台的构建
- 快速构建单个组件
- 创建测试版本

---

### 2. Docker 配置 / Docker Configuration

#### `v-connect-im/Dockerfile`
**多阶段 Docker 构建文件 / Multi-stage Docker Build File**

- ✅ 基于 Rust 1.75 构建
- ✅ 最小化运行时镜像 (Debian Bookworm Slim)
- ✅ 非 root 用户运行
- ✅ 包含健康检查
- ✅ 优化的层缓存

**特性**:
- 构建 v-connect-im 主服务
- 构建并包含插件
- 暴露端口 8080 (HTTP) 和 8081 (WebSocket)

---

#### `.dockerignore`
**Docker 构建忽略文件 / Docker Build Ignore File**

- ✅ 排除不必要的文件
- ✅ 减小构建上下文大小
- ✅ 加速 Docker 构建

---

### 3. 文档 / Documentation

#### `.github/README.md`
**GitHub Actions 工作流详细说明 / Detailed Workflow Documentation**

内容包括 / Contents:
- 工作流列表和说明
- GitHub Secrets 配置指南
- 使用说明和示例
- 构建产物说明
- 故障排查指南
- 维护和更新指南

---

#### `.github/BADGES.md`
**GitHub Actions 徽章配置 / Badge Configuration**

内容包括 / Contents:
- CI 状态徽章
- 构建状态徽章
- 版本徽章
- 许可证徽章
- 代码覆盖率徽章
- Docker 镜像徽章
- 自定义徽章示例

---

#### `GITHUB_ACTIONS_SETUP.md`
**GitHub Actions 快速配置指南 / Quick Setup Guide**

内容包括 / Contents:
- 快速开始步骤
- 工作流说明
- 配置步骤
- 使用示例
- 构建产物说明
- Docker 使用指南
- 本地测试方法
- 常见问题解答

---

#### `GITHUB_ACTIONS_FILES.md` (本文件)
**文件清单 / Files Checklist**

---

### 4. 脚本 / Scripts

#### `scripts/validate-workflows.sh`
**工作流验证脚本 / Workflow Validation Script**

功能 / Features:
- ✅ 检查 YAML 语法
- ✅ 使用 actionlint 进行详细检查
- ✅ 验证必需字段
- ✅ 检查项目结构
- ✅ 检查 Docker 配置
- ✅ 检查脚本权限

使用方法 / Usage:
```bash
./scripts/validate-workflows.sh
```

---

## 📊 文件结构 / File Structure

```
vgo-rust/
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                      # CI 工作流
│   │   ├── build-and-release.yml       # 构建和发布工作流
│   │   └── manual-build.yml            # 手动构建工作流
│   ├── README.md                        # 工作流详细说明
│   └── BADGES.md                        # 徽章配置
├── v-connect-im/
│   └── Dockerfile                       # Docker 构建文件
├── scripts/
│   └── validate-workflows.sh            # 工作流验证脚本
├── .dockerignore                        # Docker 忽略文件
├── GITHUB_ACTIONS_SETUP.md              # 快速配置指南
└── GITHUB_ACTIONS_FILES.md              # 本文件
```

---

## ✅ 配置检查清单 / Configuration Checklist

### 必需步骤 / Required Steps

- [ ] 1. 验证工作流配置
  ```bash
  ./scripts/validate-workflows.sh
  ```

- [ ] 2. 提交所有文件
  ```bash
  git add .github/ v-connect-im/Dockerfile .dockerignore scripts/validate-workflows.sh
  git add GITHUB_ACTIONS_SETUP.md GITHUB_ACTIONS_FILES.md
  git commit -m "ci: add GitHub Actions workflows and documentation"
  ```

- [ ] 3. 推送到 GitHub
  ```bash
  git push origin main
  ```

- [ ] 4. 验证 Actions 运行
  - 访问 `https://github.com/{owner}/{repo}/actions`
  - 检查 CI 工作流是否成功运行

---

### 可选步骤 / Optional Steps

- [ ] 5. 配置 Docker Hub Secrets (如果需要 Docker 镜像)
  - `DOCKER_USERNAME`
  - `DOCKER_PASSWORD`

- [ ] 6. 配置 Codecov (如果需要代码覆盖率)
  - `CODECOV_TOKEN`

- [ ] 7. 配置分支保护规则
  - 要求 CI 通过才能合并
  - 要求分支保持最新

- [ ] 8. 添加徽章到 README
  - 参考 `.github/BADGES.md`

- [ ] 9. 创建第一个 Release
  ```bash
  git tag -a v1.0.0 -m "Release v1.0.0"
  git push origin v1.0.0
  ```

---

## 🎯 工作流使用场景 / Workflow Use Cases

### 场景 1: 日常开发
**使用工作流**: `ci.yml`

```bash
# 1. 创建功能分支
git checkout -b feature/new-feature

# 2. 开发并提交
git add .
git commit -m "feat: add new feature"

# 3. 推送 (触发 CI)
git push origin feature/new-feature

# 4. 创建 PR
# CI 自动运行，必须通过才能合并
```

---

### 场景 2: 发布新版本
**使用工作流**: `build-and-release.yml`

```bash
# 1. 更新版本号
# 编辑 Cargo.toml 文件

# 2. 提交并创建标签
git add .
git commit -m "chore: bump version to 1.0.0"
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 3. 等待构建完成
# 访问 GitHub Releases 页面下载产物
```

---

### 场景 3: 测试特定平台
**使用工作流**: `manual-build.yml`

1. 访问 Actions 页面
2. 选择 "Manual Build"
3. 点击 "Run workflow"
4. 选择参数:
   - Component: `v-connect-im`
   - Platform: `linux-amd64`
   - Build Type: `release`
5. 下载构建产物测试

---

### 场景 4: 构建 Docker 镜像
**使用工作流**: `build-and-release.yml`

```bash
# 推送到 main 分支或创建标签
git push origin main

# 或
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# Docker 镜像会自动构建并推送到 Docker Hub
# 镜像标签: {username}/v-connect-im:latest
#          {username}/v-connect-im:v1.0.0
```

---

## 🔧 维护指南 / Maintenance Guide

### 更新 Rust 版本

编辑工作流文件中的工具链版本:

```yaml
- uses: dtolnay/rust-toolchain@stable
  # 或指定版本
  with:
    toolchain: 1.75.0
```

---

### 添加新平台

在 `build-and-release.yml` 的矩阵中添加:

```yaml
matrix:
  include:
    - os: windows
      arch: amd64
      runner: windows-latest
      target: x86_64-pc-windows-msvc
```

---

### 添加新插件

在打包步骤中添加新插件的构建逻辑:

```bash
cargo build --release --package v-connect-im-plugin-new-plugin --target ${{ matrix.target }}
```

---

### 修改构建产物

编辑 `build-and-release.yml` 中的打包步骤，修改目录结构和包含的文件。

---

## 📈 性能优化 / Performance Optimization

### 1. 缓存策略
- ✅ Cargo 依赖缓存
- ✅ Docker 层缓存
- ✅ 构建产物缓存

### 2. 并行构建
- ✅ 多平台并行构建
- ✅ 独立任务并行执行

### 3. 条件执行
- ✅ 仅在必要时运行 Docker 构建
- ✅ 仅在标签推送时创建 Release
- ✅ 仅在 main 分支运行代码覆盖率

---

## 🐛 故障排查 / Troubleshooting

### 问题 1: 工作流语法错误

**解决方案**:
```bash
# 运行验证脚本
./scripts/validate-workflows.sh

# 或使用 actionlint
actionlint .github/workflows/*.yml
```

---

### 问题 2: 构建失败

**解决方案**:
1. 查看 Actions 日志
2. 在本地复现问题
3. 检查依赖版本
4. 清理缓存重试

---

### 问题 3: Docker 构建失败

**解决方案**:
```bash
# 本地测试 Docker 构建
docker build -f v-connect-im/Dockerfile .

# 检查 Dockerfile 语法
docker build --dry-run -f v-connect-im/Dockerfile .
```

---

## 📚 相关资源 / Related Resources

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Rust GitHub Actions](https://github.com/actions-rs)
- [actionlint](https://github.com/rhysd/actionlint)
- [act - 本地测试](https://github.com/nektos/act)

---

## 🎉 完成 / Completion

所有 GitHub Actions 相关文件已创建完成！
All GitHub Actions related files have been created!

### 下一步 / Next Steps

1. ✅ 运行验证脚本
2. ✅ 提交所有文件
3. ✅ 推送到 GitHub
4. ✅ 查看 Actions 运行
5. ✅ 配置 Secrets (可选)
6. ✅ 添加徽章到 README
7. ✅ 创建第一个 Release

祝你使用愉快！🚀
Happy building! 🚀
