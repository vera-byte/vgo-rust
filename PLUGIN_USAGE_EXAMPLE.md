# 插件开发完整示例

## 存储插件示例

### 1. 定义事件监听器

```rust
// src/my_storage_listener.rs
use anyhow::Result;
use async_trait::async_trait;
use v::plugin::pdk::StorageEventListener;
use v::plugin::protocol::*;
use v::{debug, info};

pub struct MyStorageListener {
    // 你的存储实现
    db: sled::Db,
}

impl MyStorageListener {
    pub fn new() -> Result<Self> {
        let db = sled::open("./data/my-storage")?;
        Ok(Self { db })
    }
}

#[async_trait]
impl StorageEventListener for MyStorageListener {
    /// 保存消息
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
        info!("💾 保存消息: {}", req.message_id);
        
        // 构建键
        let key = format!("msg:{}:{}", req.to_uid, req.message_id);
        
        // 序列化消息
        let value = serde_json::json!({
            "message_id": req.message_id,
            "from_uid": req.from_uid,
            "to_uid": req.to_uid,
            "content": req.content,
            "timestamp": req.timestamp,
        });
        
        // 保存到数据库
        self.db.insert(key.as_bytes(), serde_json::to_vec(&value)?)?;
        self.db.flush()?;
        
        Ok(SaveMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }
    
    /// 保存离线消息
    async fn storage_offline_save(&mut self, req: &SaveOfflineMessageRequest) -> Result<SaveOfflineMessageResponse> {
        info!("💾 保存离线消息: {} for {}", req.message_id, req.to_uid);
        
        let key = format!("offline:{}:{}", req.to_uid, req.message_id);
        let value = serde_json::json!({
            "message_id": req.message_id,
            "to_uid": req.to_uid,
            "from_uid": req.from_uid,
            "content": req.content,
            "timestamp": req.timestamp,
        });
        
        self.db.insert(key.as_bytes(), serde_json::to_vec(&value)?)?;
        self.db.flush()?;
        
        Ok(SaveOfflineMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }
    
    /// 拉取离线消息
    async fn storage_offline_pull(&mut self, req: &PullOfflineMessagesRequest) -> Result<PullOfflineMessagesResponse> {
        info!("📤 拉取离线消息 for {}", req.uid);
        
        let prefix = format!("offline:{}:", req.uid);
        let messages: Vec<OfflineMessage> = self.db
            .scan_prefix(prefix.as_bytes())
            .take(req.limit as usize)
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| {
                serde_json::from_slice::<serde_json::Value>(&v).ok().and_then(|val| {
                    Some(OfflineMessage {
                        message_id: val.get("message_id")?.as_str()?.to_string(),
                        from_uid: val.get("from_uid")?.as_str()?.to_string(),
                        content: val.get("content")?.as_str()?.to_string(),
                        timestamp: val.get("timestamp")?.as_i64()?,
                    })
                })
            })
            .collect();
        
        Ok(PullOfflineMessagesResponse {
            status: "ok".to_string(),
            messages,
            total: messages.len() as i32,
        })
    }
    
    /// 确认离线消息
    async fn storage_offline_ack(&mut self, req: &AckOfflineMessagesRequest) -> Result<AckOfflineMessagesResponse> {
        info!("✅ 确认离线消息 for {}: {} messages", req.uid, req.message_ids.len());
        
        let mut count = 0;
        for message_id in &req.message_ids {
            let key = format!("offline:{}:{}", req.uid, message_id);
            if self.db.remove(key.as_bytes())?.is_some() {
                count += 1;
            }
        }
        
        self.db.flush()?;
        
        Ok(AckOfflineMessagesResponse {
            status: "ok".to_string(),
            count,
        })
    }
    
    /// 统计离线消息数量
    async fn storage_offline_count(&mut self, req: &CountOfflineMessagesRequest) -> Result<CountOfflineMessagesResponse> {
        let prefix = format!("offline:{}:", req.uid);
        let count = self.db.scan_prefix(prefix.as_bytes()).count() as i32;
        
        Ok(CountOfflineMessagesResponse {
            status: "ok".to_string(),
            count,
        })
    }
    
    /// 添加房间成员
    async fn storage_room_add_member(&mut self, req: &AddRoomMemberRequest) -> Result<AddRoomMemberResponse> {
        info!("➕ 添加房间成员: {} to {}", req.uid, req.room_id);
        
        let key = format!("room:{}:members", req.room_id);
        let mut members: std::collections::HashSet<String> = if let Some(data) = self.db.get(key.as_bytes())? {
            serde_json::from_slice(&data).unwrap_or_default()
        } else {
            std::collections::HashSet::new()
        };
        
        members.insert(req.uid.clone());
        self.db.insert(key.as_bytes(), serde_json::to_vec(&members)?)?;
        self.db.flush()?;
        
        Ok(AddRoomMemberResponse {
            status: "ok".to_string(),
        })
    }
    
    /// 移除房间成员
    async fn storage_room_remove_member(&mut self, req: &RemoveRoomMemberRequest) -> Result<RemoveRoomMemberResponse> {
        info!("➖ 移除房间成员: {} from {}", req.uid, req.room_id);
        
        let key = format!("room:{}:members", req.room_id);
        let mut members: std::collections::HashSet<String> = if let Some(data) = self.db.get(key.as_bytes())? {
            serde_json::from_slice(&data).unwrap_or_default()
        } else {
            std::collections::HashSet::new()
        };
        
        members.remove(&req.uid);
        self.db.insert(key.as_bytes(), serde_json::to_vec(&members)?)?;
        self.db.flush()?;
        
        Ok(RemoveRoomMemberResponse {
            status: "ok".to_string(),
        })
    }
    
    /// 获取房间成员列表
    async fn storage_room_list_members(&mut self, req: &GetRoomMembersRequest) -> Result<GetRoomMembersResponse> {
        info!("📋 获取房间成员: {}", req.room_id);
        
        let key = format!("room:{}:members", req.room_id);
        let members: Vec<String> = if let Some(data) = self.db.get(key.as_bytes())? {
            let set: std::collections::HashSet<String> = serde_json::from_slice(&data).unwrap_or_default();
            set.into_iter().collect()
        } else {
            Vec::new()
        };
        
        Ok(GetRoomMembersResponse {
            status: "ok".to_string(),
            members,
        })
    }
}
```

### 2. 定义插件主结构

```rust
// src/main.rs
use anyhow::Result;
use v::plugin::pdk::{Plugin, Context, dispatch_storage_event};
use v::info;

mod my_storage_listener;
use my_storage_listener::MyStorageListener;

struct MyStoragePlugin {
    listener: MyStorageListener,
}

impl Plugin for MyStoragePlugin {
    type Config = ();
    
    fn new() -> Self {
        info!("🚀 初始化存储插件");
        
        let listener = MyStorageListener::new()
            .expect("无法创建存储监听器");
        
        Self { listener }
    }
    
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // ✅ 使用 PDK 的自动事件分发
        // 注意：这里需要从 Context 获取 EventMessage
        // 目前 Context 还是基于 JSON 的旧设计
        // 可以临时构建 EventMessage 或等待 Context 更新
        
        // 临时方案：手动构建 EventMessage
        use prost::Message;
        use v::plugin::protocol::EventMessage;
        
        let event = EventMessage {
            event_type: ctx.event_type().to_string(),
            payload: serde_json::to_vec(&ctx.payload)?,
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: String::new(),
        };
        
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                dispatch_storage_event(&mut self.listener, &event)
            )
        })?;
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    v::plugin::pdk::run_server::<MyStoragePlugin>().await
}
```

### 3. 配置文件

```json
// plugin.json
{
  "name": "my-storage-plugin",
  "version": "0.1.0",
  "priority": 100,
  "capabilities": [
    "storage.message.save",
    "storage.offline.save",
    "storage.offline.pull",
    "storage.offline.ack",
    "storage.offline.count",
    "storage.room.add_member",
    "storage.room.remove_member",
    "storage.room.list_members"
  ]
}
```

### 4. Cargo.toml

```toml
[package]
name = "my-storage-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
v = { path = "../../v", features = ["protobuf"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sled = "0.34"
prost = "0.13"
chrono = "0.4"
```

## 认证插件示例

```rust
use anyhow::Result;
use async_trait::async_trait;
use v::plugin::pdk::{Plugin, Context, AuthEventListener, dispatch_auth_event};
use v::plugin::protocol::*;

struct MyAuthListener {
    // 你的认证实现
}

#[async_trait]
impl AuthEventListener for MyAuthListener {
    async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse> {
        // 实现登录逻辑
        Ok(LoginResponse {
            status: "ok".to_string(),
            token: "generated_token".to_string(),
            uid: req.username.clone(),
            expires_at: chrono::Utc::now().timestamp_millis() + 86400000,
        })
    }
    
    async fn auth_logout(&mut self, req: &LogoutRequest) -> Result<LogoutResponse> {
        // 实现登出逻辑
        Ok(LogoutResponse {
            status: "ok".to_string(),
        })
    }
    
    // ... 其他方法
}

struct MyAuthPlugin {
    listener: MyAuthListener,
}

impl Plugin for MyAuthPlugin {
    type Config = ();
    
    fn new() -> Self {
        Self {
            listener: MyAuthListener { /* ... */ },
        }
    }
    
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        use prost::Message;
        use v::plugin::protocol::EventMessage;
        
        let event = EventMessage {
            event_type: ctx.event_type().to_string(),
            payload: serde_json::to_vec(&ctx.payload)?,
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: String::new(),
        };
        
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                dispatch_auth_event(&mut self.listener, &event)
            )
        })?;
        
        Ok(())
    }
}
```

## 优势

### ✅ 类型安全
```rust
// ❌ 之前：运行时错误
let message_id = ctx.get_payload_str("message_id").unwrap_or("");

// ✅ 之后：编译时检查
let message_id = &req.message_id;
```

### ✅ 自动编解码
```rust
// PDK 自动处理 Protobuf 编解码
// 你只需要实现业务逻辑
```

### ✅ 零样板代码
```rust
// 不需要手动 match 事件类型
// 不需要手动解码 Protobuf
// 不需要手动构建响应
```

## 测试

```bash
# 编译插件
cargo build --release

# 运行插件
./target/release/my-storage-plugin
```

## 调试

```rust
// 添加日志
use v::{debug, info, warn, error};

async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
    debug!("收到请求: {:?}", req);
    
    // 业务逻辑
    
    info!("处理完成");
    Ok(response)
}
```

---

**创建日期**：2025-12-09  
**维护者**：VGO Team
