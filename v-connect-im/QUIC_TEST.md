# QUIC 测试指南

本文档介绍如何测试 v-connect-im 的 QUIC 功能。

## 前置要求

1. **启用 QUIC 特性编译**
   - QUIC 功能需要启用 `quic` 特性标志
   - 确保 `quiche` 依赖已正确配置

2. **TLS 证书**
   - QUIC 需要 TLS 证书用于加密连接
   - 证书文件路径：`certs/server.crt` 和 `certs/server.key`

## 快速开始

### 1. 生成测试用 TLS 证书

首先需要生成自签名证书用于测试：

```bash
cd v-connect-im

# 创建 certs 目录（如果不存在）
mkdir -p certs

# 生成私钥
openssl genrsa -out certs/server.key 2048

# 生成证书签名请求
openssl req -new -key certs/server.key -out certs/server.csr \
  -subj "/C=CN/ST=State/L=City/O=Organization/CN=localhost"

# 生成自签名证书（有效期365天）
openssl x509 -req -days 365 -in certs/server.csr \
  -signkey certs/server.key -out certs/server.crt

# 清理临时文件
rm certs/server.csr
```

**注意**：生产环境请使用由 CA 签发的正式证书。

### 2. 配置 QUIC 服务器

确保 `config/default.toml` 中 QUIC 配置已启用：

```toml
[quic]
enabled = 1
host = "0.0.0.0"
port = 5201
tls_cert = "certs/server.crt"
tls_key = "certs/server.key"
```

### 3. 编译并启动服务器（启用 QUIC 特性）

```bash
# 在项目根目录编译（启用 quic 特性）
cargo build --release --features quic

# 或者直接运行
cargo run --release --features quic

# 使用自定义配置文件
cargo run --release --features quic -- --config config/default.toml
```

启动成功后，你应该看到类似输出：

```
🟢 QUIC listening at 0.0.0.0:5201
🚀 v-connect-im WebSocket Server starting on 0.0.0.0:5200
```

### 4. 运行 QUIC 客户端测试

项目提供了一个 QUIC 客户端示例，用于测试连接：

```bash
# 在 v-connect-im 目录下运行示例
cargo run --release --features quic --example quic_client

# 指定服务器地址（默认 127.0.0.1:5201）
PEER=127.0.0.1:5201 cargo run --release --features quic --example quic_client
```

客户端示例会：
1. 建立 QUIC 连接
2. 发送认证消息（`auth`）
3. 发送心跳消息（`ping`）
4. 接收服务器响应
5. 测试连接迁移功能

## 测试步骤详解

### 测试 1: 基本连接和认证

客户端示例会自动执行以下流程：

1. **建立连接**：客户端发起 QUIC 握手
2. **发送认证**：连接建立后发送认证消息
   ```json
   {
     "type": "auth",
     "data": {
       "uid": "uClient",
       "token": "dummy"
     }
   }
   ```
3. **接收认证响应**：服务器返回 `auth_ok`
4. **发送心跳**：发送 `ping` 消息
5. **接收响应**：接收服务器的 `pong` 响应

### 测试 2: 连接迁移

客户端示例会测试 QUIC 的连接迁移功能：

1. 建立初始连接
2. 切换到新的本地端口
3. 继续发送数据
4. 验证连接在迁移后仍然可用

### 测试 3: 消息发送

你可以修改客户端示例来测试消息发送：

```rust
// 在 quic_client.rs 中添加消息发送
let message = serde_json::json!({
    "type": "message",
    "data": {
        "content": "Hello from QUIC!"
    }
});
let _ = conn.stream_send(2, message.to_string().as_bytes(), true);
```

## 使用自定义客户端测试

### 使用 curl 测试（需要支持 QUIC 的版本）

```bash
# curl 7.66.0+ 支持 QUIC
curl --http3 https://localhost:5201/health
```

### 使用 quiche 客户端工具

```bash
# 安装 quiche 客户端工具（如果可用）
# 然后使用它连接服务器
```

### 编写自定义测试客户端

参考 `examples/quic_client.rs` 编写自己的测试客户端：

```rust
use quiche::Config;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 配置 QUIC 客户端
    let mut config = Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.set_application_protos(&[b"wukong-msg"]).unwrap();
    config.verify_peer(false); // 测试环境关闭证书验证
    
    // 建立连接并测试...
}
```

## 调试技巧

### 1. 启用详细日志

```bash
# 设置日志级别为 debug
RUST_LOG=debug cargo run --release --features quic
```

### 2. 检查证书路径

确保证书文件存在且路径正确：

```bash
ls -la v-connect-im/certs/
# 应该看到 server.crt 和 server.key
```

### 3. 检查端口占用

```bash
# Linux/macOS
lsof -i :5201
netstat -an | grep 5201

# 或使用 ss
ss -ulnp | grep 5201
```

### 4. 使用 Wireshark 抓包

QUIC 使用 UDP，可以使用 Wireshark 抓包分析：

1. 启动 Wireshark
2. 过滤 UDP 端口 5201
3. 分析 QUIC 握手和数据传输

### 5. 常见问题排查

**问题：证书加载失败**
```
QUIC bind error: ...
```
- 检查证书文件是否存在
- 检查证书文件权限
- 验证证书格式是否正确

**问题：连接被拒绝**
```
accept err: ...
```
- 确认服务器已启动
- 检查防火墙设置
- 验证端口配置

**问题：认证超时**
```
disconnecting unauthenticated QUIC client_id=...
```
- 检查 `auth.deadline_ms` 配置
- 确保客户端在超时前发送认证消息

## 性能测试

### 测试并发连接

可以编写脚本测试多个并发 QUIC 连接：

```bash
# 启动 10 个并发客户端
for i in {1..10}; do
    PEER=127.0.0.1:5201 cargo run --release --features quic --example quic_client &
done
wait
```

### 监控服务器指标

通过 HTTP API 查看服务器状态：

```bash
# 查看健康检查
curl http://localhost:8080/health/detailed
```

## 生产环境注意事项

1. **使用正式证书**：不要使用自签名证书
2. **启用证书验证**：客户端应验证服务器证书
3. **配置超时**：根据网络环境调整超时参数
4. **监控连接数**：监控 QUIC 连接数量和状态
5. **日志记录**：记录连接、认证、消息等关键事件

## 相关文档

- [quiche 文档](https://docs.rs/quiche/)
- [QUIC 协议规范](https://datatracker.ietf.org/doc/html/rfc9000)
- 项目 README.md

## 故障排除

如果遇到问题，请检查：

1. ✅ QUIC 特性是否已启用（`--features quic`）
2. ✅ TLS 证书是否存在且有效
3. ✅ 配置文件中的 QUIC 设置是否正确
4. ✅ 端口是否被占用
5. ✅ 防火墙是否允许 UDP 流量
6. ✅ 日志输出中的错误信息

