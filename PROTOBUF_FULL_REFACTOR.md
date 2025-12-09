# Protobuf 完全重构方案

## 目标

1. **移除 JSON 依赖** - 插件代码完全使用 Protobuf 结构
2. **统一类型定义** - 只使用 proto 生成的类型，移除重复定义
3. **类型安全** - 编译时检查，避免运行时错误

## 当前问题

### 问题 1：重复定义

```rust
// ❌ protocol.rs 中定义
pub struct HandshakeRequest {
    pub name: String,
    pub version: String,
    // ...
}

// ❌ v.plugin.rs 中也定义（proto 生成）
pub struct HandshakeRequest {
    pub name: String,
    pub version: String,
    // ...
}
```

### 问题 2：JSON 依赖

```rust
// ❌ 插件代码使用 JSON
ctx.reply(json!({
    "status": "ok",
    "count": count
}))?;
```

## 解决方案

### 1. 统一使用 proto 生成的类型

```rust
// ✅ protocol.rs
pub use crate::plugin::proto_codec::pb::{
    HandshakeRequest,
    HandshakeResponse,
    EventMessage,
    EventResponse,
    // 业务消息
    SaveMessageRequest,
    SaveMessageResponse,
    // ...
};
```

### 2. 插件使用 Protobuf 结构

```rust
// ✅ 插件代码
use v::plugin::protocol::{
    SaveMessageRequest,
    SaveMessageResponse,
};

impl StorageEventListener for SledStorageEventListener {
    async fn storage_save_message(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
        // 类型安全的处理
        self.wal.insert(req.message_id.as_bytes(), ...)?;
        
        // 返回 Protobuf 结构
        Ok(SaveMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }
}
```

### 3. PDK 适配层

```rust
// PDK 提供便捷的适配层
pub trait StorageEventListener {
    async fn storage_save_message(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse>;
    async fn storage_save_offline_message(&mut self, req: &SaveOfflineMessageRequest) -> Result<SaveOfflineMessageResponse>;
    // ...
    
    // 自动分发
    async fn dispatch(&mut self, event: &EventMessage) -> Result<EventResponse> {
        match event.event_type.as_str() {
            "storage.save_message" => {
                let req = SaveMessageRequest::decode(event.payload.as_slice())?;
                let resp = self.storage_save_message(&req).await?;
                Ok(EventResponse {
                    status: "ok".to_string(),
                    flow: "continue".to_string(),
                    data: resp.encode_to_vec(),
                    error: String::new(),
                })
            }
            // ...
        }
    }
}
```

## 优势

### 1. 类型安全

```rust
// ✅ 编译时检查
let req = SaveMessageRequest {
    message_id: "msg123".to_string(),
    from_uid: "user1".to_string(),
    // 缺少字段会编译错误
};

// ❌ JSON 运行时才发现错误
let req = json!({
    "message_id": "msg123",
    // 缺少字段运行时才报错
});
```

### 2. IDE 支持

- ✅ 自动补全
- ✅ 类型提示
- ✅ 重构支持
- ✅ 跳转定义

### 3. 性能

- ✅ 无需 JSON 序列化/反序列化
- ✅ 直接 Protobuf 编解码
- ✅ 零拷贝（某些场景）

### 4. 文档

```proto
// proto 文件即文档
message SaveMessageRequest {
  string message_id = 1;    // 消息ID / Message ID
  string from_uid = 2;      // 发送者UID / Sender UID
  string to_uid = 3;        // 接收者UID / Receiver UID
  string content = 4;       // 消息内容 / Message content
  int64 timestamp = 5;      // 时间戳 / Timestamp
  string msg_type = 6;      // 消息类型 / Message type
}
```

## 实施步骤

### 阶段 1：更新 proto 定义 ✅

- [x] 添加所有业务消息定义
- [x] 生成 Rust 代码

### 阶段 2：统一类型定义 ✅

- [x] protocol.rs 重新导出 proto 类型
- [x] 移除重复定义

### 阶段 3：更新编解码器 ✅

- [x] 简化 proto_codec.rs
- [x] 直接使用 Protobuf 编解码

### 阶段 4：更新客户端 ✅

- [x] PluginHandler trait 使用 Protobuf 类型
- [x] 移除 Value 依赖

### 阶段 5：更新 PDK（进行中）

- [ ] 更新 Context API
- [ ] 更新事件监听器 trait
- [ ] 提供便捷的辅助方法

### 阶段 6：更新插件代码

- [ ] 存储插件使用 Protobuf 结构
- [ ] 网关插件使用 Protobuf 结构

## 示例对比

### 之前（JSON）

```rust
// 插件代码
async fn storage_save_message(&mut self, ctx: &mut Context) -> Result<()> {
    // ❌ 手动解析 JSON
    let message_id = ctx.get_payload_str("message_id").unwrap_or("");
    let from_uid = ctx.get_payload_str("from_uid").unwrap_or("");
    
    // 处理...
    
    // ❌ 手动构建 JSON
    ctx.reply(json!({
        "status": "ok",
        "message_id": message_id
    }))?;
    
    Ok(())
}
```

### 之后（Protobuf）

```rust
// 插件代码
async fn storage_save_message(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
    // ✅ 类型安全的字段访问
    let message_id = &req.message_id;
    let from_uid = &req.from_uid;
    
    // 处理...
    
    // ✅ 类型安全的响应构建
    Ok(SaveMessageResponse {
        status: "ok".to_string(),
        message_id: message_id.clone(),
    })
}
```

## 下一步

1. **完成 PDK 重构** - 更新 Context 和事件监听器
2. **更新插件代码** - 使用 Protobuf 结构
3. **测试验证** - 确保功能正常
4. **性能测试** - 对比优化效果
5. **文档更新** - 更新开发文档

---

**状态**：进行中  
**完成度**：60%  
**预计完成**：今天
