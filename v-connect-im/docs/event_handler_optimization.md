# 事件处理器优化方案 / Event Handler Optimization

## 概述 / Overview

本文档说明如何使用基于 trait 的事件处理器模式来优化插件中的事件监听处理逻辑。
This document explains how to optimize event listener handling in plugins using trait-based event handler pattern.

## 优化前 / Before Optimization

### 问题 / Problems

使用大量 `match` 分支处理不同的事件类型：
Using massive `match` branches to handle different event types:

```rust
fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    let event_type = ctx.event_type();
    debug!("📨 收到存储事件 / Received storage event: {}", event_type);

    // 根据事件类型分发到对应的处理函数 / Dispatch to corresponding handler
    match event_type {
        "storage.message.save" => self.handle_message_save(ctx)?,
        "storage.offline.save" => self.handle_offline_save(ctx)?,
        "storage.offline.pull" => self.handle_offline_pull(ctx)?,
        "storage.offline.ack" => self.handle_offline_ack(ctx)?,
        "storage.offline.count" => self.handle_offline_count(ctx)?,
        "storage.room.add_member" => self.handle_room_add_member(ctx)?,
        "storage.room.remove_member" => self.handle_room_remove_member(ctx)?,
        "storage.room.list_members" => self.handle_room_list_members(ctx)?,
        "storage.room.list" => self.handle_room_list(ctx)?,
        "storage.read.record" => self.handle_read_record(ctx)?,
        "storage.message.history" => self.handle_message_history(ctx)?,
        "storage.stats" => self.handle_stats(ctx)?,
        _ => {
            warn!("⚠️  未知的存储事件类型 / Unknown storage event type: {}", event_type);
            ctx.reply(json!({
                "status": "error",
                "message": format!("Unknown event type: {}", event_type)
            }))?;
        }
    }

    Ok(())
}
```

**缺点 / Drawbacks:**

1. **代码冗长** / Code verbosity - 大量重复的 match 分支
2. **难以维护** / Hard to maintain - 添加新事件需要修改多处
3. **缺乏类型安全** / Lack of type safety - 字符串匹配容易出错
4. **不易测试** / Hard to test - 难以单独测试每个事件处理器
5. **耦合度高** / High coupling - 所有事件处理逻辑耦合在一起

## 优化后 / After Optimization

### 解决方案 / Solution

使用 trait 定义事件处理器接口，每个事件类型对应一个方法：
Use trait to define event handler interface, each event type corresponds to a method:

```rust
use async_trait::async_trait;
use anyhow::Result;

/// 存储事件处理器 trait / Storage event handler trait
#[async_trait]
pub trait StorageEventHandler: Send + Sync {
    /// 处理消息保存事件 / Handle message save event
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        // 默认实现 / Default implementation
        ctx.reply(json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理离线消息保存事件 / Handle offline message save event
    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    // ... 其他事件方法 / Other event methods

    /// 分发事件到对应的处理方法 / Dispatch event to corresponding handler
    async fn dispatch(&self, ctx: &mut Context) -> Result<()> {
        use tracing::{debug, warn};

        let event_type = ctx.event_type();
        debug!("📨 收到存储事件 / Received storage event: {}", event_type);

        match event_type {
            "storage.message.save" => self.on_message_save(ctx).await?,
            "storage.offline.save" => self.on_offline_save(ctx).await?,
            // ... 其他事件 / Other events
            _ => {
                warn!("⚠️  未知的存储事件类型 / Unknown storage event type: {}", event_type);
                ctx.reply(json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
            }
        }

        Ok(())
    }
}
```

### 插件实现 / Plugin Implementation

```rust
pub struct MyStoragePlugin {
    // 插件字段 / Plugin fields
}

#[async_trait]
impl StorageEventHandler for MyStoragePlugin {
    /// 只需实现需要的事件处理方法 / Only implement needed event handlers
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        // 从上下文中提取数据 / Extract data from context
        let message_id: String = ctx.get("message_id")?;
        let from_uid: String = ctx.get("from_uid")?;
        let to_uid: String = ctx.get("to_uid")?;

        // 业务逻辑 / Business logic
        // ...

        // 返回响应 / Return response
        ctx.reply(json!({
            "status": "ok",
            "message_id": message_id
        }))?;

        Ok(())
    }

    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()> {
        // 实现离线消息保存 / Implement offline message save
        // ...
        Ok(())
    }

    // 其他方法使用默认实现 / Other methods use default implementation
}
```

### 使用方式 / Usage

```rust
// 创建插件实例 / Create plugin instance
let plugin = MyStoragePlugin::new();

// 创建事件上下文 / Create event context
let mut ctx = Context::new(
    "storage.message.save",
    json!({
        "message_id": "msg_001",
        "from_uid": "user_alice",
        "to_uid": "user_bob",
        "content": {"text": "Hello!"}
    })
);

// 分发事件 / Dispatch event
plugin.dispatch(&mut ctx).await?;

// 获取响应 / Get response
if let Some(response) = ctx.response() {
    println!("Response: {}", response);
}
```

## 优势 / Advantages

### 1. 清晰的接口定义 / Clear Interface Definition

- ✅ 每个事件类型都有明确的方法签名
- ✅ 使用 Rust 的类型系统保证类型安全
- ✅ IDE 可以提供更好的代码补全和提示

### 2. 更好的可维护性 / Better Maintainability

- ✅ 添加新事件只需在 trait 中添加新方法
- ✅ 实现类只需实现需要的方法
- ✅ 默认实现提供了合理的降级行为

### 3. 易于测试 / Easy to Test

```rust
#[tokio::test]
async fn test_message_save() {
    let plugin = MyStoragePlugin::new();
    let mut ctx = Context::new(
        "storage.message.save",
        json!({"message_id": "test_001"})
    );

    plugin.on_message_save(&mut ctx).await.unwrap();

    assert_eq!(ctx.response().unwrap()["status"], "ok");
}
```

### 4. 支持多种事件处理器 / Support Multiple Event Handlers

可以为不同类型的事件定义不同的 trait：
Can define different traits for different types of events:

```rust
#[async_trait]
pub trait StorageEventHandler: Send + Sync {
    // 存储相关事件 / Storage-related events
}

#[async_trait]
pub trait AuthEventHandler: Send + Sync {
    // 认证相关事件 / Auth-related events
    async fn on_login(&self, ctx: &mut Context) -> Result<()>;
    async fn on_logout(&self, ctx: &mut Context) -> Result<()>;
    async fn on_kick_out(&self, ctx: &mut Context) -> Result<()>;
}

#[async_trait]
pub trait MessageEventHandler: Send + Sync {
    // 消息相关事件 / Message-related events
}
```

### 5. 更好的代码组织 / Better Code Organization

```
src/plugins/
├── event_handler.rs          # 事件处理器 trait 定义
├── storage_handler.rs         # 存储事件处理器实现
├── auth_handler.rs            # 认证事件处理器实现
└── message_handler.rs         # 消息事件处理器实现
```

## 迁移指南 / Migration Guide

### 步骤 1: 定义 trait / Step 1: Define Trait

在 `src/plugins/event_handler.rs` 中定义事件处理器 trait。
Define event handler trait in `src/plugins/event_handler.rs`.

### 步骤 2: 实现 trait / Step 2: Implement Trait

让你的插件实现对应的 trait：
Make your plugin implement the corresponding trait:

```rust
#[async_trait]
impl StorageEventHandler for YourPlugin {
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        // 将原来的 handle_message_save 逻辑移到这里
        // Move original handle_message_save logic here
    }

    // ... 其他方法 / Other methods
}
```

### 步骤 3: 更新调用代码 / Step 3: Update Calling Code

将原来的 `receive` 方法替换为 `dispatch`：
Replace original `receive` method with `dispatch`:

```rust
// 之前 / Before
fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    match ctx.event_type() {
        // ...
    }
}

// 之后 / After
async fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    self.dispatch(ctx).await
}
```

## 示例代码 / Example Code

完整的示例代码请参考：
For complete example code, please refer to:

- `src/plugins/event_handler.rs` - Trait 定义 / Trait definitions
- `examples/storage_plugin_example.rs` - 使用示例 / Usage example

## 运行示例 / Run Example

```bash
# 运行存储插件示例 / Run storage plugin example
cargo run --example storage_plugin_example
```

## 总结 / Summary

使用基于 trait 的事件处理器模式可以显著提高代码的可维护性、可测试性和可扩展性。这是一种更加符合 Rust 惯用法的设计模式。

Using trait-based event handler pattern can significantly improve code maintainability, testability, and extensibility. This is a more idiomatic Rust design pattern.

### 关键要点 / Key Points

1. **使用 trait 定义接口** / Use trait to define interface
2. **提供默认实现** / Provide default implementation
3. **只实现需要的方法** / Only implement needed methods
4. **利用类型系统保证安全** / Leverage type system for safety
5. **便于单元测试** / Easy to unit test

### 参考资料 / References

- [Rust Async Trait](https://docs.rs/async-trait/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [SaToken Listener Example](https://github.com/dromara/sa-token) (参考的设计模式)
