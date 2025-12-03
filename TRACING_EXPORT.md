# Tracing 宏从 v 库导出 / Tracing Macros Exported from v Library

## 修改内容 / Changes

### 1. v/src/lib.rs

添加了 tracing 宏的重新导出：

```rust
// 重新导出 tracing 宏，方便插件开发
// Re-export tracing macros for plugin development convenience
pub use tracing::{debug, error, info, trace, warn};
```

### 2. 插件使用方式

**之前：**
```rust
use tracing::{debug, info};
use v::plugin::pdk::{json, Context, Plugin};
```

**现在：**
```rust
use v::plugin::pdk::{json, Context, Plugin};
use v::{debug, info}; // 从 v 库导出的 tracing 宏
```

## 优势 / Benefits

### 1. 简化依赖管理

插件的 `Cargo.toml` 不再需要单独引入 `tracing`：

**之前：**
```toml
[dependencies]
v = { path = "../../vgo-rust/v" }
tracing = "0.1"           # ← 需要单独引入
tracing-subscriber = "0.3"
```

**现在：**
```toml
[dependencies]
v = { path = "../../vgo-rust/v" }
# tracing 已经从 v 导出，无需单独引入
```

### 2. 版本一致性

确保所有插件使用相同版本的 tracing，避免版本冲突。

### 3. 更简洁的导入

```rust
// ✅ 简洁
use v::{debug, info, warn, error, trace};

// ❌ 冗长
use tracing::{debug, info, warn, error, trace};
```

## 可用的宏 / Available Macros

从 `v` 库导出的 tracing 宏：

| 宏 | 级别 | 用途 | 示例 |
|----|------|------|------|
| `trace!` | TRACE | 最详细的调试信息 | `trace!("Function called with arg: {}", arg)` |
| `debug!` | DEBUG | 调试信息 | `debug!("Processing message: {}", msg)` |
| `info!` | INFO | 一般信息 | `info!("✅ Plugin initialized")` |
| `warn!` | WARN | 警告信息 | `warn!("⚠️ Config missing, using default")` |
| `error!` | ERROR | 错误信息 | `error!("❌ Failed to process: {}", err)` |

## 使用示例 / Usage Examples

### 基本用法

```rust
use v::{debug, info, warn, error};

fn handle_message(content: &str) -> Result<()> {
    info!("💬 Received message: {}", content);
    
    if content.is_empty() {
        warn!("⚠️ Empty message received");
        return Ok(());
    }
    
    debug!("Processing message with length: {}", content.len());
    
    // ... 处理逻辑
    
    info!("✅ Message processed successfully");
    Ok(())
}
```

### 带字段的日志

```rust
use v::info;

info!(
    user_id = %uid,
    message_id = %msg_id,
    "Message sent"
);
```

### 条件日志

```rust
use v::debug;

if cfg!(debug_assertions) {
    debug!("Debug mode: detailed info here");
}
```

### 格式化输出

```rust
use v::{info, debug};

info!("User {} sent message to {}", from_uid, to_uid);
debug!("Message details: {:?}", message);
```

## 插件示例更新 / Plugin Example Update

### v-connect-im-plugin-example/src/main.rs

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use v::plugin::pdk::{json, Context, Plugin};
use v::{debug, info}; // ← 从 v 导出

impl Plugin for ExamplePlugin {
    fn new() -> Self {
        info!("🚀 Initializing Example Plugin");  // ← 使用 v::info
        Self {
            config: ExampleConfig::default(),
            message_count: 0,
        }
    }
    
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        let event_type = ctx.event_type();
        debug!("📨 Received event: {}", event_type);  // ← 使用 v::debug
        
        // ...
        
        Ok(())
    }
}
```

## 完整的日志级别说明 / Complete Log Level Description

### TRACE (最详细)

用于追踪程序执行流程的每一步：

```rust
use v::trace;

trace!("Entering function: handle_message");
trace!("Variable state: x={}, y={}", x, y);
trace!("Exiting function: handle_message");
```

### DEBUG (调试)

用于开发和调试时的详细信息：

```rust
use v::debug;

debug!("📨 Received event: {} (total: {})", event_type, count);
debug!("📤 Outgoing message: {}", content);
debug!("⏭️ Auto reply disabled, skipping");
```

### INFO (信息)

用于记录重要的程序运行信息：

```rust
use v::info;

info!("🚀 Initializing Example Plugin");
info!("💬 Message from {}: {}", from_uid, content);
info!("✅ Reply sent");
```

### WARN (警告)

用于记录潜在问题或异常情况：

```rust
use v::warn;

warn!("⚠️ Unknown event type: {}", event_type);
warn!("⚠️ Config missing, using default");
warn!("⚠️ Connection timeout, retrying...");
```

### ERROR (错误)

用于记录错误和失败：

```rust
use v::error;

error!("❌ Failed to process message: {}", err);
error!("❌ Database connection failed: {}", err);
error!("❌ Plugin initialization failed");
```

## 日志级别控制 / Log Level Control

### 在插件中设置

插件通过命令行参数控制日志级别：

```bash
# INFO 级别（默认）
cargo run -- --socket /path/to/socket.sock

# DEBUG 级别
cargo run -- --socket /path/to/socket.sock --debug

# TRACE 级别
cargo run -- --socket /path/to/socket.sock --log-level trace
```

### 在配置中设置

```toml
# v-connect-im/config/default.toml
[plugins]
debug = true
log_level = "debug"  # trace, debug, info, warn, error
```

## 最佳实践 / Best Practices

### 1. 使用合适的日志级别

```rust
// ✅ 正确
info!("Plugin started");           // 重要信息用 info
debug!("Processing step 1");       // 调试信息用 debug
error!("Failed: {}", err);         // 错误用 error

// ❌ 错误
debug!("Plugin started");          // 重要信息不应该用 debug
info!("Variable x = {}", x);       // 变量值应该用 debug
warn!("Failed: {}", err);          // 错误应该用 error
```

### 2. 添加上下文信息

```rust
// ✅ 好
info!("Message from {} to {}: {}", from_uid, to_uid, content);

// ❌ 差
info!("Message received");
```

### 3. 使用 emoji 增强可读性

```rust
info!("🚀 Plugin started");
info!("💬 Message received");
info!("✅ Processing complete");
warn!("⚠️ Warning occurred");
error!("❌ Error occurred");
debug!("📨 Event received");
```

### 4. 避免过度日志

```rust
// ❌ 过度日志
for item in items {
    debug!("Processing item: {:?}", item);  // 如果 items 很多会产生大量日志
}

// ✅ 适度日志
debug!("Processing {} items", items.len());
// 处理逻辑
debug!("Processed {} items successfully", count);
```

### 5. 使用结构化日志

```rust
// ✅ 结构化
info!(
    event = "message_received",
    from_uid = %from_uid,
    to_uid = %to_uid,
    content_length = content.len(),
    "Message received"
);

// ❌ 非结构化
info!("Message received from {} to {}, length: {}", 
    from_uid, to_uid, content.len());
```

## 迁移指南 / Migration Guide

### 对于现有插件

**步骤 1：** 移除 `tracing` 依赖

```diff
# Cargo.toml
[dependencies]
v = { path = "../../vgo-rust/v" }
- tracing = "0.1"
- tracing-subscriber = "0.3"
```

**步骤 2：** 更新导入

```diff
- use tracing::{debug, info, warn, error};
+ use v::{debug, info, warn, error};
```

**步骤 3：** 重新编译

```bash
cargo clean
cargo build
```

## 总结 / Summary

- ✅ tracing 宏现在从 `v` 库导出
- ✅ 插件无需单独引入 `tracing` 依赖
- ✅ 确保版本一致性
- ✅ 简化导入语句
- ✅ 支持所有 5 个日志级别：trace, debug, info, warn, error

现在插件开发更加简洁和统一！🎉
