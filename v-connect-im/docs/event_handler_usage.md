# 事件处理器使用指南 / Event Handler Usage Guide

## 快速开始 / Quick Start

### 1. 导入必要的类型 / Import Required Types

```rust
use v_connect_im::plugins::event_handler::{Context, StorageEventHandler};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
```

### 2. 定义你的插件 / Define Your Plugin

```rust
pub struct MyStoragePlugin {
    // 你的插件字段 / Your plugin fields
}

impl MyStoragePlugin {
    pub fn new() -> Self {
        Self {
            // 初始化字段 / Initialize fields
        }
    }
}
```

### 3. 实现事件处理器 trait / Implement Event Handler Trait

```rust
#[async_trait]
impl StorageEventHandler for MyStoragePlugin {
    /// 处理消息保存 / Handle message save
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        // 1. 从上下文提取数据 / Extract data from context
        let message_id: String = ctx.get("message_id")?;
        let from_uid: String = ctx.get("from_uid")?;
        let to_uid: String = ctx.get("to_uid")?;
        
        // 2. 执行业务逻辑 / Execute business logic
        println!("保存消息: {} -> {}", from_uid, to_uid);
        
        // 3. 返回响应 / Return response
        ctx.reply(json!({
            "status": "ok",
            "message_id": message_id
        }))?;
        
        Ok(())
    }
    
    /// 处理离线消息保存 / Handle offline message save
    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()> {
        let to_uid: String = ctx.get("to_uid")?;
        
        // 你的逻辑 / Your logic
        
        ctx.reply(json!({
            "status": "ok",
            "to_uid": to_uid
        }))?;
        
        Ok(())
    }
    
    // 只实现你需要的方法，其他方法会使用默认实现
    // Only implement methods you need, others will use default implementation
}
```

### 4. 使用插件 / Use Plugin

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 创建插件实例 / Create plugin instance
    let plugin = MyStoragePlugin::new();
    
    // 创建事件上下文 / Create event context
    let mut ctx = Context::new(
        "storage.message.save",
        json!({
            "message_id": "msg_001",
            "from_uid": "alice",
            "to_uid": "bob",
            "content": {"text": "Hello!"}
        })
    );
    
    // 分发事件 / Dispatch event
    plugin.dispatch(&mut ctx).await?;
    
    // 获取响应 / Get response
    if let Some(response) = ctx.response() {
        println!("响应: {}", response);
    }
    
    Ok(())
}
```

## Context API 说明 / Context API Reference

### 获取数据 / Get Data

```rust
// 获取字符串 / Get string
let user_id: String = ctx.get("user_id")?;

// 获取数字 / Get number
let count: i64 = ctx.get("count")?;

// 获取布尔值 / Get boolean
let is_active: bool = ctx.get("is_active")?;

// 获取复杂对象 / Get complex object
#[derive(Deserialize)]
struct UserInfo {
    name: String,
    age: u32,
}
let user: UserInfo = ctx.get("user")?;

// 直接访问载荷 / Direct access to payload
let payload = ctx.payload();
```

### 设置响应 / Set Response

```rust
// 成功响应 / Success response
ctx.reply(json!({
    "status": "ok",
    "data": {...}
}))?;

// 错误响应 / Error response
ctx.reply(json!({
    "status": "error",
    "message": "Something went wrong"
}))?;
```

### 获取事件类型 / Get Event Type

```rust
let event_type = ctx.event_type();
println!("事件类型: {}", event_type);
```

## 可用的事件处理器 / Available Event Handlers

### StorageEventHandler

存储相关事件处理器 / Storage-related event handler

**事件方法 / Event Methods:**

| 方法 / Method | 事件类型 / Event Type | 说明 / Description |
|--------------|---------------------|-------------------|
| `on_message_save` | `storage.message.save` | 保存消息 / Save message |
| `on_offline_save` | `storage.offline.save` | 保存离线消息 / Save offline message |
| `on_offline_pull` | `storage.offline.pull` | 拉取离线消息 / Pull offline messages |
| `on_offline_ack` | `storage.offline.ack` | 确认离线消息 / Acknowledge offline messages |
| `on_offline_count` | `storage.offline.count` | 统计离线消息数 / Count offline messages |
| `on_room_add_member` | `storage.room.add_member` | 添加房间成员 / Add room member |
| `on_room_remove_member` | `storage.room.remove_member` | 移除房间成员 / Remove room member |
| `on_room_list_members` | `storage.room.list_members` | 列出房间成员 / List room members |
| `on_room_list` | `storage.room.list` | 列出房间 / List rooms |
| `on_read_record` | `storage.read.record` | 记录已读 / Record read status |
| `on_message_history` | `storage.message.history` | 获取消息历史 / Get message history |
| `on_stats` | `storage.stats` | 获取统计信息 / Get statistics |

### AuthEventHandler

认证相关事件处理器 / Authentication-related event handler

**事件方法 / Event Methods:**

| 方法 / Method | 事件类型 / Event Type | 说明 / Description |
|--------------|---------------------|-------------------|
| `on_login` | `auth.login` | 用户登录 / User login |
| `on_logout` | `auth.logout` | 用户登出 / User logout |
| `on_kick_out` | `auth.kick_out` | 用户被踢出 / User kicked out |
| `on_renew_timeout` | `auth.renew_timeout` | Token 续期 / Token renewal |
| `on_replaced` | `auth.replaced` | Token 被替换 / Token replaced |
| `on_banned` | `auth.banned` | 用户被封禁 / User banned |

## 完整示例 / Complete Example

```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// 导入事件处理器 / Import event handler
use v_connect_im::plugins::event_handler::{Context, StorageEventHandler};

/// 内存存储插件 / In-memory storage plugin
pub struct MemoryStoragePlugin {
    messages: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl MemoryStoragePlugin {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StorageEventHandler for MemoryStoragePlugin {
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        // 提取数据 / Extract data
        let message_id: String = ctx.get("message_id")?;
        
        // 保存到内存 / Save to memory
        let mut messages = self.messages.write().await;
        messages.insert(message_id.clone(), ctx.payload().clone());
        
        // 返回响应 / Return response
        ctx.reply(json!({
            "status": "ok",
            "message_id": message_id,
            "saved_at": chrono::Utc::now().timestamp()
        }))?;
        
        Ok(())
    }
    
    async fn on_stats(&self, ctx: &mut Context) -> Result<()> {
        let messages = self.messages.read().await;
        
        ctx.reply(json!({
            "status": "ok",
            "stats": {
                "total_messages": messages.len()
            }
        }))?;
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let plugin = MemoryStoragePlugin::new();
    
    // 保存消息 / Save message
    let mut ctx = Context::new(
        "storage.message.save",
        json!({
            "message_id": "msg_001",
            "from_uid": "alice",
            "to_uid": "bob",
            "content": {"text": "Hello!"}
        })
    );
    
    plugin.dispatch(&mut ctx).await?;
    println!("保存响应: {}", ctx.response().unwrap());
    
    // 查询统计 / Query stats
    let mut ctx = Context::new("storage.stats", json!({}));
    plugin.dispatch(&mut ctx).await?;
    println!("统计响应: {}", ctx.response().unwrap());
    
    Ok(())
}
```

## 测试示例 / Testing Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_save() {
        let plugin = MemoryStoragePlugin::new();
        
        let mut ctx = Context::new(
            "storage.message.save",
            json!({
                "message_id": "test_001",
                "from_uid": "alice",
                "to_uid": "bob"
            })
        );
        
        plugin.dispatch(&mut ctx).await.unwrap();
        
        let response = ctx.response().unwrap();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["message_id"], "test_001");
    }
    
    #[tokio::test]
    async fn test_unknown_event() {
        let plugin = MemoryStoragePlugin::new();
        
        let mut ctx = Context::new("storage.unknown", json!({}));
        plugin.dispatch(&mut ctx).await.unwrap();
        
        let response = ctx.response().unwrap();
        assert_eq!(response["status"], "error");
    }
}
```

## 最佳实践 / Best Practices

### 1. 错误处理 / Error Handling

```rust
async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
    // 使用 ? 运算符传播错误 / Use ? operator to propagate errors
    let message_id: String = ctx.get("message_id")?;
    
    // 处理可能的错误 / Handle possible errors
    match self.save_to_db(&message_id).await {
        Ok(_) => {
            ctx.reply(json!({"status": "ok"}))?;
        }
        Err(e) => {
            ctx.reply(json!({
                "status": "error",
                "message": e.to_string()
            }))?;
        }
    }
    
    Ok(())
}
```

### 2. 日志记录 / Logging

```rust
use tracing::{info, error, debug};

async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
    let message_id: String = ctx.get("message_id")?;
    
    debug!("开始保存消息 / Starting to save message: {}", message_id);
    
    match self.save_to_db(&message_id).await {
        Ok(_) => {
            info!("消息保存成功 / Message saved successfully: {}", message_id);
            ctx.reply(json!({"status": "ok"}))?;
        }
        Err(e) => {
            error!("消息保存失败 / Failed to save message: {}", e);
            ctx.reply(json!({"status": "error"}))?;
        }
    }
    
    Ok(())
}
```

### 3. 数据验证 / Data Validation

```rust
async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
    // 验证必需字段 / Validate required fields
    let message_id: String = ctx.get("message_id")
        .map_err(|_| anyhow::anyhow!("message_id is required"))?;
    
    let from_uid: String = ctx.get("from_uid")
        .map_err(|_| anyhow::anyhow!("from_uid is required"))?;
    
    // 验证数据格式 / Validate data format
    if message_id.is_empty() {
        ctx.reply(json!({
            "status": "error",
            "message": "message_id cannot be empty"
        }))?;
        return Ok(());
    }
    
    // 继续处理 / Continue processing
    // ...
    
    Ok(())
}
```

## 常见问题 / FAQ

### Q: 如何添加新的事件类型？

A: 在对应的 trait 中添加新方法，并在 `dispatch` 方法中添加匹配分支。

### Q: 如何处理异步操作？

A: 所有方法都是 `async fn`，可以直接使用 `.await`。

### Q: 如何共享状态？

A: 使用 `Arc<RwLock<T>>` 或 `Arc<Mutex<T>>` 包装共享状态。

### Q: 默认实现会做什么？

A: 默认实现会返回 "Not implemented" 错误响应。

## 参考资料 / References

- [完整示例代码](../examples/storage_plugin_example.rs)
- [事件处理器定义](../src/plugins/event_handler.rs)
- [优化方案文档](./event_handler_optimization.md)
