# 插件消息处理流程说明 / Plugin Message Flow

## 当前问题 / Current Issue

插件已成功连接到 v-connect-im，但**没有收到消息事件**。

## 原因分析 / Root Cause

v-connect-im 有**两套独立的插件系统**：

### 1. 内置插件系统（已移除）
- 位置：`v-connect-im/src/plugins/mod.rs`
- 接口：`Plugin` trait
- 方法：`on_message_incoming`, `on_message_outgoing`
- 调用：直接在 Rust 代码中调用

### 2. 外部插件系统（Unix Socket）
- 位置：`v-connect-im/src/plugins/runtime.rs`
- 通信：Unix Socket
- 协议：JSON 消息
- **问题：没有实现消息分发机制**

## 缺失的功能 / Missing Functionality

### 需要实现的流程

```
用户发送消息
    ↓
v-connect-im 接收消息
    ↓
【缺失】遍历所有 Unix Socket 插件
    ↓
【缺失】通过 Socket 发送事件
    ↓
插件处理消息
    ↓
【缺失】插件返回响应
    ↓
【缺失】v-connect-im 根据响应决定是否继续
    ↓
v-connect-im 继续处理或停止
```

## 当前实现状态 / Current Implementation Status

### ✅ 已实现

1. **插件连接管理**
   - 插件启动
   - Socket 连接
   - 握手协议
   - 进程监控

2. **插件端消息处理**
   - `Plugin::receive()` 方法
   - 消息解析
   - 响应构建

### ❌ 未实现

1. **主服务消息分发**
   - 没有将消息事件发送给 Unix Socket 插件
   - 没有等待插件响应
   - 没有根据响应控制流程

2. **事件路由**
   - 没有根据插件 capabilities 路由事件
   - 没有优先级处理
   - 没有并发控制

## 需要实现的代码 / Code to Implement

### 1. 在 PluginRuntimeManager 中添加事件发送方法

```rust
// v-connect-im/src/plugins/runtime.rs
impl PluginRuntimeManager {
    /// 向所有插件广播消息事件 / Broadcast message event to all plugins
    pub async fn broadcast_message_event(&self, event: &MessageEvent) -> Result<Vec<PluginResponse>> {
        let mut responses = Vec::new();
        
        for plugin in self.plugins.iter() {
            let runtime = plugin.value();
            
            // 检查插件是否支持 message 事件
            if !runtime.capabilities.contains(&"message".to_string()) {
                continue;
            }
            
            // 通过 socket 发送事件
            if let Some(response) = self.send_event_to_plugin(&runtime, event).await? {
                responses.push(response);
                
                // 如果插件返回 Stop，停止传播
                if response.flow == PluginFlow::Stop {
                    break;
                }
            }
        }
        
        Ok(responses)
    }
    
    /// 向单个插件发送事件 / Send event to single plugin
    async fn send_event_to_plugin(
        &self,
        runtime: &PluginRuntime,
        event: &MessageEvent,
    ) -> Result<Option<PluginResponse>> {
        // 构建事件消息
        let msg = json!({
            "event": "message.incoming",
            "payload": {
                "content": event.content,
                "from_uid": event.from_uid,
                "to_uid": event.to_uid,
                // ... 其他字段
            }
        });
        
        // 通过 socket 发送
        // TODO: 需要维护 socket 连接池
        
        Ok(None)
    }
}
```

### 2. 在消息处理逻辑中调用插件

```rust
// v-connect-im/src/main.rs 或消息处理模块
async fn handle_message(server: &VConnectIMServer, message: ImMessage) -> Result<()> {
    // 1. 调用 Unix Socket 插件
    if let Some(runtime_manager) = &server.plugin_runtime_manager {
        let event = MessageEvent {
            content: message.content.clone(),
            from_uid: message.from_uid.clone(),
            to_uid: message.to_uid.clone(),
        };
        
        let responses = runtime_manager.broadcast_message_event(&event).await?;
        
        // 检查是否有插件要求停止处理
        for response in responses {
            if response.flow == PluginFlow::Stop {
                info!("Message processing stopped by plugin");
                return Ok(());
            }
        }
    }
    
    // 2. 继续原有的消息处理逻辑
    // ...
    
    Ok(())
}
```

### 3. 维护 Socket 连接

```rust
// v-connect-im/src/plugins/runtime.rs
pub struct PluginRuntime {
    pub name: String,
    pub path: PathBuf,
    pub version: Option<String>,
    pub status: Arc<RwLock<PluginStatus>>,
    pub process: Arc<RwLock<Option<Child>>>,
    pub socket_path: Option<PathBuf>,
    pub last_heartbeat: Arc<RwLock<Option<Instant>>>,
    pub capabilities: Vec<String>,           // ← 新增：插件能力
    pub socket_stream: Arc<RwLock<Option<UnixStream>>>,  // ← 新增：Socket 连接
}
```

## 临时解决方案 / Workaround

在完整实现消息分发之前，可以使用以下方案测试插件：

### 方案 1：手动发送测试事件

在 v-connect-im 启动后，手动通过 socket 发送测试消息：

```bash
# 使用 socat 发送测试消息
echo '{"event":"message.incoming","payload":{"content":"hello"}}' | \
  socat - UNIX-CONNECT:~/vp/sockets/runtime.sock
```

### 方案 2：实现简单的测试接口

在 v-connect-im 中添加一个测试接口：

```rust
// v-connect-im/src/api/v1/plugin/test.rs
use actix_web::{post, web, HttpResponse};

#[post("/test_plugin")]
async fn test_plugin(
    server: web::Data<Arc<VConnectIMServer>>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    if let Some(runtime_manager) = &server.plugin_runtime_manager {
        // 构建测试事件
        let event = json!({
            "event": "message.incoming",
            "payload": body.into_inner()
        });
        
        // 发送给所有插件
        // TODO: 实现发送逻辑
        
        HttpResponse::Ok().json(json!({"status": "ok"}))
    } else {
        HttpResponse::InternalServerError().json(json!({"error": "no runtime manager"}))
    }
}
```

然后通过 HTTP 测试：

```bash
curl -X POST http://localhost:8080/api/v1/plugin/test \
  -H "Content-Type: application/json" \
  -d '{"content":"hello","from_uid":"user123"}'
```

## 完整实现路线图 / Implementation Roadmap

### 阶段 1：基础消息分发（必需）
- [ ] 在 PluginRuntimeManager 中维护 socket 连接
- [ ] 实现 `send_event_to_plugin` 方法
- [ ] 实现 `broadcast_message_event` 方法
- [ ] 在消息处理流程中调用插件

### 阶段 2：能力路由（推荐）
- [ ] 在握手时保存插件 capabilities
- [ ] 根据 capabilities 路由事件
- [ ] 支持多种事件类型（message, user, room 等）

### 阶段 3：高级特性（可选）
- [ ] 插件优先级排序
- [ ] 并发控制和超时
- [ ] 错误重试机制
- [ ] 性能监控和统计

### 阶段 4：完整集成（未来）
- [ ] 统一内置和外部插件接口
- [ ] 动态插件加载/卸载
- [ ] 插件热更新
- [ ] 插件市场集成

## 参考实现 / Reference Implementation

可以参考以下项目的实现：

1. **WuKongIM 插件系统**
   - Go 语言实现
   - 支持内置和外部插件
   - 完整的事件分发机制

2. **Envoy Proxy 扩展系统**
   - C++ 实现
   - 支持 WASM 插件
   - 高性能事件处理

3. **Kong Gateway 插件系统**
   - Lua/Go 实现
   - 丰富的插件生态
   - 完善的生命周期管理

## 下一步行动 / Next Steps

1. **评估需求**
   - 确定是否需要完整的消息分发功能
   - 评估性能和复杂度要求

2. **选择方案**
   - 简单方案：只支持特定事件
   - 完整方案：实现通用事件系统

3. **开始实现**
   - 从最小可用功能开始
   - 逐步添加高级特性

4. **测试验证**
   - 单元测试
   - 集成测试
   - 性能测试

当前插件系统只完成了**连接管理**部分，**消息分发**功能还需要实现！
