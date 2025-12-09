# 插件代码迁移指南

## 概述

由于事件监听器 trait 已更新为使用 Protobuf 类型，现有插件需要进行迁移。

## 修复完成的文件

### ✅ `/v-connect-im/src/plugins/protocol_handler.rs`

**主要变更：**

1. **移除 JSON 依赖**
   ```rust
   // 之前
   use serde_json::Value;
   
   // 之后
   use prost::Message;
   ```

2. **更新 send_event 方法**
   ```rust
   // 之前
   pub async fn send_event(&mut self, event_type: &str, payload: &Value) -> Result<EventResponse>
   
   // 之后
   pub async fn send_event(&mut self, event_type: &str, payload: Vec<u8>) -> Result<EventResponse>
   ```

3. **更新握手响应**
   ```rust
   // 之前
   let response = HandshakeResponse {
       status: "ok".to_string(),
       message: None,
       config: Some(serde_json::json!({})),
       protocol: "protobuf".to_string(),
   };
   
   // 之后
   let response = HandshakeResponse {
       status: "ok".to_string(),
       message: String::new(),
       config: String::new(),
       protocol: "protobuf".to_string(),
   };
   ```

## 需要修复的插件

### ❌ `/v-plugins-hub/v-connect-im-plugin-storage-sled`

**编译错误：**

```
error[E0053]: method `storage_message_save` has an incompatible type for trait
expected `&SaveMessageRequest`
found `&Context`
```

**修复步骤：**

#### 1. 更新导入

```rust
// 文件：src/sled_listener.rs

// 之前
use v::plugin::pdk::{Context, StorageEventListener};

// 之后
use v::plugin::pdk::StorageEventListener;
use v::plugin::protocol::*;
```

#### 2. 更新方法签名

```rust
// 之前
async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
    let message_id = ctx.get_payload_str("message_id").unwrap_or("");
    let from_uid = ctx.get_payload_str("from_uid").unwrap_or("");
    
    // 保存逻辑...
    
    ctx.reply(json!({
        "status": STATUS_OK,
        "message_id": message_id
    }))?;
    
    Ok(())
}

// 之后
async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
    let message_id = &req.message_id;
    let from_uid = &req.from_uid;
    
    // 保存逻辑...
    
    Ok(SaveMessageResponse {
        status: STATUS_OK.to_string(),
        message_id: message_id.clone(),
    })
}
```

#### 3. 更新所有方法

需要更新以下方法：

| 方法名 | 旧签名 | 新签名 |
|--------|--------|--------|
| `storage_message_save` | `(&mut Context) -> Result<()>` | `(&SaveMessageRequest) -> Result<SaveMessageResponse>` |
| `storage_offline_save` | `(&mut Context) -> Result<()>` | `(&SaveOfflineMessageRequest) -> Result<SaveOfflineMessageResponse>` |
| `storage_offline_pull` | `(&mut Context) -> Result<()>` | `(&PullOfflineMessagesRequest) -> Result<PullOfflineMessagesResponse>` |
| `storage_offline_ack` | `(&mut Context) -> Result<()>` | `(&AckOfflineMessagesRequest) -> Result<AckOfflineMessagesResponse>` |
| `storage_offline_count` | `(&mut Context) -> Result<()>` | `(&CountOfflineMessagesRequest) -> Result<CountOfflineMessagesResponse>` |
| `storage_room_add_member` | `(&mut Context) -> Result<()>` | `(&AddRoomMemberRequest) -> Result<AddRoomMemberResponse>` |
| `storage_room_remove_member` | `(&mut Context) -> Result<()>` | `(&RemoveRoomMemberRequest) -> Result<RemoveRoomMemberResponse>` |
| `storage_room_list_members` | `(&mut Context) -> Result<()>` | `(&GetRoomMembersRequest) -> Result<GetRoomMembersResponse>` |

#### 4. 移除 dispatch 调用

```rust
// 文件：src/main.rs

// 之前
fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(self.listener.dispatch(ctx))
    })
}

// 之后
// dispatch 方法已从 trait 中移除，需要手动分发
fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    // 需要实现新的分发逻辑
    // 或者等待 PDK 更新
}
```

### ❌ `/v-plugins-hub/v-connect-im-plugin-gateway`

**类似的修复步骤**

## 完整示例

### 修复后的存储插件方法

```rust
use v::plugin::pdk::StorageEventListener;
use v::plugin::protocol::*;
use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
impl StorageEventListener for SledStorageEventListener {
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
        debug!(
            "💾 保存消息 / Saving message: {} from {} to {}",
            req.message_id, req.from_uid, req.to_uid
        );

        // 构建消息键 / Build message key
        let key = format!("msg:{}:{}", req.to_uid, req.message_id);
        
        // 序列化消息数据 / Serialize message data
        let value = serde_json::json!({
            "message_id": req.message_id,
            "from_uid": req.from_uid,
            "to_uid": req.to_uid,
            "content": req.content,
            "timestamp": req.timestamp,
            "msg_type": req.msg_type,
        });
        
        // 保存到数据库 / Save to database
        self.wal.insert(key.as_bytes(), serde_json::to_vec(&value)?)?;
        self.wal.flush()?;

        info!("✅ 消息已保存 / Message saved: {}", req.message_id);

        Ok(SaveMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }

    async fn storage_offline_count(&mut self, req: &CountOfflineMessagesRequest) -> Result<CountOfflineMessagesResponse> {
        debug!("📊 统计离线消息 / Counting offline messages for: {}", req.uid);

        let prefix = format!("offline:{}:", req.uid);
        let count = self.offline
            .scan_prefix(prefix.as_bytes())
            .count() as i32;

        info!("✅ 离线消息数量 / Offline message count: {} for {}", count, req.uid);

        Ok(CountOfflineMessagesResponse {
            status: "ok".to_string(),
            count,
        })
    }
    
    // ... 其他方法类似
}
```

## 迁移检查清单

### 存储插件

- [ ] 更新导入语句
- [ ] 修改 `storage_message_save` 方法
- [ ] 修改 `storage_offline_save` 方法
- [ ] 修改 `storage_offline_pull` 方法
- [ ] 修改 `storage_offline_ack` 方法
- [ ] 修改 `storage_offline_count` 方法
- [ ] 修改 `storage_room_add_member` 方法
- [ ] 修改 `storage_room_remove_member` 方法
- [ ] 修改 `storage_room_list_members` 方法
- [ ] 移除或更新 `dispatch` 调用
- [ ] 编译测试
- [ ] 运行测试

### 网关插件

- [ ] 更新导入语句
- [ ] 修改相关方法
- [ ] 编译测试
- [ ] 运行测试

## 编译命令

```bash
# 检查存储插件
cargo check -p v-connect-im-plugin-storage-sled

# 检查网关插件
cargo check -p v-connect-im-plugin-gateway

# 编译所有插件
cd v-plugins-hub
cargo build --release
```

## 常见问题

### Q: 为什么要移除 Context？

**A:** 新的设计使用类型安全的 Protobuf 消息，不再需要动态的 Context。

### Q: 如何访问字段？

**A:** 
```rust
// 之前
let message_id = ctx.get_payload_str("message_id").unwrap_or("");

// 之后
let message_id = &req.message_id; // 类型安全，编译时检查
```

### Q: 如何返回响应？

**A:**
```rust
// 之前
ctx.reply(json!({"status": "ok", "count": count}))?;
Ok(())

// 之后
Ok(CountOfflineMessagesResponse {
    status: "ok".to_string(),
    count,
})
```

### Q: dispatch 方法去哪了？

**A:** dispatch 方法已从 trait 中移除，因为现在每个方法都有明确的类型签名，不需要动态分发。PDK 层会处理事件到方法的映射。

## 下一步

1. **修复存储插件** - 按照上述步骤更新代码
2. **修复网关插件** - 类似的修改
3. **测试验证** - 确保功能正常
4. **性能测试** - 对比优化效果

## 相关文档

- [事件监听器迁移说明](/EVENTS_PROTO_MIGRATION.md)
- [Proto 完成说明](/PROTO_COMPLETE.md)
- [Proto 结构说明](/PROTO_STRUCTURE.md)

---

**创建日期**：2025-12-09  
**维护者**：VGO Team
