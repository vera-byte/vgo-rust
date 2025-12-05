# 插件间通信功能文档 / Inter-Plugin Communication Documentation

## 概述 / Overview

v-connect-im 现在支持完整的插件间通信功能，包括：
- ✅ 插件 A 直接调用插件 B（RPC）
- ✅ 插件间点对点消息传递
- ✅ 插件广播消息
- ✅ 事件订阅/发布机制

v-connect-im now supports complete inter-plugin communication features, including:
- ✅ Plugin A directly calls Plugin B (RPC)
- ✅ Point-to-point messaging between plugins
- ✅ Plugin broadcast messaging
- ✅ Event subscription/publication mechanism

---

## 功能特性 / Features

### 1. 插件 RPC 调用 / Plugin RPC Call

插件 A 可以直接调用插件 B 的方法并获取返回值。
Plugin A can directly call Plugin B's method and get the return value.

#### API 接口 / API Endpoint

```http
POST /v1/plugins/inter-communication
Content-Type: application/json

{
  "from_plugin": "plugin_a",
  "to_plugin": "plugin_b",
  "method": "process_data",
  "params": {
    "data": "hello world",
    "options": {
      "format": "json"
    }
  }
}
```

#### 响应示例 / Response Example

```json
{
  "status": "ok",
  "response": {
    "result": "processed",
    "output": "HELLO WORLD"
  },
  "error": null
}
```

#### 代码示例 / Code Example

```rust
// 在插件连接池中调用 / Call in plugin connection pool
let response = pool.plugin_call(
    "plugin_a",
    "plugin_b",
    "process_data",
    &json!({"data": "hello"})
).await?;
```

#### 插件端处理 / Plugin-side Handling

插件 B 需要处理 `plugin.call.{method}` 事件：

```rust
// 插件 B 接收调用 / Plugin B receives call
match event_type {
    "plugin.call.process_data" => {
        let from_plugin = payload.get("from_plugin").unwrap();
        let params = payload.get("params").unwrap();
        
        // 处理请求 / Process request
        let result = process_data(params);
        
        // 返回响应 / Return response
        json!({
            "status": "ok",
            "result": result
        })
    }
    _ => {}
}
```

---

### 2. 点对点消息传递 / Point-to-Point Messaging

插件间可以发送点对点消息，无需等待响应。
Plugins can send point-to-point messages without waiting for response.

#### API 接口 / API Endpoint

```http
PUT /v1/plugins/inter-communication
Content-Type: application/json

{
  "from_plugin": "plugin_a",
  "to_plugin": "plugin_b",
  "message": {
    "type": "notification",
    "content": "Data has been updated",
    "timestamp": 1234567890
  }
}
```

#### 响应示例 / Response Example

```json
{
  "status": "ok",
  "delivered": true,
  "error": null
}
```

#### 代码示例 / Code Example

```rust
// 发送消息 / Send message
let delivered = pool.plugin_send_message(
    "plugin_a",
    "plugin_b",
    &json!({
        "type": "notification",
        "content": "hello"
    })
).await?;
```

#### 插件端处理 / Plugin-side Handling

插件 B 接收 `plugin.message` 事件：

```rust
match event_type {
    "plugin.message" => {
        let from_plugin = payload.get("from_plugin").unwrap();
        let message = payload.get("message").unwrap();
        let timestamp = payload.get("timestamp").unwrap();
        
        // 处理消息 / Handle message
        handle_message(from_plugin, message);
        
        // 可选：返回确认 / Optional: return acknowledgment
        json!({"status": "received"})
    }
    _ => {}
}
```

---

### 3. 插件广播 / Plugin Broadcast

插件可以向其他所有插件或特定能力的插件广播消息。
Plugin can broadcast message to all other plugins or plugins with specific capabilities.

#### API 接口 / API Endpoint

```http
PATCH /v1/plugins/inter-communication
Content-Type: application/json

{
  "from_plugin": "plugin_a",
  "message": {
    "event": "data_updated",
    "data_id": "123",
    "timestamp": 1234567890
  },
  "filter_capabilities": ["storage"]
}
```

#### 响应示例 / Response Example

```json
{
  "status": "ok",
  "response_count": 2,
  "responses": [
    {
      "plugin_name": "storage-sled",
      "response": {
        "status": "ok",
        "cached": true
      }
    },
    {
      "plugin_name": "storage-redis",
      "response": {
        "status": "ok",
        "synced": true
      }
    }
  ]
}
```

#### 代码示例 / Code Example

```rust
// 广播给所有插件 / Broadcast to all plugins
let responses = pool.plugin_broadcast(
    "plugin_a",
    &json!({"event": "data_updated"}),
    None
).await?;

// 只广播给支持 storage 能力的插件 / Broadcast only to plugins with storage capability
let responses = pool.plugin_broadcast(
    "plugin_a",
    &json!({"event": "data_updated"}),
    Some(vec!["storage".to_string()])
).await?;
```

#### 插件端处理 / Plugin-side Handling

插件接收 `plugin.broadcast` 事件：

```rust
match event_type {
    "plugin.broadcast" => {
        let from_plugin = payload.get("from_plugin").unwrap();
        let message = payload.get("message").unwrap();
        
        // 处理广播消息 / Handle broadcast message
        handle_broadcast(from_plugin, message);
        
        json!({"status": "ok"})
    }
    _ => {}
}
```

---

### 4. 事件订阅/发布 / Event Subscription/Publication

插件可以订阅特定事件，当其他插件发布该事件时自动接收通知。
Plugins can subscribe to specific events and automatically receive notifications when other plugins publish those events.

#### 事件订阅 / Event Subscription

```http
POST /v1/plugins/event-bus
Content-Type: application/json

{
  "subscriber": "plugin_a",
  "event_pattern": "user.login",
  "priority": 10
}
```

**支持通配符 / Supports Wildcards:**
- `user.login` - 精确匹配 / Exact match
- `user.*` - 匹配所有用户事件 / Match all user events
- `*` - 匹配所有事件 / Match all events

#### 事件发布 / Event Publication

```http
PUT /v1/plugins/event-bus
Content-Type: application/json

{
  "publisher": "plugin_b",
  "event_type": "user.login",
  "payload": {
    "user_id": "123",
    "timestamp": 1234567890,
    "ip": "192.168.1.1"
  }
}
```

#### 代码示例 / Code Example

```rust
use crate::plugins::event_bus::PluginEventBus;

// 创建事件总线 / Create event bus
let event_bus = Arc::new(PluginEventBus::new(pool.clone()));

// 订阅事件 / Subscribe to event
event_bus.subscribe("plugin_a", "user.*", 10).await?;

// 发布事件 / Publish event
let responses = event_bus.publish(
    "plugin_b",
    "user.login",
    &json!({
        "user_id": "123",
        "timestamp": 1234567890
    })
).await?;

// 取消订阅 / Unsubscribe
event_bus.unsubscribe("plugin_a", "user.*").await?;
```

#### 插件端处理 / Plugin-side Handling

订阅者插件接收 `event.published` 事件：

```rust
match event_type {
    "event.published" => {
        let publisher = payload.get("publisher").unwrap();
        let event_type = payload.get("event_type").unwrap();
        let event_payload = payload.get("payload").unwrap();
        
        // 处理事件 / Handle event
        handle_event(event_type, event_payload);
        
        json!({"status": "processed"})
    }
    _ => {}
}
```

---

## 使用场景 / Use Cases

### 场景 1: 数据同步 / Data Synchronization

存储插件 A 更新数据后，通知缓存插件 B 刷新缓存：

```rust
// 存储插件 A 更新数据后 / After storage plugin A updates data
pool.plugin_send_message(
    "storage-sled",
    "cache-redis",
    &json!({
        "action": "invalidate",
        "key": "user:123"
    })
).await?;
```

### 场景 2: 权限验证 / Permission Verification

消息插件调用权限插件验证用户权限：

```rust
// 调用权限插件 / Call permission plugin
let response = pool.plugin_call(
    "message-handler",
    "auth-plugin",
    "check_permission",
    &json!({
        "user_id": "123",
        "action": "send_message",
        "resource": "room:456"
    })
).await?;

if response.get("allowed").and_then(|v| v.as_bool()) == Some(true) {
    // 允许操作 / Allow operation
}
```

### 场景 3: 事件驱动架构 / Event-Driven Architecture

用户登录时，多个插件响应登录事件：

```rust
// 认证插件发布登录事件 / Auth plugin publishes login event
event_bus.publish(
    "auth-plugin",
    "user.login",
    &json!({
        "user_id": "123",
        "timestamp": 1234567890
    })
).await?;

// 多个插件订阅并响应 / Multiple plugins subscribe and respond:
// - 日志插件记录登录日志 / Logging plugin records login log
// - 统计插件更新在线用户数 / Statistics plugin updates online user count
// - 通知插件发送欢迎消息 / Notification plugin sends welcome message
```

---

## 最佳实践 / Best Practices

### 1. 错误处理 / Error Handling

```rust
match pool.plugin_call("a", "b", "method", &params).await {
    Ok(Some(response)) => {
        // 处理响应 / Handle response
    }
    Ok(None) => {
        // 目标插件未连接 / Target plugin not connected
        warn!("Target plugin not available");
    }
    Err(e) => {
        // 调用失败 / Call failed
        error!("Plugin call failed: {}", e);
    }
}
```

### 2. 超时处理 / Timeout Handling

```rust
use tokio::time::{timeout, Duration};

// 设置超时 / Set timeout
match timeout(
    Duration::from_secs(5),
    pool.plugin_call("a", "b", "method", &params)
).await {
    Ok(Ok(Some(response))) => {
        // 成功 / Success
    }
    Ok(Ok(None)) => {
        // 插件未连接 / Plugin not connected
    }
    Ok(Err(e)) => {
        // 调用失败 / Call failed
    }
    Err(_) => {
        // 超时 / Timeout
        warn!("Plugin call timeout");
    }
}
```

### 3. 能力检查 / Capability Check

在调用插件前检查其能力：

```rust
// 检查插件是否支持特定能力 / Check if plugin supports specific capability
if let Some(runtime) = manager.plugins.get("plugin_b") {
    let capabilities = runtime.capabilities();
    if capabilities.contains(&"rpc".to_string()) {
        // 插件支持 RPC / Plugin supports RPC
        pool.plugin_call("a", "b", "method", &params).await?;
    }
}
```

### 4. 事件优先级 / Event Priority

为重要的订阅者设置更高的优先级：

```rust
// 高优先级订阅者先接收事件 / High priority subscribers receive events first
event_bus.subscribe("critical-plugin", "user.*", 100).await?;
event_bus.subscribe("normal-plugin", "user.*", 50).await?;
event_bus.subscribe("low-priority-plugin", "user.*", 10).await?;
```

---

## 注意事项 / Notes

1. **循环依赖 / Circular Dependencies**: 避免插件间的循环调用，可能导致死锁
2. **性能影响 / Performance Impact**: 频繁的插件间通信会影响性能，建议使用异步和批处理
3. **错误传播 / Error Propagation**: 插件调用失败不应影响主流程，需要适当的错误处理
4. **版本兼容性 / Version Compatibility**: 插件间通信协议应保持向后兼容
5. **安全性 / Security**: 验证插件身份，防止恶意插件滥用通信功能

---

## 总结 / Summary

插件间通信功能为 v-connect-im 提供了强大的扩展能力：
- ✅ **RPC 调用**：同步调用其他插件的方法
- ✅ **消息传递**：异步发送消息给其他插件
- ✅ **广播机制**：一对多的消息分发
- ✅ **事件系统**：松耦合的事件驱动架构

这些功能使得插件可以相互协作，构建更复杂的业务逻辑。

Inter-plugin communication provides powerful extensibility for v-connect-im:
- ✅ **RPC Call**: Synchronously call methods of other plugins
- ✅ **Messaging**: Asynchronously send messages to other plugins
- ✅ **Broadcasting**: One-to-many message distribution
- ✅ **Event System**: Loosely coupled event-driven architecture

These features enable plugins to collaborate and build more complex business logic.
