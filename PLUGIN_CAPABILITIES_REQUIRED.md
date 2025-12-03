# 插件能力声明改为必传 / Plugin Capabilities Declaration Now Required

## 修改内容 / Changes

### 1. PluginHandler Trait 修改

**之前（可选）：**
```rust
pub trait PluginHandler {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String> {
        // 默认实现
        vec!["message".into(), "room".into(), "connection".into(), "user".into()]
    }
    // ...
}
```

**现在（必传）：**
```rust
pub trait PluginHandler {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String>;  // ← 必须实现
    fn priority(&self) -> i32 {
        0  // 优先级仍然可选
    }
    // ...
}
```

### 2. PluginWrapper 实现

```rust
impl<P: Plugin> PluginHandler for PluginWrapper<P> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        // 默认支持所有能力 / Default to all capabilities
        vec![
            "message".into(),
            "room".into(),
            "connection".into(),
            "user".into(),
        ]
    }

    fn priority(&self) -> i32 {
        self.priority
    }
    
    // ...
}
```

## 能力类型说明 / Capability Types

### 支持的能力 / Supported Capabilities

| 能力 | 说明 | 事件类型 | 状态 |
|------|------|----------|------|
| `message` | 消息事件 | `message.incoming`, `message.outgoing` | ✅ 已实现 |
| `room` | 房间事件 | `room.join`, `room.leave`, `room.create` | ⏳ 待实现 |
| `connection` | 连接事件 | `connection.open`, `connection.close` | ⏳ 待实现 |
| `user` | 用户事件 | `user.online`, `user.offline`, `user.update` | ⏳ 待实现 |

### 能力声明示例 / Capability Declaration Examples

**示例 1：只处理消息**
```rust
fn capabilities(&self) -> Vec<String> {
    vec!["message".into()]
}
```

**示例 2：处理消息和房间**
```rust
fn capabilities(&self) -> Vec<String> {
    vec!["message".into(), "room".into()]
}
```

**示例 3：处理所有事件**
```rust
fn capabilities(&self) -> Vec<String> {
    vec![
        "message".into(),
        "room".into(),
        "connection".into(),
        "user".into(),
    ]
}
```

## 插件开发指南 / Plugin Development Guide

### 使用 PDK（推荐）

使用 `v::plugin::pdk` 开发的插件会**自动获得所有能力**：

```rust
use v::plugin::pdk::{Plugin, Context};

struct MyPlugin {
    config: MyConfig,
}

impl Plugin for MyPlugin {
    type Config = MyConfig;
    
    fn new() -> Self {
        Self {
            config: MyConfig::default(),
        }
    }
    
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // 处理事件
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    v::plugin::pdk::run_server::<MyPlugin>(
        "v.plugin.myplugin",
        "1.0.0",
        100  // priority
    ).await
}
```

**自动获得的能力：**
- ✅ `message`
- ✅ `room`
- ✅ `connection`
- ✅ `user`

### 直接使用 PluginHandler（高级）

如果直接实现 `PluginHandler` trait，**必须声明 capabilities**：

```rust
use v::plugin::client::{PluginHandler, PluginClient};

struct CustomPlugin;

impl PluginHandler for CustomPlugin {
    fn name(&self) -> &'static str {
        "custom.plugin"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    
    fn capabilities(&self) -> Vec<String> {
        // ⚠️ 必须实现
        vec!["message".into()]
    }
    
    fn priority(&self) -> i32 {
        50
    }
    
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        // 处理事件
        Ok(json!({"status": "ok"}))
    }
}
```

## 握手协议 / Handshake Protocol

### 插件发送

```json
{
  "name": "v.plugin.example",
  "version": "0.1.0",
  "capabilities": ["message", "room", "connection", "user"],
  "priority": 1
}
```

### 服务器响应

```json
{
  "status": "ok",
  "config": {}
}
```

## 事件路由 / Event Routing

服务器会根据插件声明的能力来路由事件：

```rust
// v-connect-im/src/plugins/runtime.rs
pub async fn broadcast_message_event(&self, message: &Value) -> Result<Vec<(String, Value)>> {
    // 获取所有插件并按优先级排序
    let mut plugins: Vec<_> = self.manager.plugins.iter()
        .map(|entry| {
            let runtime = entry.value();
            (entry.key().clone(), runtime.priority(), runtime.capabilities())
        })
        .collect();
    
    plugins.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (name, _priority, capabilities) in plugins {
        // ✅ 检查插件是否支持 message 事件
        if !capabilities.iter().any(|cap| cap == "message") {
            continue;  // 跳过不支持的插件
        }
        
        // 发送事件到插件
        // ...
    }
}
```

## 最佳实践 / Best Practices

### 1. 只声明需要的能力

❌ **不推荐：** 声明所有能力但只处理部分
```rust
fn capabilities(&self) -> Vec<String> {
    vec!["message".into(), "room".into(), "connection".into(), "user".into()]
}

fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
    match event_type {
        "message.incoming" => { /* 处理 */ },
        _ => Ok(json!({"status": "ignored"}))  // ← 浪费资源
    }
}
```

✅ **推荐：** 只声明实际处理的能力
```rust
fn capabilities(&self) -> Vec<String> {
    vec!["message".into()]  // 只声明 message
}

fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
    match event_type {
        "message.incoming" => { /* 处理 */ },
        _ => Ok(json!({"status": "ok"}))
    }
}
```

### 2. 使用常量定义能力

```rust
const CAPABILITIES: &[&str] = &["message", "room"];

impl PluginHandler for MyPlugin {
    fn capabilities(&self) -> Vec<String> {
        CAPABILITIES.iter().map(|s| s.to_string()).collect()
    }
}
```

### 3. 动态能力（高级）

根据配置动态返回能力：

```rust
struct ConfigurablePlugin {
    config: PluginConfig,
}

impl PluginHandler for ConfigurablePlugin {
    fn capabilities(&self) -> Vec<String> {
        let mut caps = vec!["message".into()];
        
        if self.config.enable_room_events {
            caps.push("room".into());
        }
        
        if self.config.enable_user_events {
            caps.push("user".into());
        }
        
        caps
    }
}
```

## 迁移指南 / Migration Guide

### 对于使用 PDK 的插件

**无需修改！** PDK 会自动提供所有能力。

### 对于直接实现 PluginHandler 的插件

需要添加 `capabilities()` 方法：

```diff
impl PluginHandler for MyPlugin {
    fn name(&self) -> &'static str { "my.plugin" }
    fn version(&self) -> &'static str { "1.0.0" }
+   fn capabilities(&self) -> Vec<String> {
+       vec!["message".into()]
+   }
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        // ...
    }
}
```

## 验证 / Verification

### 1. 检查握手日志

```
🤝 Plugin handshake: example v0.1.0 (priority: 1, capabilities: ["message", "room", "connection", "user"])
```

### 2. 测试事件分发

```bash
curl -X POST http://localhost:8080/api/v1/plugins/test \
  -H "Content-Type: application/json" \
  -d '{"content": "test"}'
```

### 3. 查看插件日志

```
DEBUG [plugin:v.plugin.example-0.1.0] event: message.incoming payload={...}
```

## 总结 / Summary

- ✅ `capabilities` 现在是必须实现的方法
- ✅ 使用 PDK 的插件自动获得所有能力
- ✅ 服务器根据能力路由事件，提高性能
- ✅ 插件可以只声明需要的能力
- ✅ 支持动态能力配置

现在插件必须明确声明支持的能力，这样可以：
1. 提高事件路由效率
2. 避免不必要的事件分发
3. 让插件功能更加明确
4. 便于调试和监控

🎉 修改完成！
