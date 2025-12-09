# GitHub Actions 工作流说明
# GitHub Actions Workflow Documentation

本目录包含项目的 GitHub Actions 工作流配置。
This directory contains the GitHub Actions workflow configurations for the project.

## 工作流列表 / Workflow List

### 1. CI (持续集成 / Continuous Integration)

**文件**: `workflows/ci.yml`

**触发条件 / Triggers**:
- 推送到 `main`, `develop`, `feature/**` 分支
- 针对 `main`, `develop` 的 Pull Request

**功能 / Features**:
- ✅ 代码格式检查 (rustfmt)
- ✅ Clippy 代码质量检查
- ✅ 单元测试 (Linux & macOS)
- ✅ 文档测试
- ✅ 编译检查
- ✅ 依赖安全审计 (cargo-audit)
- ✅ 代码覆盖率报告 (可选)

**用途 / Purpose**:
确保每次代码提交都符合质量标准，及早发现问题。
Ensures every code commit meets quality standards and catches issues early.

---

### 2. Build and Release (构建和发布)

**文件**: `workflows/build-and-release.yml`

**触发条件 / Triggers**:
- 推送到 `main`, `develop` 分支
- 推送标签 `v*` (如 `v1.0.0`)
- 手动触发 (workflow_dispatch)

**功能 / Features**:
- ✅ 多平台构建 (Linux AMD64/ARM64, macOS Intel/Apple Silicon)
- ✅ 打包 v-connect-im 服务
- ✅ 打包插件 (storage-sled, gateway)
- ✅ 生成 SHA256 校验和
- ✅ 创建 GitHub Release
- ✅ 构建 Docker 镜像 (可选)

**构建产物 / Build Artifacts**:

#### v-connect-im 服务包
```
v-connect-im-{version}-{os}-{arch}.tar.gz
├── bin/
│   └── v-connect-im          # 主程序
├── config/
│   ├── default.toml          # 默认配置
│   └── production.toml       # 生产配置模板
├── logs/                      # 日志目录
├── plugins/
│   └── sockets/              # Socket 文件目录
├── data/                      # 数据目录
├── README.md                  # 说明文档
└── VERSION                    # 版本信息
```

#### 插件包
```
{plugin-name}-{version}-{os}-{arch}.vp
├── {plugin-binary}           # 插件二进制
├── plugin.json               # 插件配置
├── README.md                 # 说明文档 (可选)
└── VERSION                   # 版本信息
```

---

## 配置 GitHub Secrets / Configure GitHub Secrets

为了使工作流正常运行，需要配置以下 Secrets：
To make the workflows work properly, configure the following Secrets:

### Docker Hub (可选 / Optional)

如果需要构建和推送 Docker 镜像：
If you need to build and push Docker images:

1. 进入仓库设置 / Go to repository settings
2. 选择 "Secrets and variables" → "Actions"
3. 添加以下 Secrets / Add the following Secrets:

| Secret Name | Description | 说明 |
|------------|-------------|------|
| `DOCKER_USERNAME` | Docker Hub username | Docker Hub 用户名 |
| `DOCKER_PASSWORD` | Docker Hub password or token | Docker Hub 密码或令牌 |

### Codecov (可选 / Optional)

如果需要上传代码覆盖率报告：
If you need to upload code coverage reports:

| Secret Name | Description | 说明 |
|------------|-------------|------|
| `CODECOV_TOKEN` | Codecov upload token | Codecov 上传令牌 |

---

## 使用说明 / Usage Guide

### 日常开发 / Daily Development

1. **创建功能分支 / Create feature branch**:
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **提交代码 / Commit code**:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   git push origin feature/my-new-feature
   ```

3. **CI 自动运行 / CI runs automatically**:
   - 代码格式检查
   - Clippy 检查
   - 单元测试
   - 编译检查

4. **创建 Pull Request**:
   - CI 必须通过才能合并
   - CI must pass before merging

### 发布新版本 / Release New Version

#### 方式一：使用 Git 标签 / Method 1: Using Git Tags

```bash
# 1. 更新版本号 / Update version number
# 编辑 Cargo.toml 文件中的 version 字段
# Edit the version field in Cargo.toml files

# 2. 提交更改 / Commit changes
git add .
git commit -m "chore: bump version to 1.0.0"

# 3. 创建标签 / Create tag
git tag -a v1.0.0 -m "Release v1.0.0"

# 4. 推送标签 / Push tag
git push origin v1.0.0
```

#### 方式二：手动触发 / Method 2: Manual Trigger

1. 进入 GitHub 仓库的 "Actions" 页面
2. 选择 "Build and Release" 工作流
3. 点击 "Run workflow"
4. 选择分支和发布类型 (snapshot/release)
5. 点击 "Run workflow"

### 构建产物下载 / Download Build Artifacts

#### 从 GitHub Actions

1. 进入 "Actions" 页面
2. 选择对应的工作流运行
3. 在 "Artifacts" 部分下载构建产物
4. 产物保留 30 天

#### 从 GitHub Releases

1. 进入仓库的 "Releases" 页面
2. 选择对应的版本
3. 下载所需的平台包和插件
4. 验证 SHA256 校验和：
   ```bash
   sha256sum -c v-connect-im-*.tar.gz.sha256
   ```

---

## 本地测试工作流 / Test Workflows Locally

使用 [act](https://github.com/nektos/act) 在本地测试 GitHub Actions：
Use [act](https://github.com/nektos/act) to test GitHub Actions locally:

```bash
# 安装 act / Install act
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# 测试 CI 工作流 / Test CI workflow
act -j test

# 测试构建工作流 / Test build workflow
act -j build

# 列出所有任务 / List all jobs
act -l
```

---

## 工作流优化建议 / Workflow Optimization Tips

### 1. 缓存策略 / Caching Strategy

工作流已配置 Cargo 缓存，可以显著加速构建：
Workflows are configured with Cargo caching to speed up builds:

- `~/.cargo/bin/` - Cargo 安装的工具
- `~/.cargo/registry/` - 依赖注册表
- `~/.cargo/git/` - Git 依赖
- `target/` - 编译产物

### 2. 并行构建 / Parallel Builds

构建矩阵支持多平台并行构建，充分利用 GitHub Actions 的并发能力。
Build matrix supports parallel multi-platform builds, fully utilizing GitHub Actions concurrency.

### 3. 条件执行 / Conditional Execution

某些任务仅在特定条件下运行：
Some jobs only run under specific conditions:

- Docker 构建：仅在 `main` 分支或标签推送时
- Release 创建：仅在标签推送或手动触发时
- 代码覆盖率：仅在推送到 `main` 分支时

---

## 故障排查 / Troubleshooting

### 构建失败 / Build Failures

1. **依赖问题 / Dependency Issues**:
   ```bash
   # 更新 Cargo.lock
   cargo update
   ```

2. **格式检查失败 / Format Check Fails**:
   ```bash
   # 本地运行格式化
   cargo fmt --all
   ```

3. **Clippy 警告 / Clippy Warnings**:
   ```bash
   # 本地运行 Clippy
   cargo clippy --all-targets --all-features -- -D warnings
   ```

### Docker 构建失败 / Docker Build Fails

1. 检查 Dockerfile 语法
2. 确保所有依赖都已正确安装
3. 验证 Docker Hub 凭据是否正确配置

### Release 创建失败 / Release Creation Fails

1. 确保标签格式正确 (v*.*.*)
2. 检查 GitHub Token 权限
3. 验证构建产物是否存在

---

## 维护和更新 / Maintenance and Updates

### 更新 Rust 版本 / Update Rust Version

编辑工作流文件中的 Rust 工具链版本：
Edit the Rust toolchain version in workflow files:

```yaml
- uses: dtolnay/rust-toolchain@stable
  # 或指定版本 / Or specify version
  # with:
  #   toolchain: 1.75.0
```

### 添加新平台支持 / Add New Platform Support

在 `build-and-release.yml` 的构建矩阵中添加新平台：
Add new platforms in the build matrix of `build-and-release.yml`:

```yaml
matrix:
  include:
    - os: windows
      arch: amd64
      runner: windows-latest
      target: x86_64-pc-windows-msvc
```

### 添加新插件 / Add New Plugins

在打包步骤中添加新插件的构建和打包逻辑。
Add build and packaging logic for new plugins in the packaging step.

---

## 相关资源 / Related Resources

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Rust GitHub Actions](https://github.com/actions-rs)
- [Docker Build Push Action](https://github.com/docker/build-push-action)
- [act - 本地测试工具](https://github.com/nektos/act)

---

## 联系方式 / Contact

如有问题或建议，请创建 Issue 或 Pull Request。
For questions or suggestions, please create an Issue or Pull Request.
