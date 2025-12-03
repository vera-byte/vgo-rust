# 插件消息分发测试指南 / Plugin Message Dispatch Test Guide

## 修复内容 / Fixes

### 1. 添加 Priority 支持
- ✅ 在 `PluginHandler` trait 中添加 `priority()` 方法
- ✅ 在 `PluginWrapper` 中存储和返回 priority
- ✅ 在握手时发送 priority 到服务器

### 2. 修复事件格式
- ✅ 将事件格式从 `{"event": "...", "payload": {...}}` 改为 `{"event_type": "...", "payload": {...}}`
- ✅ 与插件客户端的事件循环格式匹配

## 测试步骤 / Test Steps

### 步骤 1：重新编译插件

```bash
cd /Users/mac/workspace/v-connect-im-plugin-example
cargo build --release
```

### 步骤 2：启动 v-connect-im

```bash
cd /Users/mac/workspace/vgo-rust/v-connect-im
cargo run
```

**期望日志：**
```
🔌 Unix Socket server starting on: /Users/mac/vp/sockets/runtime.sock
🛠️ Starting dev plugin example with cargo run
🤝 Plugin handshake: example v0.1.0 (priority: 1, capabilities: ["message", "room", "connection", "user"])
✅ Plugin example registered to connection pool
🚀 All plugins started
```

### 步骤 3：测试插件消息分发

```bash
curl -X POST http://localhost:8080/api/v1/plugins/test \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Hello from test!",
    "from_uid": "user123",
    "to_uid": "user456"
  }'
```

**期望响应：**
```json
{
  "status": "ok",
  "plugin_responses": [
    {
      "plugin_name": "example",
      "response": {
        "type": 1,
        "content": "我是AIExample, 收到您的消息: Hello from test!"
      }
    }
  ]
}
```

### 步骤 4：查看插件日志

插件应该输出：
```
2025-12-03T07:48:32.162173Z  INFO [plugin:v.plugin.example-0.1.0] handshake sent: {"capabilities":["message","room","connection","user"],"name":"v.plugin.example","priority":1,"version":"0.1.0"}
2025-12-03T07:48:32.162416Z  INFO [plugin:v.plugin.example-0.1.0] handshake ack: {"config":{},"status":"ok"}
2025-12-03T07:48:32.162450Z DEBUG [plugin:v.plugin.example-0.1.0] config applied from handshake
2025-12-03T07:48:45.123456Z DEBUG [plugin:v.plugin.example-0.1.0] event: message.incoming payload={"content":"Hello from test!","from_uid":"user123","to_uid":"user456","timestamp":1701590925}
2025-12-03T07:48:45.123789Z DEBUG [plugin:v.plugin.example-0.1.0] response sent: {"type":1,"content":"我是AIExample, 收到您的消息: Hello from test!"}
```

## 关键修改 / Key Changes

### 1. v/src/plugin/client.rs

```rust
pub trait PluginHandler {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String> { ... }
    fn priority(&self) -> i32 {  // ← 新增
        0
    }
    fn config(&mut self, _cfg: &Value) -> Result<()> { ... }
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value>;
}

// 握手时发送 priority
async fn send_handshake(&mut self, stream: &mut UnixStream) -> Result<()> {
    let info = serde_json::json!({
        "name": self.handler.name(),
        "version": self.handler.version(),
        "capabilities": self.handler.capabilities(),
        "priority": self.handler.priority(),  // ← 新增
    });
    // ...
}
```

### 2. v/src/plugin/pdk.rs

```rust
struct PluginWrapper<P: Plugin> {
    plugin: P,
    name: &'static str,
    version: &'static str,
    priority: i32,  // ← 新增
}

impl<P: Plugin> PluginHandler for PluginWrapper<P> {
    // ...
    fn priority(&self) -> i32 {  // ← 新增
        self.priority
    }
}
```

### 3. v-connect-im/src/plugins/runtime.rs

```rust
// 修复事件格式
let msg = serde_json::json!({
    "event_type": event_type,  // ← 改为 event_type
    "payload": payload
});
```

## 故障排查 / Troubleshooting

### 问题：插件没有收到消息

**检查 1：插件是否注册到连接池？**
```
✅ Plugin example registered to connection pool
```

**检查 2：握手是否包含 priority？**
```
handshake sent: {"capabilities":[...],"name":"...","priority":1,"version":"..."}
```

**检查 3：测试 API 路径是否正确？**
```bash
# 新路径
curl -X POST http://localhost:8080/api/v1/plugins/test

# 旧路径（已废弃）
# curl -X POST http://localhost:8080/api/v1/plugin/test_message
```

### 问题：事件格式错误

**插件期望格式：**
```json
{
  "event_type": "message.incoming",
  "payload": {
    "content": "...",
    "from_uid": "..."
  }
}
```

**不是：**
```json
{
  "event": "message.incoming",  // ← 错误
  "payload": {...}
}
```

### 问题：连接断开

**原因：** 插件在握手后立即返回，连接被关闭

**解决：** 确保插件进入事件循环 `listen_loop`

## 完整流程 / Complete Flow

```
1. v-connect-im 启动
   ↓
2. 创建 Unix Socket 服务器
   ↓
3. 启动插件进程（cargo run）
   ↓
4. 插件连接到 Socket
   ↓
5. 插件发送握手（包含 priority）
   ↓
6. 服务器保存插件信息到 PluginRuntime
   ↓
7. 服务器注册连接到 PluginConnectionPool
   ↓
8. 插件进入事件循环，等待事件
   ↓
9. 用户调用测试 API
   ↓
10. PluginConnectionPool.broadcast_message_event()
   ↓
11. 按优先级排序插件
   ↓
12. 发送事件到插件（event_type + payload）
   ↓
13. 插件处理事件，返回响应
   ↓
14. 服务器收集所有响应
   ↓
15. 返回给用户
```

## 下一步 / Next Steps

1. **集成到实际消息处理**
   - 在消息接收时调用 `broadcast_message_event()`
   - 支持消息拦截

2. **添加其他事件类型**
   - `room.join`
   - `room.leave`
   - `connection.open`
   - `connection.close`

3. **性能优化**
   - 并发发送事件到多个插件
   - 添加超时控制
   - 连接池健康检查

现在可以测试插件消息分发功能了！🎉
