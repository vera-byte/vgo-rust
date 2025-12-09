# ✅ 事件监听器 Protobuf 迁移完成

## 概述

所有事件监听器 trait 的方法签名已更新为使用 Protobuf 消息类型，实现类型安全的插件开发。

## 变更内容

### 1. 存储事件监听器（StorageEventListener）

**文件：** `/v/src/plugin/events/storage.rs`

#### 之前（JSON）

```rust
async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
    let message_id = ctx.get_payload_str("message_id").unwrap_or("");
    // 手动解析 JSON 字段
}
```

#### 之后（Protobuf）

```rust
async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
    // 类型安全的字段访问
    let message_id = &req.message_id;
    let from_uid = &req.from_uid;
    
    Ok(SaveMessageResponse {
        status: "ok".to_string(),
        message_id: message_id.clone(),
    })
}
```

#### 更新的方法

| 方法名 | 入参 | 出参 |
|--------|------|------|
| `storage_message_save` | `&SaveMessageRequest` | `SaveMessageResponse` |
| `storage_offline_save` | `&SaveOfflineMessageRequest` | `SaveOfflineMessageResponse` |
| `storage_offline_pull` | `&PullOfflineMessagesRequest` | `PullOfflineMessagesResponse` |
| `storage_offline_ack` | `&AckOfflineMessagesRequest` | `AckOfflineMessagesResponse` |
| `storage_offline_count` | `&CountOfflineMessagesRequest` | `CountOfflineMessagesResponse` |
| `storage_room_add_member` | `&AddRoomMemberRequest` | `AddRoomMemberResponse` |
| `storage_room_remove_member` | `&RemoveRoomMemberRequest` | `RemoveRoomMemberResponse` |
| `storage_room_list_members` | `&GetRoomMembersRequest` | `GetRoomMembersResponse` |

**总计：8 个方法**

### 2. 认证事件监听器（AuthEventListener）

**文件：** `/v/src/plugin/events/auth.rs`

#### 之前（JSON）

```rust
async fn auth_login(&mut self, ctx: &mut Context) -> Result<()> {
    let username = ctx.get_payload_str("username").unwrap_or("");
    let password = ctx.get_payload_str("password").unwrap_or("");
    // 手动解析和验证
}
```

#### 之后（Protobuf）

```rust
async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse> {
    // 类型安全的字段访问
    let username = &req.username;
    let password = &req.password;
    let device_id = &req.device_id;
    
    Ok(LoginResponse {
        status: "ok".to_string(),
        token: "token123".to_string(),
        uid: username.clone(),
        expires_at: 1234567890,
    })
}
```

#### 更新的方法

| 方法名 | 入参 | 出参 |
|--------|------|------|
| `auth_login` | `&LoginRequest` | `LoginResponse` |
| `auth_logout` | `&LogoutRequest` | `LogoutResponse` |
| `auth_kick_out` | `&KickOutRequest` | `KickOutResponse` |
| `auth_renew_token` | `&RenewTokenRequest` | `RenewTokenResponse` |
| `auth_token_replaced` | `&TokenReplacedRequest` | `TokenReplacedResponse` |
| `auth_ban_user` | `&BanUserRequest` | `BanUserResponse` |

**总计：6 个方法**

## 优势对比

### 1. 类型安全

#### 之前（JSON）

```rust
// ❌ 运行时错误
let message_id = ctx.get_payload_str("messge_id"); // 拼写错误
let count = ctx.get_payload_str("count"); // 类型错误（应该是 int）
```

#### 之后（Protobuf）

```rust
// ✅ 编译时检查
let message_id = &req.message_id; // 字段名错误会编译失败
let count = req.count; // 类型自动正确（i32）
```

### 2. IDE 支持

#### 之前（JSON）

```rust
// ❌ 无自动补全
ctx.get_payload_str("???"); // 不知道有哪些字段
```

#### 之后（Protobuf）

```rust
// ✅ 自动补全
req.  // IDE 自动提示所有字段
    // - message_id
    // - from_uid
    // - to_uid
    // - content
    // - timestamp
    // - msg_type
```

### 3. 文档即代码

#### 之前（JSON）

```rust
// ❌ 需要查看文档才知道字段
async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()>;
```

#### 之后（Protobuf）

```rust
// ✅ 类型定义即文档
async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse>;

// 查看 SaveMessageRequest 定义即可知道所有字段
```

### 4. 重构支持

#### 之前（JSON）

```rust
// ❌ 字段重命名需要全局搜索替换
ctx.get_payload_str("message_id") // 容易遗漏
```

#### 之后（Protobuf）

```rust
// ✅ IDE 重构工具自动处理
req.message_id // 重命名字段，所有引用自动更新
```

## 使用示例

### 存储插件实现

```rust
use v::plugin::pdk::StorageEventListener;
use v::plugin::protocol::*;
use async_trait::async_trait;
use anyhow::Result;

pub struct SledStorageEventListener {
    db: sled::Db,
}

#[async_trait]
impl StorageEventListener for SledStorageEventListener {
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
        // 类型安全的字段访问
        let key = format!("msg:{}", req.message_id);
        let value = serde_json::json!({
            "from_uid": req.from_uid,
            "to_uid": req.to_uid,
            "content": req.content,
            "timestamp": req.timestamp,
            "msg_type": req.msg_type,
        });
        
        self.db.insert(key.as_bytes(), serde_json::to_vec(&value)?)?;
        
        Ok(SaveMessageResponse {
            status: "ok".to_string(),
            message_id: req.message_id.clone(),
        })
    }
    
    async fn storage_offline_count(&mut self, req: &CountOfflineMessagesRequest) -> Result<CountOfflineMessagesResponse> {
        let prefix = format!("offline:{}:", req.uid);
        let count = self.db
            .scan_prefix(prefix.as_bytes())
            .count() as i32;
        
        Ok(CountOfflineMessagesResponse {
            status: "ok".to_string(),
            count,
        })
    }
    
    // ... 实现其他方法
}
```

### 认证插件实现

```rust
use v::plugin::pdk::AuthEventListener;
use v::plugin::protocol::*;
use async_trait::async_trait;
use anyhow::Result;

pub struct MyAuthListener {
    // 你的字段
}

#[async_trait]
impl AuthEventListener for MyAuthListener {
    async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse> {
        // 验证用户名和密码
        if req.username == "admin" && req.password == "password" {
            Ok(LoginResponse {
                status: "ok".to_string(),
                token: generate_token(),
                uid: req.username.clone(),
                expires_at: chrono::Utc::now().timestamp() + 3600,
            })
        } else {
            Ok(LoginResponse {
                status: "error".to_string(),
                token: String::new(),
                uid: String::new(),
                expires_at: 0,
            })
        }
    }
    
    async fn auth_logout(&mut self, req: &LogoutRequest) -> Result<LogoutResponse> {
        // 清理 token
        invalidate_token(&req.token);
        
        Ok(LogoutResponse {
            status: "ok".to_string(),
        })
    }
    
    // ... 实现其他方法
}
```

## 迁移指南

### 步骤 1：更新导入

```rust
// 之前
use crate::plugin::pdk::Context;

// 之后
use crate::plugin::protocol::*;
```

### 步骤 2：更新方法签名

```rust
// 之前
async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()>

// 之后
async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse>
```

### 步骤 3：更新实现

```rust
// 之前
let message_id = ctx.get_payload_str("message_id").unwrap_or("");
ctx.reply(json!({"status": "ok", "message_id": message_id}))?;

// 之后
let message_id = &req.message_id;
Ok(SaveMessageResponse {
    status: "ok".to_string(),
    message_id: message_id.clone(),
})
```

## 编译验证

```bash
cargo check -p v
# ✅ Finished `dev` profile
```

## 破坏性变更

### 影响范围

- ✅ 所有实现 `StorageEventListener` 的插件
- ✅ 所有实现 `AuthEventListener` 的插件

### 迁移工作量

- **小型插件**：10-30 分钟
- **中型插件**：30-60 分钟
- **大型插件**：1-2 小时

### 迁移收益

- ✅ 类型安全
- ✅ 编译时检查
- ✅ IDE 支持
- ✅ 更好的文档
- ✅ 更容易重构

## 下一步

1. **更新现有插件** - 迁移存储插件和认证插件代码
2. **添加网关事件监听器** - 为网关插件创建 trait
3. **性能测试** - 对比 JSON vs Protobuf 性能
4. **文档更新** - 更新插件开发文档

## 相关文档

- [Proto 完成说明](/PROTO_COMPLETE.md)
- [Proto 结构说明](/PROTO_STRUCTURE.md)
- [Protobuf 完全重构](/PROTOBUF_FULL_REFACTOR.md)

---

**完成日期**：2025-12-09  
**状态**：✅ 完成  
**维护者**：VGO Team
