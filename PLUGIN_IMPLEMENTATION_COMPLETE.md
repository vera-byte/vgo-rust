# 插件消息分发系统实现完成 / Plugin Message Dispatch System Implementation Complete

## 已实现功能 / Implemented Features

### ✅ 1. 插件能力管理 / Plugin Capability Management
- 在握手时保存插件的 `capabilities` 和 `priority`
- 支持根据能力路由事件

### ✅ 2. 插件连接池 / Plugin Connection Pool
- `PluginConnectionPool` 管理所有插件连接
- 支持注册/注销插件连接
- 维护 socket 连接用于事件分发

### ✅ 3. 事件分发 / Event Dispatch
- `send_event()` - 向单个插件发送事件
- `broadcast_message_event()` - 广播消息事件到所有支持的插件
- 按优先级排序插件
- 支持消息拦截（flow: stop）

### ✅ 4. 消息拦截 / Message Interception
- 插件可以返回 `{"flow": "stop"}` 停止消息传播
- 高优先级插件先处理

### ✅ 5. 测试 API / Test API
- `/api/v1/plugin/test_message` - 测试插件消息分发

## 使用方法 / Usage

### 1. 启动 v-connect-im

```bash
cd /Users/mac/workspace/vgo-rust/v-connect-im
cargo run
```

**期望日志：**
```
🔌 Plugin runtime manager initialized
🛠️ Registered dev plugin: example from /Users/mac/workspace/v-connect-im-plugin-example
🔌 Unix Socket server starting on: /Users/mac/vp/sockets/runtime.sock
🛠️ Starting dev plugin example with cargo run
🤝 Plugin handshake: example v0.1.0 (priority: 1, capabilities: ["message", "room", "connection", "user"])
✅ Plugin example registered to connection pool
🚀 All plugins started
```

### 2. 测试插件消息分发

```bash
curl -X POST http://localhost:8080/api/v1/plugin/test_message \
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

### 3. 插件日志

插件应该输出：
```
📨 Received event: message.incoming
📝 Message content: Hello from test!
✅ Response sent
```

## 代码结构 / Code Structure

```
v-connect-im/
├── src/
│   ├── api/v1/plugin/
│   │   ├── mod.rs          # 插件 API 模块
│   │   └── test.rs         # 测试接口
│   ├── plugins/runtime.rs
│   │   ├── PluginRuntime   # 插件运行时信息（新增 capabilities, priority）
│   │   ├── PluginConnectionPool  # 插件连接池（新增）
│   │   └── UnixSocketServer      # Socket 服务器（更新握手逻辑）
│   ├── server/mod.rs
│   │   └── VConnectIMServer      # 新增 plugin_connection_pool 字段
│   └── main.rs             # 初始化连接池并设置到 server
```

## 插件开发 / Plugin Development

### 插件声明能力 / Plugin Declares Capabilities

```rust
// v-connect-im-plugin-example/src/main.rs
const PLUGIN_NO: &str = "v.plugin.example";
const VERSION: &str = "0.1.0";
const PRIORITY: i32 = 1;  // 优先级：数字越大越先执行

impl Plugin for AIExample {
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        let content = ctx.get_payload_str("content").unwrap_or("");
        
        // 处理消息
        ctx.reply(json!({
            "type": 1,
            "content": format!("我是{}, 收到您的消息: {}", self.config.name, content)
        }))?;
        
        Ok(())
    }
}
```

### 插件拦截消息 / Plugin Intercepts Message

如果插件想要停止消息传播：

```rust
ctx.reply(json!({
    "type": 1,
    "content": "消息已被拦截",
    "flow": "stop"  // ← 停止传播
}))?;
```

## 能力类型 / Capability Types

插件在握手时声明支持的能力：

```rust
// v/src/plugin/client.rs
fn capabilities(&self) -> Vec<String> {
    vec![
        "message".into(),      // 消息事件
        "room".into(),         // 房间事件
        "connection".into(),   // 连接事件
        "user".into()          // 用户事件
    ]
}
```

**当前实现：**
- ✅ `message` - 消息事件（已实现）
- ⏳ `room` - 房间事件（待实现）
- ⏳ `connection` - 连接事件（待实现）
- ⏳ `user` - 用户事件（待实现）

## 事件分发流程 / Event Dispatch Flow

```
1. 用户发送消息
   ↓
2. v-connect-im 接收消息
   ↓
3. 调用 plugin_connection_pool.broadcast_message_event()
   ↓
4. 获取所有支持 "message" 能力的插件
   ↓
5. 按优先级排序（降序）
   ↓
6. 依次发送事件到每个插件
   ↓
7. 等待插件响应
   ↓
8. 检查响应中的 flow 字段
   ├─ "stop" → 停止传播
   └─ "continue" 或无 → 继续下一个插件
   ↓
9. 返回所有插件响应
```

## 优先级示例 / Priority Example

```
Plugin A: priority = 100
Plugin B: priority = 50
Plugin C: priority = 10

执行顺序 / Execution Order:
1. Plugin A (100) - 先执行
2. Plugin B (50)
3. Plugin C (10) - 最后执行

如果 Plugin A 返回 flow: "stop"，则 B 和 C 不会执行
```

## 下一步 / Next Steps

### 1. 集成到实际消息处理流程

在消息处理逻辑中调用插件：

```rust
// v-connect-im/src/main.rs 或消息处理模块
async fn handle_incoming_message(server: &VConnectIMServer, message: ImMessage) -> Result<()> {
    // 1. 调用插件
    if let Some(pool) = &server.plugin_connection_pool {
        let payload = json!({
            "content": message.content,
            "from_uid": message.from_uid,
            "to_uid": message.to_uid,
        });
        
        let responses = pool.broadcast_message_event(&payload).await?;
        
        // 检查是否被拦截
        for (name, response) in responses {
            if response.get("flow").and_then(|v| v.as_str()) == Some("stop") {
                info!("Message intercepted by plugin: {}", name);
                return Ok(());
            }
        }
    }
    
    // 2. 继续原有处理逻辑
    // ...
    
    Ok(())
}
```

### 2. 实现其他事件类型

- `room.join` - 用户加入房间
- `room.leave` - 用户离开房间
- `connection.open` - 连接建立
- `connection.close` - 连接关闭
- `user.online` - 用户上线
- `user.offline` - 用户下线

### 3. 添加超时控制

```rust
// 在 send_event 中添加超时
use tokio::time::timeout;

let result = timeout(
    Duration::from_secs(5),
    self.send_event(plugin_name, event_type, payload)
).await??;
```

### 4. 添加错误重试

```rust
// 在发送失败时重试
let mut retries = 3;
while retries > 0 {
    match self.send_event(...).await {
        Ok(response) => return Ok(response),
        Err(e) => {
            retries -= 1;
            if retries == 0 {
                return Err(e);
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}
```

### 5. 添加性能监控

```rust
// 记录插件处理时间
let start = Instant::now();
let response = self.send_event(...).await?;
let duration = start.elapsed();

if duration > Duration::from_millis(100) {
    warn!("Plugin {} slow response: {:?}", plugin_name, duration);
}
```

## 测试清单 / Test Checklist

- [ ] 插件能够接收消息事件
- [ ] 插件响应被正确返回
- [ ] 优先级排序正确
- [ ] 消息拦截功能正常
- [ ] 多个插件同时工作
- [ ] 插件崩溃不影响主服务
- [ ] 连接断开后能够重连

## 已知限制 / Known Limitations

1. **连接池只在握手时注册**
   - 如果插件重启，需要重新握手
   
2. **没有连接健康检查**
   - 建议添加心跳机制

3. **没有并发控制**
   - 所有插件串行执行
   - 可以改为并发执行提高性能

4. **没有事件队列**
   - 事件是同步发送的
   - 高负载时可能阻塞

## 故障排查 / Troubleshooting

### 问题：插件没有收到消息

**检查：**
1. 插件是否成功注册到连接池？
   ```
   ✅ Plugin example registered to connection pool
   ```

2. 插件是否声明了 "message" 能力？
   ```rust
   fn capabilities(&self) -> Vec<String> {
       vec!["message".into(), ...]
   }
   ```

3. 测试 API 是否正常？
   ```bash
   curl -X POST http://localhost:8080/api/v1/plugin/test_message -d '{"content":"test"}'
   ```

### 问题：插件响应超时

**原因：** 插件处理时间过长

**解决：** 在插件中添加超时控制或异步处理

### 问题：消息没有被拦截

**检查响应格式：**
```json
{
    "type": 1,
    "content": "...",
    "flow": "stop"  // ← 必须是 "stop"
}
```

## 完成状态 / Completion Status

✅ **核心功能已完成：**
- 插件能力管理
- 连接池管理
- 事件分发
- 消息拦截
- 优先级排序
- 测试 API

⏳ **待完善：**
- 集成到实际消息流程
- 其他事件类型
- 超时和重试
- 性能监控
- 健康检查

现在可以通过测试 API 验证插件消息分发功能了！🎉
