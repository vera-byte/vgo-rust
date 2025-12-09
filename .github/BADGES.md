# GitHub Actions 徽章 / GitHub Actions Badges

将以下徽章添加到项目的 README.md 中，展示项目的构建状态。
Add the following badges to your project's README.md to show build status.

## 使用方法 / Usage

将 `{owner}` 和 `{repo}` 替换为你的 GitHub 用户名和仓库名。
Replace `{owner}` and `{repo}` with your GitHub username and repository name.

---

## 徽章代码 / Badge Code

### CI 状态 / CI Status

```markdown
[![CI](https://github.com/{owner}/{repo}/actions/workflows/ci.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/ci.yml)
```

示例 / Example:
[![CI](https://github.com/vera-byte/vgo-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/vera-byte/vgo-rust/actions/workflows/ci.yml)

---

### 构建和发布状态 / Build and Release Status

```markdown
[![Build and Release](https://github.com/{owner}/{repo}/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/build-and-release.yml)
```

示例 / Example:
[![Build and Release](https://github.com/vera-byte/vgo-rust/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/vera-byte/vgo-rust/actions/workflows/build-and-release.yml)

---

### 最新版本 / Latest Release

```markdown
[![GitHub release](https://img.shields.io/github/v/release/{owner}/{repo})](https://github.com/{owner}/{repo}/releases/latest)
```

示例 / Example:
[![GitHub release](https://img.shields.io/github/v/release/vera-byte/vgo-rust)](https://github.com/vera-byte/vgo-rust/releases/latest)

---

### 许可证 / License

```markdown
[![License](https://img.shields.io/github/license/{owner}/{repo})](LICENSE)
```

示例 / Example:
[![License](https://img.shields.io/github/license/vera-byte/vgo-rust)](LICENSE)

---

### Rust 版本 / Rust Version

```markdown
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
```

示例 / Example:
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

---

### 代码覆盖率 / Code Coverage

如果配置了 Codecov:
If Codecov is configured:

```markdown
[![codecov](https://codecov.io/gh/{owner}/{repo}/branch/main/graph/badge.svg)](https://codecov.io/gh/{owner}/{repo})
```

示例 / Example:
[![codecov](https://codecov.io/gh/vera-byte/vgo-rust/branch/main/graph/badge.svg)](https://codecov.io/gh/vera-byte/vgo-rust)

---

### Docker 镜像 / Docker Image

如果发布了 Docker 镜像:
If Docker images are published:

```markdown
[![Docker Image](https://img.shields.io/docker/v/{dockerhub-username}/v-connect-im?label=docker)](https://hub.docker.com/r/{dockerhub-username}/v-connect-im)
```

---

## 完整示例 / Complete Example

将以下内容添加到 README.md 的顶部：
Add the following to the top of your README.md:

```markdown
# vgo-rust

[![CI](https://github.com/vera-byte/vgo-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/vera-byte/vgo-rust/actions/workflows/ci.yml)
[![Build and Release](https://github.com/vera-byte/vgo-rust/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/vera-byte/vgo-rust/actions/workflows/build-and-release.yml)
[![GitHub release](https://img.shields.io/github/v/release/vera-byte/vgo-rust)](https://github.com/vera-byte/vgo-rust/releases/latest)
[![License](https://img.shields.io/github/license/vera-byte/vgo-rust)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

高性能即时通讯服务器 / High-performance Instant Messaging Server

[English](README.md) | [中文](README_CN.md)
```

---

## 自定义徽章 / Custom Badges

### 平台支持 / Platform Support

```markdown
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey)
```

示例 / Example:
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey)

---

### 架构支持 / Architecture Support

```markdown
![Architecture](https://img.shields.io/badge/arch-x86__64%20%7C%20ARM64-blue)
```

示例 / Example:
![Architecture](https://img.shields.io/badge/arch-x86__64%20%7C%20ARM64-blue)

---

### 状态 / Status

```markdown
![Status](https://img.shields.io/badge/status-active-success)
```

示例 / Example:
![Status](https://img.shields.io/badge/status-active-success)

---

## 更多徽章 / More Badges

访问 [shields.io](https://shields.io/) 创建自定义徽章。
Visit [shields.io](https://shields.io/) to create custom badges.

---

## 注意事项 / Notes

1. 徽章会自动更新，反映最新的构建状态
2. 点击徽章可以跳转到对应的 Actions 页面
3. 建议将徽章放在 README 的顶部，增加项目的专业性

1. Badges update automatically to reflect the latest build status
2. Clicking badges navigates to the corresponding Actions page
3. It's recommended to place badges at the top of README for professionalism
