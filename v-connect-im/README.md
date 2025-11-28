- **插件配置示例**
 ```toml
 [plugins]
 trace_enabled = 1
 trace_log_payload = 0
 sensitive_words = ["违禁词", "badword"]
 ```
# v-connect-im 即时通讯服务器

v-connect-im 是一个高性能的即时通讯服务器，采用 Rust 语言开发，支持 WebSocket 和 HTTP 双协议，提供完整的实时消息传输解决方案。

## 🌟 主要特性

### 双协议支持
- **WebSocket 服务器**：支持长连接，实时消息推送
- **HTTP API 服务器**：提供 RESTful 接口，支持消息发送和广播
- **双端口独立运行**：WebSocket (默认5200) 和 HTTP (默认8080)

### 消息功能
- **点对点消息**：支持指定目标客户端的消息发送
- **消息广播**：向所有在线客户端广播消息
- **私聊消息**：专门的私聊消息类型
- **消息回声**：未指定目标时的消息回声机制

### 连接管理
- **客户端连接管理**：支持多客户端并发连接
- **心跳检测**：自动 ping/pong 心跳机制
- **超时清理**：自动清理超时连接
- **连接状态跟踪**：实时监控客户端在线状态

### 插件系统
- **统一插件注册中心**：`PluginRegistry` 负责调度上行/下行钩子，并提供 `on_startup / on_config_update / on_shutdown` 等生命周期回调，插件可以安全感知配置变化。  
  `PluginRegistry` orchestrates inbound/outbound hooks with lifecycle callbacks so each plugin can react to startup, config updates, and graceful shutdowns.
- **授权与敏感词插件**：内置 `DefaultAuthPlugin` 与 `SensitiveWordPlugin`，提供基础鉴权与敏感词替换能力，后者可通过 `plugins.sensitive_words` 配置实时热更。  
  Built-in `DefaultAuthPlugin` and `SensitiveWordPlugin` cover authentication and sensitive-word replacement with dynamic configuration support.
- **Trace 插件**：通过 `plugins.trace_enabled` 与 `plugins.trace_log_payload` 开关调试日志，快速洞察消息收发链路。  
  The Trace plugin helps troubleshoot message flows with optional payload logging.
- **测试插件**：`TestPluginManager` 注入的测试插件可模拟阻塞/统计等行为，方便集成测试或故障注入。  
  The bundled test plugin lets you simulate blocking flows and collect stats for integration testing.
- **插件安装与运行**：保留 `.wkp` 本地插件运行时，支持从 URL 自动下载并解压 .tar.gz 包、`${os}/${arch}` 变量替换、Unix Socket 通信以及自动启动/停止流程。  
  Local `.wkp` plugins are still supported through the runtime manager, including auto-download, `${os}/${arch}` templating, Unix-socket IPC, and lifecycle supervision—without额外的 HTTP 插件 API 依赖。

### Webhook 事件通知
- **客户端上线/离线事件**：实时通知第三方系统
- **消息发送/送达事件**：消息生命周期事件通知
- **失败事件通知**：消息发送失败的事件通知
- **签名验证**：支持 HMAC-SHA256 签名验证
- **重试机制**：可配置的重试次数和间隔

### 健康检查
- **基础健康检查**：服务存活状态
- **详细健康检查**：包含在线客户端数量等详细信息
- **就绪状态检查**：服务是否准备好接收请求
- **存活状态检查**：服务是否正常运行

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- 系统支持：Linux, macOS, Windows

### 编译运行

```bash
# 克隆项目
git clone <repository-url>
cd v-connect-im

# 编译项目
cargo build --release

# 运行服务器（默认配置）
cargo run
```

### 命令行参数

```bash
# 自定义主机和端口
cargo run -- --host 0.0.0.0 --ws-port 5200 --http-port 8080 --timeout-ms 10000

# 启用 Webhook 通知
cargo run -- --webhook-url http://your-webhook-server/events --webhook-secret your-secret-key

# 查看帮助信息
cargo run -- --help
```

#### 参数说明
- `--host`: 服务器主机地址 (默认: 127.0.0.1)
- `--ws-port`: WebSocket 端口 (默认: 5200)
- `--http-port`: HTTP API 端口 (默认: 8080)
- `--timeout-ms`: 客户端超时时间，毫秒 (默认: 10000)
- `--webhook-url`: Webhook 事件通知URL
- `--webhook-timeout-ms`: Webhook 请求超时时间，毫秒 (默认: 3000)
- `--webhook-secret`: Webhook 签名密钥

## 📡 消息协议

### WebSocket 消息格式

所有消息采用 JSON 格式，结构如下：

```json
{
    "type": "message_type",
    "data": { /* 消息数据 */ },
    "target_id": "可选的目标客户端ID"
}
```

### 支持的消息类型

#### 客户端 → 服务器
- `ping`: 心跳检测
- `auth`: 身份认证
- `message`: 普通消息（可指定目标）
- `private_message`: 私聊消息（必须指定目标）
- `online_clients`: 查询在线客户端列表

#### 服务器 → 客户端
- `pong`: 心跳响应
- `auth_response`: 认证响应
- `message_echo`: 消息回声
- `forwarded_message`: 转发消息
- `private_message`: 私聊消息
- `message_sent`: 消息发送确认
- `online_clients_response`: 在线客户端列表
- `error`: 错误信息

### 连接响应格式

```json
{
    "status": "connected",
    "message": "Welcome to v-connect-im Server",
    "client_id": "生成的唯一客户端ID"
}
```

## 💻 使用示例

### WebSocket 客户端示例

```javascript
// 连接服务器
const ws = new WebSocket('ws://localhost:5200');

ws.onopen = function() {
    console.log('✅ 已连接到服务器');
    
    // 发送认证
    ws.send(JSON.stringify({
        type: 'auth',
        data: {
            uid: 'user123',
            token: 'token123'
        }
    }));
    
    // 发送心跳
    ws.send(JSON.stringify({
        type: 'ping',
        data: {}
    }));
    
    // 发送普通消息
    ws.send(JSON.stringify({
        type: 'message',
        data: {
            content: 'Hello v-connect-im!'
        }
    }));
    
    // 发送私聊消息
    ws.send(JSON.stringify({
        type: 'private_message',
        data: {
            content: 'Hello privately!'
        },
        target_id: 'target-client-id'
    }));
    
    // 查询在线客户端
    ws.send(JSON.stringify({
        type: 'online_clients',
        data: {}
    }));
};

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    console.log('📨 收到消息:', message);
};

ws.onerror = function(error) {
    console.error('❌ WebSocket错误:', error);
};
```

### HTTP API 使用示例

#### 发送点对点消息
```bash
curl -X POST http://localhost:8080/api/send \
  -H "Content-Type: application/json" \
  -d '{
    "from_client_id": "sender-client-id",
    "to_client_id": "receiver-client-id",
    "content": {"text": "Hello via HTTP!"},
    "message_type": "http_message"
  }'
```

#### 广播消息给所有客户端
```bash
curl -X POST http://localhost:8080/api/broadcast \
  -H "Content-Type: application/json" \
  -d '{
    "from_client_id": "sender-client-id",
    "content": {"text": "Broadcast message!"},
    "message_type": "http_broadcast"
  }'
```

### 健康检查接口

```bash
# 基础健康检查
curl http://localhost:8080/health

# 详细健康检查（包含在线客户端数量）
curl http://localhost:8080/health/detailed

# 就绪状态检查
curl http://localhost:8080/health/ready

# 存活状态检查
curl http://localhost:8080/health/live
```

## 🔧 Webhook 事件通知

### 事件类型
- `client_online`: 客户端上线
- `client_offline`: 客户端离线
- `message_sent`: 消息已发送
- `message_delivered`: 消息已送达
- `message_failed`: 消息发送失败

### Webhook 载荷格式

```json
{
    "event_type": "client_online",
    "event_id": "唯一事件ID",
    "timestamp": 事件时间戳,
    "data": {
        // 事件具体数据
    },
    "retry_count": 0
}
```

### 签名验证

如果配置了 `webhook-secret`，服务器会在请求头中包含签名：
```
X-VConnectIM-Signature: sha256=<签名值>
```

签名生成方式：HMAC-SHA256(event_id + event_type + timestamp)

## 🏗️ 系统架构

### 核心组件

1. **VConnectIMServer**: 主服务器，管理所有连接和业务逻辑
2. **连接管理器**: 使用 DashMap 管理并发客户端连接
3. **消息处理器**: 处理不同类型的消息协议
4. **心跳管理器**: 自动处理客户端心跳和超时清理
5. **Webhook 客户端**: 异步发送事件通知到第三方系统

### 技术栈

- **异步运行时**: Tokio - 高性能异步 Rust 运行时
- **WebSocket**: tokio-tungstenite - 异步 WebSocket 实现
- **HTTP 框架**: Axum - 现代异步 Web 框架
- **并发集合**: DashMap - 高性能并发哈希表
- **序列化**: Serde - Rust 序列化框架
- **日志**: Tracing - 结构化日志和诊断
- **HTTP 客户端**: Reqwest - 异步 HTTP 客户端
- **加密**: HMAC-SHA256 - Webhook 签名验证

### 项目结构

```
wukongim-server/
├── Cargo.toml          # 项目依赖配置
├── src/
│   └── main.rs         # 主服务器代码
└── README.md           # 项目文档
```

## 📊 性能特点

- **高并发**: 基于 Tokio 异步运行时，支持大量并发连接
- **内存安全**: Rust 的所有权系统保证内存安全
- **零成本抽象**: 高性能的抽象，无运行时开销
- **自动资源管理**: 智能的连接清理和资源回收
- **异步 I/O**: 非阻塞的网络 I/O 操作

## 🔍 监控与调试

### 日志输出示例

```
🎯 Starting VConnectIM Hybrid Server (WebSocket + HTTP)...
📋 Configuration:
   Host: 127.0.0.1
   WebSocket Port: 5200
   HTTP Port: 8080
   Client Timeout: 10000ms
📡 Webhook: Disabled

📖 WebSocket message types:
   - ping: Heartbeat (with automatic heartbeat tracking)
   - auth: Authentication
   - message: Send message with optional target_id
   - private_message: Send private message (requires target_id)
   - online_clients: Query online clients list

🚀 Starting WebSocket server on 127.0.0.1:5200
🌐 Starting HTTP server on 127.0.0.1:8080
📨 New connection from: 127.0.0.1:54321
✅ Client 550e8400-e29b-41d4-a716-446655440000 connected from 127.0.0.1:54321
🏓 Ping from 550e8400-e29b-41d4-a716-446655440000
💬 Message from 550e8400-e29b-41d4-a716-446655440000: {"content":"Hello"}
👋 Client 550e8400-e29b-41d4-a716-446655440000 disconnected
```

### 调试建议

1. **使用 DEBUG 日志级别**: 设置 `RUST_LOG=debug` 环境变量
2. **监控连接数**: 通过 `/health/detailed` 接口监控在线客户端
3. **Webhook 测试**: 使用 webhook 测试工具验证事件通知
4. **性能分析**: 使用 Rust 的性能分析工具进行优化

## 🚀 生产环境建议

### 必需功能
- **TLS/SSL 支持**: 配置 HTTPS/WSS 加密传输
- **身份认证**: 实现真实的用户认证机制
- **消息持久化**: 添加消息存储和离线消息支持
- **用户管理**: 完整的用户注册、登录、权限管理

### 性能优化
- **连接池**: 数据库连接池管理
- **消息队列**: 异步消息处理队列
- **缓存机制**: 热点数据缓存
- **负载均衡**: 多服务器负载均衡部署

### 监控运维
- **指标收集**: Prometheus 指标暴露
- **链路追踪**: 分布式链路追踪
- **错误报警**: 异常情况自动报警
- **日志收集**: 结构化日志集中收集

## 📄 许可证

本项目是 v-connect-im 即时通讯系统在 Rust 语言中的实现版本。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目！

## 🆘 支持

如遇到问题，请通过以下方式获取支持：
1. 查看项目文档和示例代码
2. 在 Issue 区提交问题
3. 查看运行日志进行调试