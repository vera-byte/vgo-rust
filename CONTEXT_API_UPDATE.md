# Context API 更新 / Context API Update

## 问题 / Issue

插件示例中使用 `ctx.event_type()` 方法，但 `Context` 结构体中没有定义这个方法。

```rust
// 错误
let event_type = ctx.event_type();  // ❌ no method named `event_type`
```

## 解决方案 / Solution

在 `v/src/plugin/pdk.rs` 的 `Context` 实现中添加 `event_type()` 方法：

```rust
impl Context {
    /// 获取事件类型 / Get event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
}
```

## Context API 完整列表 / Complete Context API

### 1. 事件信息 / Event Information

```rust
/// 获取事件类型 / Get event type
pub fn event_type(&self) -> &str
```

**示例：**
```rust
let event_type = ctx.event_type();
match event_type {
    "message.incoming" => { /* ... */ },
    "room.join" => { /* ... */ },
    _ => { /* ... */ }
}
```

### 2. 获取载荷字段 / Get Payload Fields

#### 字符串字段 / String Field

```rust
/// 获取载荷中的字符串字段 / Get string field from payload
pub fn get_payload_str(&self, key: &str) -> Option<&str>
```

**示例：**
```rust
let content = ctx.get_payload_str("content").unwrap_or("");
let from_uid = ctx.get_payload_str("from_uid").unwrap_or("unknown");
```

#### 整数字段 / Integer Field

```rust
/// 获取载荷中的整数字段 / Get integer field from payload
pub fn get_payload_i64(&self, key: &str) -> Option<i64>
```

**示例：**
```rust
let timestamp = ctx.get_payload_i64("timestamp").unwrap_or(0);
let count = ctx.get_payload_i64("count").unwrap_or(0);
```

#### 布尔字段 / Boolean Field

```rust
/// 获取载荷中的布尔字段 / Get boolean field from payload
pub fn get_payload_bool(&self, key: &str) -> Option<bool>
```

**示例：**
```rust
let is_read = ctx.get_payload_bool("is_read").unwrap_or(false);
let enabled = ctx.get_payload_bool("enabled").unwrap_or(true);
```

#### 对象字段 / Object Field

```rust
/// 获取载荷中的对象字段 / Get object field from payload
pub fn get_payload_object(&self, key: &str) -> Option<&serde_json::Map<String, Value>>
```

**示例：**
```rust
if let Some(user) = ctx.get_payload_object("user") {
    let name = user.get("name").and_then(|v| v.as_str());
    let age = user.get("age").and_then(|v| v.as_i64());
}
```

#### 数组字段 / Array Field

```rust
/// 获取载荷中的数组字段 / Get array field from payload
pub fn get_payload_array(&self, key: &str) -> Option<&Vec<Value>>
```

**示例：**
```rust
if let Some(tags) = ctx.get_payload_array("tags") {
    for tag in tags {
        if let Some(tag_str) = tag.as_str() {
            println!("Tag: {}", tag_str);
        }
    }
}
```

### 3. 载荷解析 / Payload Parsing

```rust
/// 反序列化载荷为指定类型 / Deserialize payload to specified type
pub fn parse_payload<T: DeserializeOwned>(&self) -> Result<T>
```

**示例：**
```rust
#[derive(Deserialize)]
struct MessagePayload {
    content: String,
    from_uid: String,
    to_uid: String,
}

let payload: MessagePayload = ctx.parse_payload()?;
println!("Content: {}", payload.content);
```

### 4. 响应处理 / Response Handling

```rust
/// 设置响应 / Set response
pub fn reply(&mut self, response: Value) -> Result<()>
```

**示例：**
```rust
ctx.reply(json!({
    "status": "ok",
    "flow": "continue",
    "content": "处理成功"
}))?;
```

```rust
/// 获取响应（内部使用）/ Get response (internal use)
pub fn take_response(self) -> Value
```

## 使用示例 / Usage Examples

### 完整的消息处理示例

```rust
use v::plugin::pdk::{Context, Plugin, json};
use v::{debug, info};
use anyhow::Result;

impl Plugin for MyPlugin {
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // 1. 获取事件类型
        let event_type = ctx.event_type();
        debug!("📨 Event: {}", event_type);
        
        // 2. 根据事件类型处理
        match event_type {
            "message.incoming" => {
                // 3. 获取消息内容
                let content = ctx.get_payload_str("content").unwrap_or("");
                let from_uid = ctx.get_payload_str("from_uid").unwrap_or("unknown");
                
                info!("💬 Message from {}: {}", from_uid, content);
                
                // 4. 构建响应
                ctx.reply(json!({
                    "status": "ok",
                    "flow": "continue",
                    "content": format!("收到: {}", content)
                }))?;
            }
            "room.join" => {
                let room_id = ctx.get_payload_str("room_id").unwrap_or("");
                let uid = ctx.get_payload_str("uid").unwrap_or("");
                
                info!("🚪 User {} joined room {}", uid, room_id);
                
                ctx.reply(json!({
                    "status": "ok",
                    "flow": "continue"
                }))?;
            }
            _ => {
                debug!("⚠️ Unknown event: {}", event_type);
                ctx.reply(json!({
                    "status": "ignored"
                }))?;
            }
        }
        
        Ok(())
    }
}
```

### 使用结构化载荷

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct MessageEvent {
    content: String,
    from_uid: String,
    to_uid: String,
    timestamp: i64,
}

fn handle_message(ctx: &mut Context) -> Result<()> {
    // 方法 1: 逐个字段获取
    let content = ctx.get_payload_str("content").unwrap_or("");
    let from_uid = ctx.get_payload_str("from_uid").unwrap_or("");
    
    // 方法 2: 整体解析
    let event: MessageEvent = ctx.parse_payload()?;
    
    info!("Message: {} from {} at {}", 
        event.content, event.from_uid, event.timestamp);
    
    Ok(())
}
```

### 条件响应

```rust
fn handle_message(ctx: &mut Context) -> Result<()> {
    let content = ctx.get_payload_str("content").unwrap_or("");
    
    // 根据内容决定是否拦截
    if content.contains("spam") {
        ctx.reply(json!({
            "status": "blocked",
            "flow": "stop",  // 停止传播
            "reason": "Spam detected"
        }))?;
        return Ok(());
    }
    
    // 正常处理
    ctx.reply(json!({
        "status": "ok",
        "flow": "continue"
    }))?;
    
    Ok(())
}
```

## 字段访问对比 / Field Access Comparison

### 直接访问（不推荐）

```rust
// ❌ 不推荐：直接访问字段
let event_type = &ctx.event_type;
let payload = &ctx.payload;
```

**问题：**
- 暴露内部实现
- 无法进行验证
- 难以维护

### 方法访问（推荐）

```rust
// ✅ 推荐：使用方法访问
let event_type = ctx.event_type();
let content = ctx.get_payload_str("content");
```

**优势：**
- 封装内部实现
- 可以添加验证逻辑
- 易于维护和扩展

## API 设计原则 / API Design Principles

### 1. 类型安全

```rust
// ✅ 类型安全
let count: Option<i64> = ctx.get_payload_i64("count");

// ❌ 不安全
let count = ctx.payload.get("count").unwrap();  // 可能 panic
```

### 2. 提供默认值

```rust
// ✅ 提供默认值
let content = ctx.get_payload_str("content").unwrap_or("");

// ❌ 直接 unwrap
let content = ctx.get_payload_str("content").unwrap();  // 可能 panic
```

### 3. 链式调用

```rust
// ✅ 链式调用
ctx.reply(json!({
    "status": "ok"
}))?;

// 支持多次调用
ctx.reply(json!({"step": 1}))?;
ctx.reply(json!({"step": 2}))?;  // 会覆盖前一个
```

## 最佳实践 / Best Practices

### 1. 总是检查 Option

```rust
// ✅ 正确
if let Some(content) = ctx.get_payload_str("content") {
    process(content);
} else {
    warn!("Content missing");
}

// 或使用 unwrap_or
let content = ctx.get_payload_str("content").unwrap_or("");
```

### 2. 使用 match 处理事件

```rust
// ✅ 清晰
match ctx.event_type() {
    "message.incoming" => handle_message(ctx)?,
    "room.join" => handle_room_join(ctx)?,
    _ => handle_unknown(ctx)?,
}
```

### 3. 明确指定 flow

```rust
// ✅ 明确
ctx.reply(json!({
    "status": "ok",
    "flow": "continue"  // 明确指定
}))?;

// ❌ 不明确
ctx.reply(json!({
    "status": "ok"
    // flow 未指定
}))?;
```

## 总结 / Summary

- ✅ 添加了 `event_type()` 方法
- ✅ Context API 现在更加完整
- ✅ 支持多种类型的字段访问
- ✅ 提供类型安全的 API
- ✅ 遵循 Rust 最佳实践

现在插件可以正常使用 `ctx.event_type()` 方法了！🎉
