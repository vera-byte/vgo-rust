# 插件间通信功能测试指南 / Plugin Inter-Communication Test Guide

## 概述 / Overview

本文档提供插件间通信功能的完整测试指南，包括单元测试、集成测试和手动测试。

This document provides a complete testing guide for inter-plugin communication features, including unit tests, integration tests, and manual tests.

---

## 🧪 测试环境准备 / Test Environment Setup

### 1. 启动服务器 / Start Server

```bash
# 编译项目 / Build project
cargo build --release

# 启动服务器 / Start server
cargo run -- --config config/default.toml
```

### 2. 启动测试插件 / Start Test Plugins

需要至少两个插件来测试插件间通信：
You need at least two plugins to test inter-plugin communication:

```bash
# 终端 1: 启动插件 A / Terminal 1: Start Plugin A
cd /path/to/plugin-a
cargo run

# 终端 2: 启动插件 B / Terminal 2: Start Plugin B
cd /path/to/plugin-b
cargo run
```

---

## 📝 测试用例 / Test Cases

### 测试 1: 插件 RPC 调用 / Test 1: Plugin RPC Call

#### 目标 / Objective
验证插件 A 可以直接调用插件 B 的方法并获取返回值。
Verify that Plugin A can directly call Plugin B's method and get the return value.

#### 测试步骤 / Test Steps

1. **发送 RPC 调用请求 / Send RPC Call Request**

```bash
curl -X POST http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "example",
    "to_plugin": "storage-sled",
    "method": "get_stats",
    "params": {}
  }'
```

2. **预期响应 / Expected Response**

```json
{
  "status": "ok",
  "response": {
    "message_count": 100,
    "storage_size": 1024000
  },
  "error": null
}
```

3. **验证点 / Verification Points**
   - ✅ 响应状态为 "ok"
   - ✅ response 字段包含插件 B 的返回数据
   - ✅ 服务器日志显示调用成功

#### 错误场景测试 / Error Scenario Tests

**场景 1: 目标插件未连接 / Target Plugin Not Connected**

```bash
curl -X POST http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "example",
    "to_plugin": "non-existent-plugin",
    "method": "test",
    "params": {}
  }'
```

预期响应 / Expected Response:
```json
{
  "status": "error",
  "response": null,
  "error": "Target plugin not connected: non-existent-plugin"
}
```

**场景 2: 发送方插件未连接 / Sender Plugin Not Connected**

```bash
curl -X POST http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "non-existent-sender",
    "to_plugin": "storage-sled",
    "method": "test",
    "params": {}
  }'
```

预期响应 / Expected Response:
```json
{
  "status": "error",
  "response": null,
  "error": "Sender plugin not connected: non-existent-sender"
}
```

---

### 测试 2: 插件点对点消息 / Test 2: Plugin P2P Messaging

#### 目标 / Objective
验证插件 A 可以向插件 B 发送消息。
Verify that Plugin A can send message to Plugin B.

#### 测试步骤 / Test Steps

1. **发送点对点消息 / Send P2P Message**

```bash
curl -X PUT http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "example",
    "to_plugin": "storage-sled",
    "message": {
      "type": "notification",
      "content": "Cache invalidated",
      "key": "user:123"
    }
  }'
```

2. **预期响应 / Expected Response**

```json
{
  "status": "ok",
  "delivered": true,
  "error": null
}
```

3. **验证点 / Verification Points**
   - ✅ delivered 字段为 true
   - ✅ 插件 B 的日志显示收到消息
   - ✅ 插件 B 正确处理了消息

---

### 测试 3: 插件广播 / Test 3: Plugin Broadcast

#### 目标 / Objective
验证插件可以向其他插件广播消息，支持能力过滤。
Verify that plugin can broadcast message to other plugins with capability filtering.

#### 测试步骤 / Test Steps

1. **广播给所有插件 / Broadcast to All Plugins**

```bash
curl -X PATCH http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "example",
    "message": {
      "event": "system_update",
      "version": "1.0.1"
    }
  }'
```

2. **广播给特定能力的插件 / Broadcast to Plugins with Specific Capability**

```bash
curl -X PATCH http://localhost:8080/v1/plugins/inter-communication \
  -H "Content-Type: application/json" \
  -d '{
    "from_plugin": "example",
    "message": {
      "event": "data_sync_required"
    },
    "filter_capabilities": ["storage"]
  }'
```

3. **预期响应 / Expected Response**

```json
{
  "status": "ok",
  "response_count": 2,
  "responses": [
    {
      "plugin_name": "storage-sled",
      "response": {
        "status": "ok",
        "synced": true
      }
    },
    {
      "plugin_name": "storage-redis",
      "response": {
        "status": "ok",
        "synced": true
      }
    }
  ]
}
```

4. **验证点 / Verification Points**
   - ✅ response_count 等于实际响应的插件数量
   - ✅ 只有符合能力过滤条件的插件收到消息
   - ✅ 发送方插件不会收到自己的广播

---

### 测试 4: 事件订阅/发布 / Test 4: Event Subscription/Publication

#### 目标 / Objective
验证插件可以订阅事件并接收发布的事件。
Verify that plugins can subscribe to events and receive published events.

#### 测试步骤 / Test Steps

1. **订阅事件 / Subscribe to Event**

```bash
# 插件 A 订阅所有用户事件
curl -X POST http://localhost:8080/v1/plugins/event-bus \
  -H "Content-Type: application/json" \
  -d '{
    "subscriber": "logging-plugin",
    "event_pattern": "user.*",
    "priority": 100
  }'

# 插件 B 订阅登录事件
curl -X POST http://localhost:8080/v1/plugins/event-bus \
  -H "Content-Type: application/json" \
  -d '{
    "subscriber": "statistics-plugin",
    "event_pattern": "user.login",
    "priority": 50
  }'
```

2. **发布事件 / Publish Event**

```bash
curl -X PUT http://localhost:8080/v1/plugins/event-bus \
  -H "Content-Type: application/json" \
  -d '{
    "publisher": "auth-plugin",
    "event_type": "user.login",
    "payload": {
      "user_id": "123",
      "username": "alice",
      "timestamp": 1234567890
    }
  }'
```

3. **预期响应 / Expected Response**

```json
{
  "status": "ok",
  "subscriber_count": 2,
  "responses": [
    {
      "subscriber": "logging-plugin",
      "response": {
        "status": "logged",
        "log_id": "log_001"
      }
    },
    {
      "subscriber": "statistics-plugin",
      "response": {
        "status": "counted",
        "online_users": 42
      }
    }
  ]
}
```

4. **验证点 / Verification Points**
   - ✅ 订阅者按优先级顺序接收事件
   - ✅ 通配符匹配正确工作
   - ✅ 订阅者数量正确

---

## 🔍 性能测试 / Performance Testing

### 测试场景 1: 高频 RPC 调用 / High-Frequency RPC Calls

```bash
# 使用 Apache Bench 进行压力测试
ab -n 1000 -c 10 -p rpc_request.json -T application/json \
  http://localhost:8080/v1/plugins/inter-communication
```

**性能指标 / Performance Metrics:**
- 吞吐量 / Throughput: > 1000 req/s
- 平均延迟 / Average Latency: < 10ms
- 99th 百分位延迟 / 99th Percentile: < 50ms

### 测试场景 2: 大量订阅者 / Large Number of Subscribers

```bash
# 创建 100 个订阅者
for i in {1..100}; do
  curl -X POST http://localhost:8080/v1/plugins/event-bus \
    -H "Content-Type: application/json" \
    -d "{
      \"subscriber\": \"plugin_$i\",
      \"event_pattern\": \"test.*\",
      \"priority\": $i
    }"
done

# 发布事件并测量时间
time curl -X PUT http://localhost:8080/v1/plugins/event-bus \
  -H "Content-Type: application/json" \
  -d '{
    "publisher": "test-publisher",
    "event_type": "test.event",
    "payload": {"data": "test"}
  }'
```

---

## 🐛 调试技巧 / Debugging Tips

### 1. 启用详细日志 / Enable Verbose Logging

```bash
RUST_LOG=debug cargo run
```

### 2. 检查插件连接状态 / Check Plugin Connection Status

```bash
curl http://localhost:8080/v1/plugins/runtime/list
```

### 3. 查看事件历史 / View Event History

```bash
# 需要在代码中启用事件历史
# Need to enable event history in code
event_bus.enable_history(true);
```

### 4. 监控插件通信 / Monitor Plugin Communication

查看服务器日志中的关键信息：
Look for key information in server logs:

```
🔗 插件调用 / Plugin call: plugin_a -> plugin_b (method: test)
✅ 插件调用成功 / Plugin call succeeded: plugin_a -> plugin_b
📨 插件消息 / Plugin message: plugin_a -> plugin_b
📢 插件广播 / Plugin broadcast from: plugin_a
📣 发布事件 / Publish event: plugin_a -> user.login
```

---

## ✅ 测试检查清单 / Test Checklist

### 功能测试 / Functional Tests
- [ ] 插件 RPC 调用成功
- [ ] 插件 RPC 调用失败处理（目标不存在）
- [ ] 插件点对点消息发送
- [ ] 插件广播（无过滤）
- [ ] 插件广播（能力过滤）
- [ ] 事件订阅（精确匹配）
- [ ] 事件订阅（通配符匹配）
- [ ] 事件发布
- [ ] 事件优先级排序

### 错误处理测试 / Error Handling Tests
- [ ] 发送方插件不存在
- [ ] 接收方插件不存在
- [ ] 插件未连接
- [ ] 无效的方法名
- [ ] 无效的事件模式
- [ ] 超时处理

### 性能测试 / Performance Tests
- [ ] 高频 RPC 调用
- [ ] 大量订阅者
- [ ] 大消息传输
- [ ] 并发广播

### 集成测试 / Integration Tests
- [ ] 多插件协作场景
- [ ] 跨节点插件通信（如果支持）
- [ ] 插件热重载后的通信恢复

---

## 📊 测试报告模板 / Test Report Template

```markdown
# 插件间通信测试报告 / Inter-Plugin Communication Test Report

## 测试环境 / Test Environment
- 服务器版本 / Server Version: v0.1.0
- 测试日期 / Test Date: 2025-12-05
- 测试人员 / Tester: [Your Name]

## 测试结果 / Test Results

### 功能测试 / Functional Tests
| 测试项 | 状态 | 备注 |
|--------|------|------|
| RPC 调用 | ✅ | 正常 |
| P2P 消息 | ✅ | 正常 |
| 广播 | ✅ | 正常 |
| 事件订阅/发布 | ⚠️ | 待集成到服务器 |

### 性能测试 / Performance Tests
| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| RPC 吞吐量 | >1000 req/s | 1500 req/s | ✅ |
| 平均延迟 | <10ms | 8ms | ✅ |
| 99th 延迟 | <50ms | 45ms | ✅ |

### 问题列表 / Issues
1. [问题描述]
2. [问题描述]

### 建议 / Recommendations
1. [建议内容]
2. [建议内容]
```

---

## 🚀 快速演示 / Quick Demo

运行内置演示程序：
Run the built-in demo program:

```bash
cargo run --example plugin_communication_demo
```

这将展示所有插件间通信功能的使用示例。
This will demonstrate all inter-plugin communication features.

---

## 📚 参考文档 / References

- [插件间通信功能文档](./plugin_inter_communication.md)
- [插件开发指南](./plugin_dev_guide.md)
- [API 文档](./api_documentation.md)
