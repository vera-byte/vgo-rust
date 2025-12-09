# PDK 重新设计方案

## 目标

1. **移除 `receive` 方法** - 不再需要手动处理事件
2. **自动事件分发** - PDK 自动调用对应的监听器方法
3. **类型安全** - 使用 Protobuf 消息类型
4. **更高可读性** - 清晰的插件结构

## 新设计

### 1. 插件特化（Specialization）

不再使用通用的 `Plugin` trait，而是为每种插件类型提供专门的 trait：

```rust
// 存储插件
pub trait StoragePlugin: Send + Sync {
    type Config: DeserializeOwned + Default;
    
    fn new() -> Self;
    fn listener(&mut self) -> &mut dyn StorageEventListener;
}

// 认证插件
pub trait AuthPlugin: Send + Sync {
    type Config: DeserializeOwned + Default;
    
    fn new() -> Self;
    fn listener(&mut self) -> &mut dyn AuthEventListener;
}
```

### 2. 自动事件分发

PDK 内部实现自动分发：

```rust
// 在 pdk.rs 中
pub async fn dispatch_storage_event(
    listener: &mut dyn StorageEventListener,
    event: &EventMessage,
) -> Result<EventResponse> {
    use prost::Message;
    
    match event.event_type.as_str() {
        "storage.message.save" => {
            let req = SaveMessageRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_message_save(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.save" => {
            let req = SaveOfflineMessageRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_save(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.pull" => {
            let req = PullOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_pull(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.ack" => {
            let req = AckOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_ack(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.count" => {
            let req = CountOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_count(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.add_member" => {
            let req = AddRoomMemberRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_add_member(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.remove_member" => {
            let req = RemoveRoomMemberRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_remove_member(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.list_members" => {
            let req = GetRoomMembersRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_list_members(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        _ => Err(anyhow::anyhow!("Unknown storage event: {}", event.event_type))
    }
}
```

### 3. 简化的插件代码

#### 之前（需要手动 dispatch）

```rust
struct StoragePlugin {
    listener: SledStorageEventListener,
}

impl Plugin for StoragePlugin {
    type Config = SledStorageConfig;
    
    fn new() -> Self {
        let config = SledStorageConfig::default();
        let listener = SledStorageEventListener::new(config).unwrap();
        Self { listener }
    }
    
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // ❌ 需要手动调用 dispatch
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.listener.dispatch(ctx))
        })
    }
}
```

#### 之后（自动 dispatch）

```rust
struct StoragePlugin {
    listener: SledStorageEventListener,
}

impl StoragePlugin for StoragePlugin {
    type Config = SledStorageConfig;
    
    fn new() -> Self {
        let config = SledStorageConfig::default();
        let listener = SledStorageEventListener::new(config).unwrap();
        Self { listener }
    }
    
    fn listener(&mut self) -> &mut dyn StorageEventListener {
        &mut self.listener
    }
}

// ✅ 不需要实现 receive，PDK 自动处理
```

### 4. 新的 PluginHandler 实现

```rust
impl<P: StoragePlugin> PluginHandler for PluginWrapper<P> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn protocol(&self) -> ProtocolFormat {
        self.protocol
    }

    fn config(&mut self, cfg: &str) -> Result<()> {
        if !cfg.is_empty() {
            if let Ok(value) = serde_json::from_str::<Value>(cfg) {
                if let Ok(config) = serde_json::from_value::<P::Config>(value) {
                    // 更新配置
                }
            }
        }
        Ok(())
    }

    fn on_event(&mut self, event: &EventMessage) -> Result<EventResponse> {
        // ✅ 自动分发到对应的监听器方法
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                dispatch_storage_event(self.plugin.listener(), event)
            )
        })
    }
}
```

### 5. 启动函数

```rust
// 存储插件启动
pub async fn run_storage_plugin<P: StoragePlugin + 'static>() -> Result<()> {
    // ... 读取配置
    
    let plugin = P::new();
    let wrapper = StoragePluginWrapper {
        plugin,
        name,
        version,
        priority,
        capabilities,
        protocol: ProtocolFormat::Protobuf,
    };
    
    let client = PluginClient::new(socket_path, wrapper);
    client.run().await
}

// 认证插件启动
pub async fn run_auth_plugin<P: AuthPlugin + 'static>() -> Result<()> {
    // 类似实现
}
```

## 使用示例

### 存储插件

```rust
use v::plugin::pdk::{StoragePlugin, StorageEventListener};
use v::plugin::protocol::*;

struct MyStoragePlugin {
    listener: MyStorageListener,
}

impl StoragePlugin for MyStoragePlugin {
    type Config = MyConfig;
    
    fn new() -> Self {
        Self {
            listener: MyStorageListener::new(),
        }
    }
    
    fn listener(&mut self) -> &mut dyn StorageEventListener {
        &mut self.listener
    }
}

// ✅ 只需实现 StorageEventListener 的方法
#[async_trait]
impl StorageEventListener for MyStorageListener {
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
        // 实现逻辑
        Ok(SaveMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }
    
    // ... 其他方法
}

#[tokio::main]
async fn main() -> Result<()> {
    v::plugin::pdk::run_storage_plugin::<MyStoragePlugin>().await
}
```

## 优势

### ✅ 更简洁

- 不需要实现 `receive` 方法
- 不需要手动调用 `dispatch`
- 代码量减少 50%

### ✅ 类型安全

- 自动 Protobuf 编解码
- 编译时检查事件类型
- 无需手动解析

### ✅ 更高可读性

- 清晰的插件类型（Storage, Auth, Gateway）
- 专注于业务逻辑
- 零样板代码

### ✅ 易于维护

- 事件分发逻辑集中在 PDK
- 插件代码更简单
- 易于测试

## 实施步骤

### 阶段 1：添加分发函数

- [x] 添加 `dispatch_storage_event`
- [ ] 添加 `dispatch_auth_event`
- [ ] 添加 `dispatch_gateway_event`

### 阶段 2：更新 PDK

- [ ] 添加 `StoragePlugin` trait
- [ ] 添加 `AuthPlugin` trait
- [ ] 添加 `GatewayPlugin` trait
- [ ] 更新 `PluginWrapper`

### 阶段 3：更新插件

- [ ] 更新存储插件使用新 API
- [ ] 更新认证插件使用新 API
- [ ] 更新网关插件使用新 API

### 阶段 4：移除旧代码

- [ ] 移除 `Plugin` trait
- [ ] 移除 `Context` 结构
- [ ] 移除 `receive` 方法

## 对比

| 项目 | 旧设计 | 新设计 |
|------|--------|--------|
| 插件代码行数 | ~120 行 | ~60 行 |
| 需要实现的方法 | 5 个 | 2 个 |
| 样板代码 | 多 | 无 |
| 类型安全 | 部分 | 完全 |
| 可读性 | 中 | 高 |
| 维护性 | 中 | 高 |

---

**创建日期**：2025-12-09  
**状态**：设计完成  
**维护者**：VGO Team
