# 依赖统一管理方案

## 目标

统一所有包的依赖版本，通过 v 库导出常用依赖，避免版本冲突。

## 已完成的工作

### 1. ✅ v 库导出常用依赖

**文件：** `/v/src/lib.rs`

**导出的依赖：**
```rust
// 异步运行时 / Async runtime
pub use tokio;

// 序列化 / Serialization
pub use serde;
pub use serde_json;

// 错误处理 / Error handling
pub use anyhow;
pub use thiserror;

// 异步 trait / Async trait
pub use async_trait;

// Protobuf / Protocol Buffers
#[cfg(feature = "protobuf")]
pub use prost;
#[cfg(feature = "protobuf")]
pub use prost_types;

// 时间处理 / Time handling
pub use chrono;

// 日志 / Logging
pub use tracing::{debug, error, info, trace, warn};
```

### 2. ✅ 更新 v-connect-im/Cargo.toml

**移除的重复依赖：**
- ❌ `tokio = "1.0"`
- ❌ `serde = "1.0"`
- ❌ `serde_json = "1.0"`
- ❌ `anyhow = "1.0"`
- ❌ `async-trait = "0.1"`
- ❌ `chrono = "0.4"`
- ❌ `prost = "0.13"`
- ❌ `prost-types = "0.13"`
- ❌ `tracing = "0.1"`
- ❌ `tracing-subscriber = "0.3"`
- ❌ `clap = "4.0"`

**保留的独立依赖：**
- ✅ `tokio-tungstenite = "0.20"` - WebSocket 特定
- ✅ `futures-util = "0.3"` - Future 工具
- ✅ `dashmap = "5.5"` - 并发 HashMap
- ✅ `uuid = "1.0"` - UUID 生成
- ✅ `actix-web = "4"` - Web 框架
- ✅ `reqwest = "0.11"` - HTTP 客户端
- ✅ `hmac = "0.12"` - 加密
- ✅ `sha2 = "0.10"` - 哈希
- ✅ `hex = "0.4"` - 十六进制编码

## 导入语句更新指南

### 方案 1：保持现有导入（推荐）

**优势：**
- 无需修改现有代码
- Rust 会自动使用 v 导出的版本
- 编译器会处理版本统一

**说明：**
```rust
// 这些导入会自动使用 v 导出的版本
use anyhow::Result;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
```

由于 `v-connect-im` 的 `Cargo.toml` 中移除了这些依赖的直接声明，
Rust 编译器会自动从 `v` 的依赖中解析这些类型。

### 方案 2：显式从 v 导入（可选）

**优势：**
- 更明确的依赖关系
- 便于理解代码结构

**示例：**
```rust
// 之前
use anyhow::Result;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

// 之后（可选）
use v::anyhow::Result;
use v::tokio::sync::mpsc;
use v::serde::{Deserialize, Serialize};
```

## 插件依赖更新

### v-connect-im-plugin-storage-sled

**更新 Cargo.toml：**
```toml
[dependencies]
# 核心依赖：从 v 导出
v = { path = "../../v", features = ["protobuf"] }

# 插件特定依赖
sled = "0.34"
```

**移除：**
- ❌ `tokio = "1.0"`
- ❌ `anyhow = "1.0"`
- ❌ `async-trait = "0.1"`
- ❌ `serde = "1.0"`
- ❌ `serde_json = "1.0"`
- ❌ `prost = "0.13"`
- ❌ `chrono = "0.4"`

### v-admin

**更新 Cargo.toml：**
```toml
[dependencies]
v = { path = "../v", features = ["web_actix", "protobuf"] }

# admin 特定依赖
# ...
```

### v-auth-center

**更新 Cargo.toml：**
```toml
[dependencies]
v = { path = "../v", features = ["protobuf"] }

# auth 特定依赖
# ...
```

## 版本统一表

| 依赖 | 版本 | 来源 |
|------|------|------|
| tokio | 1.x | v |
| serde | 1.x | v |
| serde_json | 1.x | v |
| anyhow | 1.x | v |
| thiserror | 2.x | v |
| async-trait | 0.1 | v |
| chrono | 0.4 | v |
| prost | 0.13 | v |
| prost-types | 0.13 | v |
| tracing | 0.1 | v |
| sqlx | 0.7 | v |
| actix-web | 4.x | v (可选) |

## 优势

### ✅ 版本统一
- 所有包使用相同版本的依赖
- 避免版本冲突
- 减少编译时间

### ✅ 依赖管理简化
- 只需在 v 中更新版本
- 其他包自动使用新版本
- 减少 Cargo.toml 维护成本

### ✅ 编译优化
- 减少重复编译
- 共享依赖缓存
- 更快的增量编译

### ✅ 二进制体积
- 避免重复链接
- 更小的最终二进制

## 编译验证

```bash
# 清理并重新编译
cargo clean
cargo build --workspace

# 检查依赖树
cargo tree -p v-connect-im | grep -E "(tokio|serde|anyhow|prost)"

# 验证版本统一
cargo tree -d
```

## 迁移步骤

### 阶段 1：v-connect-im ✅
- [x] 更新 Cargo.toml
- [x] 移除重复依赖
- [x] 验证编译

### 阶段 2：插件包
- [ ] v-connect-im-plugin-storage-sled
- [ ] v-connect-im-plugin-gateway
- [ ] 其他插件

### 阶段 3：其他服务
- [ ] v-admin
- [ ] v-auth-center
- [ ] v-admin-vue (前端，不需要)

### 阶段 4：验证
- [ ] 所有包编译通过
- [ ] 运行测试
- [ ] 检查依赖树

## 注意事项

### 1. feature flags

确保启用正确的 feature：
```toml
v = { path = "../v", features = ["protobuf", "web_actix"] }
```

### 2. 可选依赖

某些依赖可能需要保持独立：
- 特定版本要求
- 平台特定依赖
- 可选功能依赖

### 3. 构建依赖

build-dependencies 可能需要独立声明：
```toml
[build-dependencies]
prost-build = "0.13"
tonic-build = "0.11"
```

### 4. 开发依赖

dev-dependencies 通常保持独立：
```toml
[dev-dependencies]
tokio-test = "0.4"
```

## 回滚方案

如果遇到问题，可以回滚：

```toml
# 恢复直接依赖
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ...
```

## 相关文档

- [Cargo 工作区文档](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [依赖解析](https://doc.rust-lang.org/cargo/reference/resolver.html)

---

**创建日期**：2025-12-09  
**状态**：✅ v-connect-im 完成  
**维护者**：VGO Team
