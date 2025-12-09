# GitHub Actions 配置指南
# GitHub Actions Setup Guide

本文档提供 GitHub Actions 的快速配置和使用指南。
This document provides a quick setup and usage guide for GitHub Actions.

---

## 📋 目录 / Table of Contents

- [快速开始](#快速开始--quick-start)
- [工作流说明](#工作流说明--workflow-description)
- [配置步骤](#配置步骤--configuration-steps)
- [使用示例](#使用示例--usage-examples)
- [常见问题](#常见问题--faq)

---

## 🚀 快速开始 / Quick Start

### 1. 验证配置 / Validate Configuration

运行验证脚本检查所有配置是否正确：
Run the validation script to check if all configurations are correct:

```bash
./scripts/validate-workflows.sh
```

### 2. 提交工作流文件 / Commit Workflow Files

```bash
git add .github/
git commit -m "ci: add GitHub Actions workflows"
git push origin main
```

### 3. 查看 Actions 运行 / View Actions Runs

访问 GitHub 仓库的 Actions 页面：
Visit the Actions page of your GitHub repository:

```
https://github.com/{owner}/{repo}/actions
```

---

## 📝 工作流说明 / Workflow Description

### CI 工作流 (ci.yml)

**触发时机 / Triggers**:
- 推送到 `main`, `develop`, `feature/**` 分支
- Pull Request 到 `main`, `develop` 分支

**执行任务 / Tasks**:
1. ✅ 代码格式检查 (rustfmt)
2. ✅ Clippy 代码质量检查
3. ✅ 单元测试 (Linux & macOS)
4. ✅ 文档测试
5. ✅ 编译检查
6. ✅ 依赖安全审计

**运行时间 / Duration**: 约 5-10 分钟

---

### 构建和发布工作流 (build-and-release.yml)

**触发时机 / Triggers**:
- 推送到 `main`, `develop` 分支
- 推送标签 `v*` (如 `v1.0.0`)
- 手动触发

**执行任务 / Tasks**:
1. ✅ 多平台构建 (Linux AMD64/ARM64, macOS Intel/Apple Silicon)
2. ✅ 打包 v-connect-im 服务
3. ✅ 打包插件 (storage-sled, gateway)
4. ✅ 生成 SHA256 校验和
5. ✅ 创建 GitHub Release
6. ✅ 构建 Docker 镜像 (可选)

**运行时间 / Duration**: 约 15-30 分钟

**构建产物 / Artifacts**:
- `v-connect-im-{version}-{os}-{arch}.tar.gz` - 主服务包
- `storage-sled-{version}-{os}-{arch}.vp` - 存储插件
- `gateway-{version}-{os}-{arch}.vp` - 网关插件
- 对应的 `.sha256` 校验和文件

---

## ⚙️ 配置步骤 / Configuration Steps

### 步骤 1: 配置 GitHub Secrets (可选)

如果需要使用 Docker Hub 或 Codecov，需要配置相应的 Secrets。
If you need to use Docker Hub or Codecov, configure the corresponding Secrets.

#### Docker Hub

1. 访问仓库设置 / Visit repository settings:
   ```
   https://github.com/{owner}/{repo}/settings/secrets/actions
   ```

2. 点击 "New repository secret"

3. 添加以下 Secrets / Add the following Secrets:
   - `DOCKER_USERNAME`: Docker Hub 用户名
   - `DOCKER_PASSWORD`: Docker Hub 密码或访问令牌

#### Codecov (可选)

1. 访问 [Codecov](https://codecov.io/) 并登录
2. 添加你的仓库
3. 获取上传令牌 (Upload Token)
4. 在 GitHub Secrets 中添加 `CODECOV_TOKEN`

### 步骤 2: 启用 GitHub Actions

1. 访问仓库的 Actions 页面
2. 如果 Actions 被禁用，点击 "I understand my workflows, go ahead and enable them"
3. 工作流将自动运行

### 步骤 3: 配置分支保护 (推荐)

1. 访问仓库设置 → Branches
2. 添加分支保护规则 (Branch protection rule)
3. 选择 `main` 分支
4. 启用以下选项：
   - ✅ Require status checks to pass before merging
   - ✅ Require branches to be up to date before merging
   - 选择必需的状态检查：
     - CI / fmt
     - CI / clippy
     - CI / test
     - CI / build

---

## 💡 使用示例 / Usage Examples

### 示例 1: 日常开发流程

```bash
# 1. 创建功能分支
git checkout -b feature/new-feature

# 2. 开发并提交代码
git add .
git commit -m "feat: add new feature"

# 3. 推送分支 (触发 CI)
git push origin feature/new-feature

# 4. 创建 Pull Request
# CI 会自动运行，必须通过才能合并
```

### 示例 2: 发布新版本

```bash
# 1. 更新版本号
# 编辑 v-connect-im/Cargo.toml
version = "1.0.0"

# 编辑插件的 Cargo.toml
# v-plugins-hub/v-connect-im-plugin-storage-sled/Cargo.toml
# v-plugins-hub/v-connect-im-plugin-gateway/Cargo.toml

# 2. 提交版本更新
git add .
git commit -m "chore: bump version to 1.0.0"
git push origin main

# 3. 创建并推送标签
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 4. 等待构建完成
# 访问 https://github.com/{owner}/{repo}/actions 查看进度

# 5. 下载发布产物
# 访问 https://github.com/{owner}/{repo}/releases
```

### 示例 3: 手动触发构建

1. 访问 Actions 页面
2. 选择 "Build and Release" 工作流
3. 点击 "Run workflow"
4. 选择分支和发布类型
5. 点击 "Run workflow" 按钮

---

## 📦 构建产物说明 / Build Artifacts

### v-connect-im 服务包

```
v-connect-im-1.0.0-linux-amd64.tar.gz
├── bin/
│   └── v-connect-im          # 主程序二进制
├── config/
│   ├── default.toml          # 默认配置
│   └── production.toml       # 生产配置模板
├── logs/                      # 日志目录 (空)
├── plugins/
│   └── sockets/              # Socket 文件目录 (空)
├── data/                      # 数据目录 (空)
├── README.md                  # 使用说明
└── VERSION                    # 版本信息
```

### 插件包

```
storage-sled-0.1.0-linux-amd64.vp
├── v-connect-im-plugin-storage-sled  # 插件二进制
├── plugin.json                        # 插件配置
├── README.md                          # 说明文档 (可选)
└── VERSION                            # 版本信息
```

### 使用方法 / Usage

```bash
# 1. 下载并解压服务包
tar -xzf v-connect-im-1.0.0-linux-amd64.tar.gz
cd v-connect-im-1.0.0-linux-amd64

# 2. 验证校验和
sha256sum -c ../v-connect-im-1.0.0-linux-amd64.tar.gz.sha256

# 3. 配置服务
cp config/default.toml config/production.toml
vim config/production.toml

# 4. 安装插件 (可选)
mkdir -p plugins
tar -xzf ../storage-sled-0.1.0-linux-amd64.vp -C plugins/

# 5. 运行服务
./bin/v-connect-im
```

---

## 🐳 Docker 使用 / Docker Usage

### 拉取镜像 / Pull Image

```bash
docker pull {dockerhub-username}/v-connect-im:latest
```

### 运行容器 / Run Container

```bash
docker run -d \
  --name v-connect-im \
  -p 8080:8080 \
  -p 8081:8081 \
  -v $(pwd)/config:/app/config \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  {dockerhub-username}/v-connect-im:latest
```

### 使用 Docker Compose

创建 `docker-compose.yml`:

```yaml
version: '3.8'

services:
  v-connect-im:
    image: {dockerhub-username}/v-connect-im:latest
    ports:
      - "8080:8080"
      - "8081:8081"
    volumes:
      - ./config:/app/config
      - ./data:/app/data
      - ./logs:/app/logs
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    restart: unless-stopped
```

运行:

```bash
docker-compose up -d
```

---

## 🔧 本地测试 / Local Testing

### 使用 act 测试工作流

[act](https://github.com/nektos/act) 允许在本地运行 GitHub Actions。
[act](https://github.com/nektos/act) allows you to run GitHub Actions locally.

#### 安装 act / Install act

```bash
# macOS
brew install act

# Linux
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
```

#### 运行测试 / Run Tests

```bash
# 列出所有任务
act -l

# 测试 CI 工作流
act -j test

# 测试构建工作流
act -j build

# 测试特定事件
act push
act pull_request
```

### 本地构建测试

```bash
# 测试服务构建
cargo build --release --package v-connect-im

# 测试插件构建
./scripts/build-plugins.sh

# 测试完整打包
./scripts/build-release.sh
```

---

## ❓ 常见问题 / FAQ

### Q1: 构建失败怎么办？

**A**: 
1. 检查 Actions 日志查看具体错误
2. 在本地运行相同的命令进行调试
3. 确保所有依赖都已正确配置
4. 运行 `./scripts/validate-workflows.sh` 检查配置

### Q2: 如何跳过 CI 检查？

**A**: 
在 commit 消息中添加 `[skip ci]` 或 `[ci skip]`:

```bash
git commit -m "docs: update README [skip ci]"
```

### Q3: 如何只构建特定平台？

**A**: 
手动触发工作流时，可以修改构建矩阵。或者创建一个新的工作流文件，只包含需要的平台。

### Q4: Docker 构建失败怎么办？

**A**:
1. 检查 Docker Hub 凭据是否正确
2. 确保 Dockerfile 语法正确
3. 本地测试 Docker 构建：
   ```bash
   docker build -f v-connect-im/Dockerfile .
   ```

### Q5: 如何加速构建？

**A**:
1. 工作流已配置 Cargo 缓存
2. 使用 `sccache` 进一步加速编译
3. 减少构建矩阵中的平台数量
4. 使用 GitHub Actions 的并发限制

### Q6: 如何添加新的构建平台？

**A**:
编辑 `.github/workflows/build-and-release.yml`，在 `matrix.include` 中添加新平台：

```yaml
- os: windows
  arch: amd64
  runner: windows-latest
  target: x86_64-pc-windows-msvc
```

### Q7: 如何自定义发布说明？

**A**:
编辑 `.github/workflows/build-and-release.yml` 中的 "生成发布说明" 步骤，修改 `release_notes.md` 的内容。

---

## 📚 相关资源 / Related Resources

### 官方文档 / Official Documentation
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Rust GitHub Actions](https://github.com/actions-rs)
- [Docker Build Push Action](https://github.com/docker/build-push-action)

### 工具 / Tools
- [act - 本地测试工具](https://github.com/nektos/act)
- [actionlint - 工作流检查工具](https://github.com/rhysd/actionlint)
- [cargo-audit - 依赖审计工具](https://github.com/rustsec/rustsec)

### 项目文档 / Project Documentation
- [工作流详细说明](.github/README.md)
- [徽章配置](.github/BADGES.md)
- [项目文档](docs/)

---

## 🤝 贡献 / Contributing

如果你发现任何问题或有改进建议，欢迎：
If you find any issues or have suggestions for improvement, feel free to:

1. 创建 Issue
2. 提交 Pull Request
3. 参与讨论

---

## 📄 许可证 / License

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

## ✨ 下一步 / Next Steps

1. ✅ 运行验证脚本: `./scripts/validate-workflows.sh`
2. ✅ 提交工作流文件: `git add .github/ && git commit -m "ci: add GitHub Actions"`
3. ✅ 推送到 GitHub: `git push origin main`
4. ✅ 查看 Actions 运行: 访问 GitHub Actions 页面
5. ✅ 配置 Secrets (如果需要 Docker 或 Codecov)
6. ✅ 添加徽章到 README: 参考 `.github/BADGES.md`
7. ✅ 创建第一个 Release: `git tag v1.0.0 && git push origin v1.0.0`

祝你使用愉快！🎉
Happy building! 🎉
