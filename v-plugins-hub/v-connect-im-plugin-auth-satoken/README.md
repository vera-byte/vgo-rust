# SaToken 认证插件
# SaToken Authentication Plugin

## 📋 简介 / Introduction

基于 SaToken 的认证插件，为 v-connect-im 提供完整的用户认证功能。
Authentication plugin based on SaToken, providing complete user authentication for v-connect-im.

## ✨ 功能特性 / Features

- ✅ **用户登录** / User Login
- ✅ **用户登出** / User Logout
- ✅ **Token 验证** / Token Validation
- ✅ **Token 续期** / Token Renewal
- ✅ **用户踢出** / User Kick Out
- ✅ **用户封禁** / User Ban
- ✅ **类型安全** / Type Safe (Protobuf)
- ✅ **高性能** / High Performance

## 🏗️ 架构设计 / Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    v-connect-im                         │
│                                                         │
│  WebSocket 连接 → 发送 auth 消息 → 调用认证插件         │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│         v-connect-im-plugin-auth-satoken                │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │  SaTokenAuthListener                            │   │
│  │  - auth_login()                                 │   │
│  │  - auth_logout()                                │   │
│  │  - auth_kick_out()                              │   │
│  │  - auth_renew_token()                           │   │
│  │  - auth_token_replaced()                        │   │
│  │  - auth_ban_user()                              │   │
│  └─────────────────────────────────────────────────┘   │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              SaToken 认证中心                            │
│              (http://127.0.0.1:8090)                    │
│                                                         │
│  - POST /v1/sso/login       (登录)                      │
│  - POST /v1/sso/logout      (登出)                      │
│  - GET  /v1/sso/auth        (验证)                      │
│  - POST /v1/sso/kickout     (踢出)                      │
│  - POST /v1/sso/ban         (封禁)                      │
└─────────────────────────────────────────────────────────┘
```

## 🚀 快速开始 / Quick Start

### 1. 编译插件 / Build Plugin

```bash
cd v-plugins-hub/v-connect-im-plugin-auth-satoken
cargo build --release
```

### 2. 配置 / Configuration

编辑 `plugin.json`:
```json
{
    "plugin_no": "v.plugin.auth-satoken",
    "name": "v-connect-im-plugin-auth-satoken",
    "version": "0.1.0",
    "capabilities": ["auth"],
    "priority": 1000
}
```

### 3. 运行插件 / Run Plugin

```bash
# 开发模式 / Development mode
cargo run

# 生产模式 / Production mode
./target/release/v-connect-im-plugin-auth-satoken --socket ./plugins/auth-satoken.sock
```

### 4. 配置 IM 服务 / Configure IM Service

在 `v-connect-im/config/default.toml` 中添加:

```toml
[plugins]
dev_plugins = [
    "auth-satoken:/Users/mac/workspace/vgo-rust/v-plugins-hub/v-connect-im-plugin-auth-satoken",
]
```

## 📝 配置说明 / Configuration

### SaTokenAuthConfig

```rust
pub struct SaTokenAuthConfig {
    /// SaToken 服务地址 / SaToken service URL
    pub satoken_url: String,        // 默认: "http://127.0.0.1:8090"

    /// 请求超时时间（毫秒）/ Request timeout (milliseconds)
    pub timeout_ms: u64,            // 默认: 3000

    /// Token 有效期（秒）/ Token validity period (seconds)
    pub token_ttl: i64,             // 默认: 7200 (2小时)
}
```

## 🔐 认证流程 / Authentication Flow

### 1. 用户登录 / User Login

```
客户端 → IM 服务 → 认证插件 → SaToken
  ↓         ↓          ↓          ↓
发送登录   转发请求   调用API    验证凭证
请求                            返回Token
  ↓         ↓          ↓          ↓
接收Token ← 返回响应 ← 处理响应 ← 生成Token
```

**客户端请求**:
```json
{
    "msg_type": "auth",
    "data": {
        "uid": "user123",
        "token": "eyJhbGc..."
    }
}
```

**认证插件调用 SaToken**:
```http
POST http://127.0.0.1:8090/v1/sso/auth?token=eyJhbGc...
```

**响应**:
```json
{
    "msg_type": "auth_response",
    "data": {
        "status": "success",
        "message": "Authentication successful"
    }
}
```

## 🔌 API 接口 / API Endpoints

### 1. auth_login - 用户登录

**请求** (LoginRequest):
```protobuf
message LoginRequest {
    string username = 1;
    string password = 2;
}
```

**响应** (LoginResponse):
```protobuf
message LoginResponse {
    string status = 1;      // "ok" or "error"
    string token = 2;       // JWT token
    string uid = 3;         // User ID
    int64 expires_at = 4;   // Token expiration timestamp
}
```

### 2. auth_logout - 用户登出

**请求** (LogoutRequest):
```protobuf
message LogoutRequest {
    string uid = 1;
    string token = 2;
}
```

**响应** (LogoutResponse):
```protobuf
message LogoutResponse {
    string status = 1;
}
```

### 3. auth_kick_out - 踢出用户

**请求** (KickOutRequest):
```protobuf
message KickOutRequest {
    string uid = 1;
    string reason = 2;
}
```

### 4. auth_renew_token - Token 续期

**请求** (RenewTokenRequest):
```protobuf
message RenewTokenRequest {
    string token = 1;
}
```

**响应** (RenewTokenResponse):
```protobuf
message RenewTokenResponse {
    string status = 1;
    string token = 2;
    int64 expires_at = 3;
}
```

### 5. auth_ban_user - 封禁用户

**请求** (BanUserRequest):
```protobuf
message BanUserRequest {
    string uid = 1;
    string reason = 2;
    int64 duration = 3;  // 封禁时长（秒）
}
```

## 🧪 测试 / Testing

### 测试认证流程

```bash
# 1. 启动 SaToken 认证中心
# (假设已在 http://127.0.0.1:8090 运行)

# 2. 启动 IM 服务
cd v-connect-im
cargo run

# 3. 启动认证插件
cd v-plugins-hub/v-connect-im-plugin-auth-satoken
cargo run

# 4. 连接 WebSocket 并测试
```

**JavaScript 测试代码**:
```javascript
const ws = new WebSocket('ws://localhost:5200/ws');

ws.onopen = () => {
    // 发送认证消息
    ws.send(JSON.stringify({
        msg_type: "auth",
        data: {
            uid: "user123",
            token: "your_token_here"
        }
    }));
};

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    console.log('收到消息:', msg);
    
    if (msg.msg_type === "auth_response") {
        if (msg.data.status === "success") {
            console.log("✅ 认证成功！");
        } else {
            console.log("❌ 认证失败！");
        }
    }
};
```

## 📊 性能特性 / Performance

- ✅ **异步处理**: 基于 Tokio 异步运行时
- ✅ **连接池**: 复用 HTTP 连接
- ✅ **超时控制**: 可配置的请求超时
- ✅ **类型安全**: Protobuf 编译时检查
- ✅ **零拷贝**: 直接处理 Protobuf 消息

## 🔧 故障排查 / Troubleshooting

### 1. 插件无法启动

**问题**: `Failed to read plugin.json`

**解决**:
```bash
# 确保 plugin.json 在正确位置
ls -la plugin.json

# 检查 JSON 格式
cat plugin.json | jq .
```

### 2. 认证失败

**问题**: 所有 token 都被拒绝

**检查**:
```bash
# 1. SaToken 服务是否运行
curl http://127.0.0.1:8090/health

# 2. 检查插件日志
# 查看是否有连接错误

# 3. 测试 SaToken API
curl "http://127.0.0.1:8090/v1/sso/auth?token=test_token"
```

### 3. 超时错误

**问题**: `timeout_ms` 配置

**解决**:
```rust
// 增加超时时间
SaTokenAuthConfig {
    timeout_ms: 5000,  // 5秒
    ..Default::default()
}
```

## 📚 相关文档 / Related Documentation

- [v-connect-im 认证流程](../../v-connect-im/docs/AUTHENTICATION.md)
- [插件开发指南](../../docs/plugin/development.md)
- [SaToken 官方文档](https://sa-token.cc/)

## 🤝 贡献 / Contributing

欢迎提交 Issue 和 Pull Request！
Welcome to submit Issues and Pull Requests!

## 📄 许可证 / License

MIT License

---

**开发团队**: VGO Team
**最后更新**: 2025-12-09
