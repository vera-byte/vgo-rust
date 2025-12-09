# IM 连接授权流程文档
# IM Connection Authentication Flow

## 📋 授权流程概览 / Authentication Flow Overview

```
┌─────────────┐                                    ┌─────────────┐
│   客户端     │                                    │  IM 服务器   │
│   Client    │                                    │  IM Server  │
└──────┬──────┘                                    └──────┬──────┘
       │                                                  │
       │  1. WebSocket 连接 / WebSocket Connect          │
       ├─────────────────────────────────────────────────>│
       │                                                  │
       │  2. 欢迎消息 / Welcome Message                   │
       │<─────────────────────────────────────────────────┤
       │  { "msg_type": "welcome" }                       │
       │                                                  │
       │                                  3. 启动认证看门狗 / Start Auth Watchdog
       │                                  (deadline_ms = 1500ms)
       │                                                  │
       │  4. 发送认证消息 / Send Auth Message             │
       ├─────────────────────────────────────────────────>│
       │  {                                               │
       │    "msg_type": "auth",                           │
       │    "data": {                                     │
       │      "uid": "user123",                           │
       │      "token": "eyJhbGc..."                       │
       │    }                                             │
       │  }                                               │
       │                                                  │
       │                                  5. 验证 Token / Validate Token
       │                                  ├─> auth.enabled = false?
       │                                  │   └─> 直接通过 / Pass
       │                                  │
       │                                  ├─> auth.enabled = true?
       │                                  │   └─> 调用认证中心 / Call Auth Center
       │                                  │       GET {center_url}/v1/sso/auth?token=xxx
       │                                  │
       │  6. 认证响应 / Auth Response                     │
       │<─────────────────────────────────────────────────┤
       │  {                                               │
       │    "msg_type": "auth_response",                  │
       │    "data": {                                     │
       │      "status": "success",                        │
       │      "message": "Authentication successful"      │
       │    }                                             │
       │  }                                               │
       │                                                  │
       │                                  7. 设置连接 UID / Set Connection UID
       │                                  conn.uid = Some("user123")
       │                                                  │
       │                                  8. 触发事件 / Emit Event
       │                                  emit_custom("connection.authenticated")
       │                                                  │
       │  9. 可以正常通信 / Ready for Communication        │
       │<────────────────────────────────────────────────>│
       │                                                  │
```

---

## 🔐 详细流程说明 / Detailed Flow Description

### 1️⃣ **WebSocket 连接建立** / WebSocket Connection

**文件**: `v-connect-im/src/ws/connection.rs`

```rust
// 客户端连接到 WebSocket
ws://localhost:5200/ws
```

**服务端操作**:
- 生成唯一的 `client_id` (UUID)
- 创建 `Connection` 对象
- 存储到 `connections` 映射中

---

### 2️⃣ **发送欢迎消息** / Send Welcome Message

**文件**: `v-connect-im/src/ws/connection.rs:59-67`

```rust
let welcome_msg = ImMessage {
    msg_type: "welcome".to_string(),
    data: serde_json::json!({}),
    target_uid: None,
    message: welcome_text,
};
server.send_message_to_client(&client_id, Message::Text(...)).await?;
```

**客户端收到**:
```json
{
    "msg_type": "welcome",
    "data": {},
    "message": "Welcome to v-connect-im!"
}
```

---

### 3️⃣ **启动认证看门狗** / Start Authentication Watchdog

**文件**: `v-connect-im/src/ws/connection.rs:69-90`

```rust
let auth_deadline_ms: u64 = cm.get_or("auth.deadline_ms", 1000_u64);

tokio::spawn(async move {
    tokio::time::sleep(Duration::from_millis(auth_deadline_ms)).await;
    
    if let Some(conn) = watchdog_connections.get(&watchdog_client) {
        if conn.uid.is_none() {  // ❌ 如果还没认证
            // 断开连接
            let _ = watchdog_server.send_close_message(&watchdog_client).await;
            watchdog_connections.remove(&watchdog_client);
            tracing::warn!("disconnecting unauthenticated client_id={}", watchdog_client);
        }
    }
});
```

**配置**: `config/default.toml`
```toml
[auth]
deadline_ms = 1500  # 1.5 秒内必须完成认证
```

**作用**:
- ✅ 防止未认证的连接占用资源
- ✅ 强制客户端在规定时间内完成认证
- ✅ 超时自动断开连接

---

### 4️⃣ **客户端发送认证消息** / Client Sends Auth Message

**客户端发送**:
```json
{
    "msg_type": "auth",
    "data": {
        "uid": "user123",
        "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIn0.xxx"
    }
}
```

**字段说明**:
- `uid`: 用户唯一标识符
- `token`: JWT 或其他格式的认证令牌

---

### 5️⃣ **服务端验证 Token** / Server Validates Token

**文件**: `v-connect-im/src/main.rs:422-469`

```rust
"auth" => {
    info!("🔐 Auth request from {}", client_id);
    
    // 提取 token 和 uid
    let token = wk_msg.data.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let uid_opt = wk_msg.data.get("uid").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    // ✅ 验证 token (已修复)
    let is_valid = self.validate_token(token).await.unwrap_or(false);
    
    // ... 后续处理
}
```

#### **Token 验证逻辑** (`main.rs:1212-1234`)

```rust
async fn validate_token(&self, token: &str) -> Result<bool> {
    if token.is_empty() {
        return Ok(false);  // ❌ 空 token 不通过
    }
    
    if let Some(cfg) = &self.auth_config {
        if !cfg.enabled {
            // ⚠️ 开发模式：认证关闭时直接通过
            return Ok(true);
        }
        
        // ✅ 生产模式：调用认证中心验证
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(cfg.timeout_ms))
            .build()?;
            
        let resp = client
            .get(format!("{}/v1/sso/auth", cfg.center_url))
            .query(&[("token", token)])
            .send()
            .await?;
            
        Ok(resp.status().is_success())
    } else {
        // 没有配置认证时，默认通过
        Ok(true)
    }
}
```

**验证模式**:

| 配置 | 行为 |
|-----|------|
| `auth.enabled = false` | ✅ 所有 token 通过（开发模式） |
| `auth.enabled = true` | 🔐 调用认证中心验证 |
| 无 `auth_config` | ✅ 默认通过 |

---

### 6️⃣ **发送认证响应** / Send Auth Response

**成功响应**:
```json
{
    "msg_type": "auth_response",
    "data": {
        "status": "success",
        "message": "Authentication successful"
    }
}
```

**失败响应**:
```json
{
    "msg_type": "auth_response",
    "data": {
        "status": "failed",
        "message": "Authentication failed"
    }
}
```

---

### 7️⃣ **设置连接 UID** / Set Connection UID

```rust
if is_valid {
    if let Some(uid_val) = uid_opt {
        // 设置连接的 uid
        if let Some(mut conn) = self.connections.get_mut(client_id) {
            conn.uid = Some(uid_val.clone());
        }
    }
}
```

**作用**:
- ✅ 将 `uid` 绑定到连接
- ✅ 允许通过 `uid` 查找连接
- ✅ 用于消息路由和离线消息推送

---

### 8️⃣ **触发认证成功事件** / Emit Authenticated Event

```rust
let auth_event = serde_json::json!({
    "client_id": client_id,
    "uid": uid_val,
    "timestamp": chrono::Utc::now().timestamp_millis(),
});

self.plugin_registry
    .emit_custom("connection.authenticated", &auth_event)
    .await;
```

**插件可以监听此事件**:
- 记录用户登录日志
- 推送离线消息
- 更新在线状态
- 触发业务逻辑

---

## ⚙️ 配置说明 / Configuration

### **config/default.toml**

```toml
[auth]
# 认证超时时间（毫秒）/ Authentication deadline (milliseconds)
deadline_ms = 1500

# 是否启用认证 / Enable authentication
# false: 开发模式，所有 token 通过
# true: 生产模式，调用认证中心验证
enabled = false

# 认证中心 URL / Authentication center URL
center_url = "http://127.0.0.1:8090"

# 认证请求超时时间（毫秒）/ Authentication request timeout (milliseconds)
timeout_ms = 1000
```

---

## 🔌 认证中心集成 / Auth Center Integration

### **认证中心 API**

**端点**: `GET {center_url}/v1/sso/auth`

**请求参数**:
```
?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**响应**:
- `200 OK`: Token 有效
- `401 Unauthorized`: Token 无效
- `其他`: 验证失败

### **示例**:

```bash
# 验证 token
curl "http://127.0.0.1:8090/v1/sso/auth?token=xxx"

# 成功响应 (200 OK)
{
    "code": 0,
    "message": "success",
    "data": {
        "uid": "user123",
        "username": "张三"
    }
}

# 失败响应 (401 Unauthorized)
{
    "code": 401,
    "message": "invalid token"
}
```

---

## 🧪 测试场景 / Test Scenarios

### **场景 1: 开发模式（认证关闭）**

**配置**:
```toml
[auth]
enabled = false
```

**行为**:
- ✅ 任何 token 都会通过
- ✅ 甚至空 token 也会通过（如果没有 `validate_token` 检查）
- ✅ 适合本地开发和测试

**测试**:
```javascript
// 客户端代码
ws.send(JSON.stringify({
    msg_type: "auth",
    data: {
        uid: "test_user",
        token: "any_token_works"  // 任何值都可以
    }
}));
```

---

### **场景 2: 生产模式（认证开启）**

**配置**:
```toml
[auth]
enabled = true
center_url = "http://127.0.0.1:8090"
```

**行为**:
- 🔐 调用认证中心验证 token
- ❌ 无效 token 会被拒绝
- ✅ 只有有效 token 才能通过

**测试**:
```javascript
// 1. 先从认证中心获取 token
const loginResp = await fetch('http://127.0.0.1:8090/v1/sso/login', {
    method: 'POST',
    body: JSON.stringify({ username: 'user123', password: 'pass123' })
});
const { token } = await loginResp.json();

// 2. 使用 token 连接 IM
ws.send(JSON.stringify({
    msg_type: "auth",
    data: {
        uid: "user123",
        token: token  // 必须是有效的 token
    }
}));
```

---

### **场景 3: 认证超时**

**配置**:
```toml
[auth]
deadline_ms = 1500  # 1.5 秒
```

**行为**:
- ⏱️ 连接后 1.5 秒内必须完成认证
- ❌ 超时未认证的连接会被断开

**测试**:
```javascript
// 连接但不发送认证消息
const ws = new WebSocket('ws://localhost:5200/ws');

// 1.5 秒后连接会被服务器关闭
// 控制台会看到: "disconnecting unauthenticated client_id=xxx"
```

---

## 🚨 错误处理 / Error Handling

### **1. Token 为空**

```rust
if token.is_empty() {
    return Ok(false);  // 拒绝
}
```

### **2. 认证中心不可达**

```rust
let resp = client.get(...).send().await?;
// 如果网络错误，会返回 Err，unwrap_or(false) 会拒绝认证
```

### **3. 认证超时**

```rust
tokio::time::sleep(Duration::from_millis(auth_deadline_ms)).await;
if conn.uid.is_none() {
    // 断开连接
    watchdog_server.send_close_message(&watchdog_client).await;
}
```

---

## 📊 流程图 / Flow Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    WebSocket 连接                        │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
          ┌──────────────────────┐
          │  发送欢迎消息          │
          │  msg_type: "welcome"  │
          └──────────┬───────────┘
                     │
                     ▼
          ┌──────────────────────┐
          │  启动认证看门狗        │
          │  deadline: 1500ms     │
          └──────────┬───────────┘
                     │
                     ▼
          ┌──────────────────────┐
          │  等待客户端认证消息    │
          │  msg_type: "auth"     │
          └──────────┬───────────┘
                     │
                     ▼
          ┌──────────────────────┐
          │  验证 Token            │
          └──────────┬───────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
         ▼                       ▼
    ┌────────┐            ┌────────┐
    │ 有效    │            │ 无效    │
    └────┬───┘            └───┬────┘
         │                    │
         ▼                    ▼
    ┌────────┐            ┌────────┐
    │设置 UID │            │拒绝认证 │
    └────┬───┘            └───┬────┘
         │                    │
         ▼                    ▼
    ┌────────┐            ┌────────┐
    │触发事件 │            │断开连接 │
    └────┬───┘            └────────┘
         │
         ▼
    ┌────────┐
    │正常通信 │
    └────────┘
```

---

## ✅ 总结 / Summary

### **关键点**:

1. ✅ **认证超时**: 1.5 秒内必须完成认证
2. ✅ **Token 验证**: 支持本地和远程验证
3. ✅ **开发模式**: `enabled = false` 时直接通过
4. ✅ **生产模式**: `enabled = true` 时调用认证中心
5. ✅ **事件通知**: 认证成功后触发 `connection.authenticated` 事件
6. ✅ **UID 绑定**: 认证后将 `uid` 绑定到连接

### **安全建议**:

- 🔐 生产环境必须启用认证 (`enabled = true`)
- 🔐 使用 HTTPS/WSS 加密传输
- 🔐 Token 应该有过期时间
- 🔐 认证中心应该验证 Token 签名
- 🔐 考虑添加 IP 白名单或速率限制

---

**文档更新时间**: 2025-12-09
**Document Updated**: 2025-12-09
