# v-connect-im 存储插件 / v-connect-im Storage Plugin

基于 Sled 实现的高性能存储插件，为 v-connect-im 提供消息持久化、离线消息管理、房间成员管理等功能。

High-performance storage plugin based on Sled, providing message persistence, offline message management, room member management for v-connect-im.

## 功能特性 / Features

- ✅ **消息持久化** / Message Persistence - 将所有消息保存到 WAL
- ✅ **离线消息管理** / Offline Message Management - 存储和管理用户离线消息
- ✅ **房间成员管理** / Room Member Management - 管理聊天室成员关系
- ✅ **已读回执存储** / Read Receipt Storage - 记录消息已读状态
- ✅ **高性能** / High Performance - 基于 Sled 嵌入式数据库
- ✅ **配置灵活** / Flexible Configuration - 支持动态配置更新
- ✅ **统计信息** / Statistics - 提供详细的存储统计数据
- ✅ **能力声明** / Capability Declaration - 声明 `storage` 能力以接收存储事件

## 快速开始 / Quick Start

### 1. 编译插件 / Build Plugin

```bash
cd /Users/mac/workspace/v-connect-im-plugin-storage
cargo build --release
```

### 2. 配置服务器 / Configure Server

在 `v-connect-im/config/default.toml` 中添加：

```toml
[plugins]
socket_path = "~/vp/sockets/runtime.sock"
debug = true

# 开发模式 / Development mode
dev_plugins = [
    "storage:/Users/mac/workspace/v-connect-im-plugin-storage",
]
```

### 3. 启动服务器 / Start Server

```bash
cd /Users/mac/workspace/vgo-rust/v-connect-im
cargo run
```

插件会自动启动并连接到服务器。

## 支持的事件 / Supported Events

### 消息存储 / Message Storage

#### `storage.message.save`
保存消息到 WAL

**载荷 / Payload**:
```json
{
  "message_id": "uuid",
  "from_uid": "user1",
  "to_uid": "user2",
  "content": {"text": "Hello"},
  "timestamp": 1701619200000,
  "msg_type": "message"
}
```

### 离线消息 / Offline Messages

#### `storage.offline.save`
保存离线消息

#### `storage.offline.pull`
拉取离线消息

**载荷 / Payload**:
```json
{
  "to_uid": "user2",
  "limit": 100
}
```

**响应 / Response**:
```json
{
  "status": "ok",
  "messages": [...],
  "count": 10
}
```

#### `storage.offline.ack`
确认已读离线消息

**载荷 / Payload**:
```json
{
  "to_uid": "user2",
  "message_ids": ["uuid1", "uuid2"]
}
```

#### `storage.offline.count`
统计离线消息数量

### 房间管理 / Room Management

#### `storage.room.add_member`
添加房间成员

**载荷 / Payload**:
```json
{
  "room_id": "room123",
  "uid": "user1"
}
```

#### `storage.room.remove_member`
移除房间成员

#### `storage.room.list_members`
列出房间成员

**响应 / Response**:
```json
{
  "status": "ok",
  "members": ["user1", "user2"],
  "count": 2
}
```

#### `storage.room.list`
列出所有房间

### 已读回执 / Read Receipts

#### `storage.read.record`
记录已读回执

### 统计信息 / Statistics

#### `storage.stats`
查询存储统计信息

**响应 / Response**:
```json
{
  "status": "ok",
  "stats": {
    "messages_saved": 1000,
    "offline_saved": 50,
    "offline_pulled": 30,
    "offline_acked": 20,
    "db_size": 1048576
  }
}
```

## 配置选项 / Configuration Options

```json
{
  "db_path": "./data/plugin-storage",
  "max_offline_messages": 10000,
  "enable_compression": false
}
```

### 配置说明 / Configuration Description

- **db_path**: 数据库文件路径 / Database file path
- **max_offline_messages**: 每个用户的最大离线消息数 / Max offline messages per user
- **enable_compression**: 是否启用压缩（未实现）/ Enable compression (not implemented)

## 数据结构 / Data Structure

### 数据库树 / Database Trees

- **wal**: 消息 WAL，键格式 `timestamp:message_id`
- **offline**: 离线消息，键格式 `to_uid:timestamp:message_id`
- **room_members**: 房间成员，键格式 `room_id:uid`
- **reads**: 已读回执，键格式 `uid:message_id`

## 能力声明 / Capability Declaration

### 重要：必须声明 `storage` 能力

存储插件**必须**在 `capabilities()` 方法中声明 `storage` 能力，否则服务器无法识别该插件为存储插件。

**代码示例 / Code Example**:
```rust
impl Plugin for StoragePlugin {
    // ... 其他方法

    /// 声明插件能力 / Declare plugin capabilities
    fn capabilities(&self) -> Vec<String> {
        vec!["storage".into()]  // ⚠️ 必须声明 / Must declare
    }
}
```

### 工作原理 / How It Works

1. **插件启动时**：插件在握手阶段向服务器发送能力列表
2. **服务器识别**：服务器检查插件是否声明了 `storage` 能力
3. **事件路由**：只有声明了 `storage` 能力的插件才会接收存储相关事件
4. **查找机制**：`PluginConnectionPool::send_storage_event()` 会查找第一个支持 `storage` 能力的插件

### 注意事项 / Notes

- ⚠️ 如果忘记声明 `storage` 能力，插件将不会接收任何存储事件
- ⚠️ 同时只应有一个存储插件声明 `storage` 能力
- ✅ 可以与其他能力组合，如 `vec!["storage".into(), "message".into()]`

## 性能特性 / Performance Features

- 🚀 **高性能写入** - Sled 提供快速的写入性能
- 💾 **低内存占用** - 嵌入式数据库，无需额外进程
- 🔄 **自动刷盘** - 确保数据持久化
- 📊 **前缀扫描** - 高效的范围查询
- 🗜️ **自动压缩** - Sled 自动进行数据压缩

## 开发 / Development

### 运行测试 / Run Tests

```bash
cargo test
```

### 调试模式 / Debug Mode

```bash
cargo run -- --socket ~/vp/sockets/runtime.sock --debug --log-level debug
```

### 查看日志 / View Logs

插件会输出详细的日志信息：

```
🗄️  初始化存储插件 / Initializing Storage Plugin
✅ 存储插件初始化完成 / Storage Plugin initialized
📁 数据库路径 / Database path: ./data/plugin-storage
💾 保存消息 / Saving message: xxx at 1701619200000
✅ 消息已保存 / Message saved: xxx
```

## 故障排查 / Troubleshooting

### 问题：插件无法启动

**检查**:
1. 数据库路径是否有写权限
2. Socket 路径是否正确
3. 是否有端口冲突

### 问题：消息没有保存

**检查**:
1. 插件是否成功注册
2. 能力声明是否包含 `storage`
3. 查看插件日志

### 问题：离线消息达到上限

**解决方案**:
- 增加 `max_offline_messages` 配置
- 定期清理旧的离线消息
- 提醒用户及时拉取消息

## 最佳实践 / Best Practices

1. **定期备份数据** - 定期备份 `data/plugin-storage` 目录
2. **监控磁盘空间** - 确保有足够的磁盘空间
3. **合理设置限制** - 根据实际情况设置离线消息上限
4. **性能监控** - 定期查询统计信息监控性能

## 许可证 / License

MIT

## 相关链接 / Related Links

- [v-connect-im](https://github.com/vera-byte/vgo-rust)
- [Sled Database](https://github.com/spacejam/sled)
- [插件开发文档](https://docs.example.com/plugin)
