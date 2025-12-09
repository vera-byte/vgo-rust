# v-connect-im-plugin-gateway

HTTP API 网关插件，为 v-connect-im 提供 RESTful API 接口服务。

HTTP API Gateway plugin that provides RESTful API interface services for v-connect-im.

## 功能特性 / Features

- ✅ HTTP API 服务 / HTTP API Service
- ✅ 路由管理 / Route Management
- ✅ OpenAPI 文档 / OpenAPI Documentation
- ✅ 健康检查 / Health Check
- ✅ 消息发送接口 / Message Sending API
- ✅ 房间管理接口 / Room Management API
- ✅ 连接管理接口 / Connection Management API
- ✅ 离线消息接口 / Offline Message API

## 架构设计 / Architecture

本插件采用插件化架构，将 HTTP API 服务从主服务中分离：

This plugin uses a plugin-based architecture to separate HTTP API service from the main service:

- 独立的 HTTP 服务器 / Independent HTTP server
- 通过插件通信协议与主服务交互 / Interact with main service via plugin communication protocol
- 支持动态路由注册 / Support dynamic route registration
- 可独立部署和扩展 / Can be deployed and scaled independently

## API 接口分组 / API Groups

### 健康检查 / Health Check
- `GET /health` - 基础健康检查
- `GET /health/live` - 存活检查
- `GET /health/ready` - 就绪检查
- `GET /health/detailed` - 详细健康信息

### 消息接口 / Message API
- `POST /v1/message/send` - 发送消息
- `POST /v1/message/broadcast` - 广播消息
- `GET /v1/message/history` - 消息历史
- `POST /v1/message/read` - 标记已读
- `GET /v1/message/status` - 消息状态

### 房间接口 / Room API
- `POST /v1/room/create` - 创建房间
- `POST /v1/room/join` - 加入房间
- `POST /v1/room/leave` - 离开房间
- `GET /v1/room/list` - 房间列表
- `GET /v1/room/members` - 房间成员
- `POST /v1/room/send` - 房间消息

### 连接接口 / Connection API
- `GET /v1/connection/list` - 连接列表
- `GET /v1/connection/ws_by_uid` - 按UID查询连接

### 离线消息 / Offline Message
- `GET /v1/offline/pull` - 拉取离线消息
- `POST /v1/offline/ack` - 确认离线消息
- `POST /v1/offline/cleanup` - 清理离线消息

### 管理接口 / Admin API
- `POST /v1/admin/uid/block` - 封禁用户
- `POST /v1/admin/uid/rate_limit` - 限流设置
- `GET /v1/admin/quic/stats` - QUIC统计

## 配置说明 / Configuration

```json
{
  "host": "0.0.0.0",
  "port": 8080,
  "workers": 4,
  "enable_openapi": true
}
```

## 使用方法 / Usage

### 编译 / Build

```bash
cd v-plugins-hub/v-connect-im-plugin-gateway
cargo build --release
```

### 运行 / Run

```bash
./target/release/v-connect-im-plugin-gateway
```

## 开发指南 / Development Guide

### 添加新接口 / Adding New API

1. 在 `src/api/v1/` 下创建对应的模块
2. 实现路由注册函数 `register()`
3. 实现处理函数
4. 在 `src/router.rs` 中注册路由

### 与主服务通信 / Communication with Main Service

插件通过以下方式与主服务通信：

The plugin communicates with the main service through:

- 发送请求到主服务的内部接口 / Send requests to main service's internal API
- 接收主服务的事件通知 / Receive event notifications from main service
- 使用共享的数据结构 / Use shared data structures

## 许可证 / License

MIT
