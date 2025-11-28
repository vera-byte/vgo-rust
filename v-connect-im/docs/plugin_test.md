# 插件测试功能文档 / Plugin Test Feature Documentation

## 概述 / Overview

测试插件 (`TestPlugin`) 是一个内置插件，用于验证插件系统的各项功能。它提供了完整的测试工具和 API，可以用于：

- 验证插件生命周期钩子
- 测试消息拦截和处理
- 验证自定义事件处理
- 测试插件配置更新
- 验证插件数据存储

## 功能特性 / Features

### 1. 消息计数 / Message Counting

测试插件会自动统计：
- **接收消息数** (`incoming_count`) - 通过 `on_message_incoming` 处理的消息数量
- **发送消息数** (`outgoing_count`) - 通过 `on_message_outgoing` 处理的消息数量

### 2. 消息拦截 / Message Blocking

测试插件支持消息拦截功能：
- 可以设置 `should_block` 标志来阻止消息继续处理
- 当设置为 `true` 时，`on_message_incoming` 会返回 `PluginFlow::Stop`

### 3. 自定义事件记录 / Custom Event Recording

测试插件会记录所有接收到的自定义事件：
- 事件类型 (`event_type`)
- 事件负载 (`payload`)
- 时间戳 (`timestamp`)

### 4. 测试数据存储 / Test Data Storage

测试插件可以存储和检索测试数据：
- 自动记录消息处理数据
- 记录生命周期事件数据（startup、config_update、shutdown）
- 支持通过 API 查询特定键值

### 5. 统计信息 / Statistics

测试插件提供完整的统计信息，包括：
- 消息计数
- 自定义事件数量
- 阻止状态
- 所有测试数据键列表

## API 接口 / API Endpoints

### 1. 获取统计信息 / Get Statistics

```http
GET /v1/plugins/test/stats
```

**响应示例 / Response Example:**
```json
{
  "incoming_count": 10,
  "outgoing_count": 8,
  "custom_events_count": 5,
  "should_block": false,
  "test_data_keys": ["incoming_1", "outgoing_1", "startup", "config_update"]
}
```

### 2. 重置测试插件 / Reset Test Plugin

```http
POST /v1/plugins/test/reset
```

**响应示例 / Response Example:**
```json
{
  "status": "reset",
  "message": "Test plugin reset successfully"
}
```

### 3. 设置阻止模式 / Set Block Mode

```http
POST /v1/plugins/test/block
Content-Type: application/json

{
  "block": true
}
```

**响应示例 / Response Example:**
```json
{
  "status": "updated",
  "block": true
}
```

### 4. 获取测试数据 / Get Test Data

**获取所有数据 / Get All Data:**
```http
GET /v1/plugins/test/data
```

**获取特定键值 / Get Specific Key:**
```http
GET /v1/plugins/test/data?key=incoming_1
```

**响应示例 / Response Example:**
```json
{
  "key": "incoming_1",
  "value": {
    "client_id": "client_123",
    "msg_type": "text",
    "timestamp": 1234567890
  }
}
```

## 使用示例 / Usage Examples

### 1. 测试消息处理 / Test Message Processing

```bash
# 1. 获取初始统计信息
curl http://localhost:8080/v1/plugins/test/stats

# 2. 发送一些消息（通过 WebSocket 或其他方式）
# ... 发送消息 ...

# 3. 再次获取统计信息，查看计数是否增加
curl http://localhost:8080/v1/plugins/test/stats
```

### 2. 测试消息拦截 / Test Message Blocking

```bash
# 1. 启用阻止模式
curl -X POST http://localhost:8080/v1/plugins/test/block \
  -H "Content-Type: application/json" \
  -d '{"block": true}'

# 2. 发送消息（应该被阻止）
# ... 发送消息 ...

# 3. 检查统计信息（incoming_count 会增加，但消息不会继续处理）
curl http://localhost:8080/v1/plugins/test/stats

# 4. 禁用阻止模式
curl -X POST http://localhost:8080/v1/plugins/test/block \
  -H "Content-Type: application/json" \
  -d '{"block": false}'
```

### 3. 测试自定义事件 / Test Custom Events

```bash
# 1. 触发一些自定义事件（例如：连接建立、用户上线等）
# ... 触发事件 ...

# 2. 获取统计信息，查看自定义事件数量
curl http://localhost:8080/v1/plugins/test/stats

# 3. 获取特定事件数据
curl "http://localhost:8080/v1/plugins/test/data?key=custom_1"
```

### 4. 测试生命周期钩子 / Test Lifecycle Hooks

```bash
# 1. 检查启动数据
curl "http://localhost:8080/v1/plugins/test/data?key=startup"

# 2. 更新配置（会触发 on_config_update）
curl -X PUT http://localhost:8080/v1/plugins/config \
  -H "Content-Type: application/json" \
  -d '{"test": "value"}'

# 3. 检查配置更新数据
curl "http://localhost:8080/v1/plugins/test/data?key=config_update"
```

### 5. 重置测试数据 / Reset Test Data

```bash
# 重置所有测试数据
curl -X POST http://localhost:8080/v1/plugins/test/reset

# 验证重置（所有计数应该为 0）
curl http://localhost:8080/v1/plugins/test/stats
```

## 测试场景 / Test Scenarios

### 场景 1: 验证插件优先级 / Verify Plugin Priority

测试插件具有高优先级（priority = 1），应该在其他插件之前执行。可以通过以下方式验证：

1. 启用测试插件的阻止模式
2. 发送消息
3. 验证消息被阻止，其他插件不会收到该消息

### 场景 2: 验证消息流控制 / Verify Message Flow Control

1. 设置 `should_block = true`
2. 发送消息
3. 验证 `on_message_incoming` 返回 `PluginFlow::Stop`
4. 验证消息计数增加，但消息不会继续处理

### 场景 3: 验证生命周期钩子 / Verify Lifecycle Hooks

1. 启动服务器（触发 `on_startup`）
2. 更新配置（触发 `on_config_update`）
3. 关闭服务器（触发 `on_shutdown`）
4. 验证每个钩子都记录了相应的数据

### 场景 4: 验证自定义事件 / Verify Custom Events

1. 触发各种自定义事件（连接、用户、房间等）
2. 验证事件被正确记录
3. 验证事件数据包含正确的类型和负载

## 集成测试 / Integration Testing

测试插件可以用于集成测试，验证整个插件系统的工作流程：

```bash
#!/bin/bash
# 集成测试脚本 / Integration test script

BASE_URL="http://localhost:8080"

# 1. 重置测试插件
echo "Resetting test plugin..."
curl -X POST "$BASE_URL/v1/plugins/test/reset"

# 2. 发送测试消息
echo "Sending test messages..."
# ... 发送消息的代码 ...

# 3. 验证统计信息
echo "Checking statistics..."
STATS=$(curl -s "$BASE_URL/v1/plugins/test/stats")
echo "$STATS"

# 4. 验证消息计数
INCOMING=$(echo "$STATS" | jq -r '.incoming_count')
if [ "$INCOMING" -gt "0" ]; then
    echo "✓ Message processing verified"
else
    echo "✗ Message processing failed"
    exit 1
fi

# 5. 测试消息拦截
echo "Testing message blocking..."
curl -X POST "$BASE_URL/v1/plugins/test/block" \
  -H "Content-Type: application/json" \
  -d '{"block": true}'

# ... 发送消息并验证被阻止 ...

# 6. 清理
curl -X POST "$BASE_URL/v1/plugins/test/block" \
  -H "Content-Type: application/json" \
  -d '{"block": false}'
curl -X POST "$BASE_URL/v1/plugins/test/reset"
```

## 注意事项 / Notes

1. **测试插件是内置插件**：测试插件在服务器启动时自动注册，无需手动配置
2. **数据持久化**：测试数据存储在内存中，服务器重启后会丢失
3. **性能影响**：测试插件会记录所有消息和事件，可能对性能有轻微影响
4. **生产环境**：建议在生产环境中禁用测试插件或仅用于调试

## 相关文件 / Related Files

- `src/plugins/test.rs` - 测试插件实现
- `src/api/plugins.rs` - 测试 API 接口
- `src/main.rs` - 测试插件初始化

## 总结 / Summary

测试插件提供了完整的插件系统测试工具，可以用于：
- 验证插件功能
- 调试插件问题
- 性能测试
- 集成测试

通过测试插件的 API，可以方便地监控和验证插件系统的各项功能。


