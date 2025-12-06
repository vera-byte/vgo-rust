# 插件示例文档 / Plugin Examples Documentation

本目录包含完整的 v-connect-im 插件开发示例。
This directory contains complete plugin development examples for v-connect-im.

---

## 📚 示例列表 / Examples List

### 1. **AI 插件示例** / AI Plugin Example
**文件**: `plugin_ai_example.rs`

一个简单的 AI 对话插件，演示如何处理用户消息并生成智能回复。
A simple AI conversation plugin demonstrating how to handle user messages and generate intelligent replies.

**功能特性 / Features:**
- ✅ 接收用户消息并生成 AI 回复
- ✅ 支持配置自定义 AI 名称和提示词
- ✅ 多种事件类型处理（聊天、补全、摘要）
- ✅ 完整的错误处理和日志记录

**运行方式 / How to Run:**
```bash
cargo run --example plugin_ai_example -- --socket ./plugins/ai.sock
```

**事件类型 / Event Types:**
- `ai.chat` - 聊天对话
- `ai.completion` - 文本补全
- `ai.summarize` - 文本摘要

**配置示例 / Configuration Example:**
```json
{
  "ai_name": "小智助手",
  "system_prompt": "你是一个友好、专业的AI助手",
  "max_reply_length": 500
}
```

---

### 2. **消息过滤插件示例** / Message Filter Plugin Example
**文件**: `plugin_filter_example.rs`

一个消息内容过滤插件，演示如何实现敏感词过滤、垃圾消息检测等功能。
A message content filter plugin demonstrating sensitive word filtering and spam detection.

**功能特性 / Features:**
- ✅ 敏感词过滤和替换
- ✅ 垃圾消息检测（重复字符、全大写、过多感叹号）
- ✅ URL 链接过滤
- ✅ 自定义过滤规则
- ✅ 实时统计信息

**运行方式 / How to Run:**
```bash
cargo run --example plugin_filter_example -- --socket ./plugins/filter.sock
```

**事件类型 / Event Types:**
- `filter.message` - 过滤消息内容
- `filter.check` - 检查内容是否安全
- `filter.stats` - 获取过滤统计信息

**配置示例 / Configuration Example:**
```json
{
  "sensitive_words": ["垃圾", "广告", "spam"],
  "enable_spam_detection": true,
  "enable_url_filter": false,
  "replacement": "*"
}
```

---

### 3. **简化存储插件示例** / Simple Storage Plugin Example
**文件**: `plugin_storage_simple_example.rs`

一个使用内存存储的简化存储插件，演示如何使用 `StorageEventListener` trait。
A simplified storage plugin using in-memory storage, demonstrating how to use the `StorageEventListener` trait.

**功能特性 / Features:**
- ✅ 使用 `StorageEventListener` trait
- ✅ 自动事件分发（零样板代码）
- ✅ 内存存储（HashMap）
- ✅ 完整的存储功能实现

**运行方式 / How to Run:**
```bash
cargo run --example plugin_storage_simple_example -- --socket ./plugins/storage-simple.sock
```

**事件类型 / Event Types:**
所有 `storage.*` 事件都会自动分发到对应的方法：
- `storage.message.save` - 保存消息
- `storage.offline.save` - 保存离线消息
- `storage.offline.pull` - 拉取离线消息
- `storage.offline.ack` - 确认离线消息
- `storage.offline.count` - 统计离线消息
- `storage.room.add_member` - 添加房间成员
- `storage.room.remove_member` - 移除房间成员
- `storage.room.list_members` - 列出房间成员
- `storage.room.list` - 列出所有房间
- `storage.read.record` - 记录已读回执
- `storage.message.history` - 查询历史消息
- `storage.stats` - 获取统计信息

**配置示例 / Configuration Example:**
```json
{
  "max_messages": 1000
}
```

---

## 🎯 插件开发最佳实践 / Plugin Development Best Practices

### 1. **使用 Trait 抽象** / Use Trait Abstraction

对于有标准事件集的插件（如存储插件），使用 trait 可以：
- 零样板代码
- 自动事件分发
- 类型安全
- 易于测试

```rust
use v::plugin::pdk::{Context, Plugin, StorageEventListener};

#[async_trait]
impl StorageEventListener for MyStorageListener {
    async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
        // 实现逻辑
    }
}

// 在 Plugin::receive 中一行搞定
fn receive(&mut self, ctx: &mut Context) -> Result<()> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(self.listener.dispatch(ctx))  // 自动分发！
    })
}
```

### 2. **完善的错误处理** / Robust Error Handling

```rust
// 使用 map_err 提供详细错误信息
let db = sled::open(&config.db_path)
    .map_err(|e| anyhow::anyhow!("无法打开数据库 / Failed to open database: {}", e))?;
```

### 3. **双语注释** / Bilingual Comments

```rust
/// 保存消息到持久化存储 / Save message to persistent storage
///
/// # 参数 / Parameters
/// - `ctx`: 插件上下文 / Plugin context
///
/// # 返回 / Returns
/// - `Result<()>`: 成功或错误 / Success or error
async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
    // ...
}
```

### 4. **合理的日志级别** / Appropriate Log Levels

```rust
use v::{debug, info, warn, error};

debug!("🔍 详细调试信息 / Detailed debug info");
info!("✅ 重要操作完成 / Important operation completed");
warn!("⚠️  警告信息 / Warning message");
error!("❌ 错误信息 / Error message");
```

### 5. **配置管理** / Configuration Management

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyConfig {
    #[serde(default = "default_value")]
    field: String,
}

fn default_value() -> String {
    "default".to_string()
}

impl Default for MyConfig {
    fn default() -> Self {
        Self {
            field: default_value(),
        }
    }
}
```

---

## 🚀 快速开始 / Quick Start

### 1. 创建新插件 / Create New Plugin

```bash
# 在 examples 目录下创建新文件
touch examples/plugin_my_example.rs
```

### 2. 基本结构 / Basic Structure

```rust
use anyhow::Result;
use v::plugin::pdk::{json, Context, Plugin};
use v::info;

struct MyPlugin {
    // 你的字段
}

impl Plugin for MyPlugin {
    type Config = MyConfig;

    fn new() -> Self {
        info!("🚀 初始化插件 / Initializing plugin");
        Self { /* ... */ }
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["my_capability".into()]
    }

    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // 处理事件
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    v::plugin::pdk::run_server::<MyPlugin>("v.plugin.my", "0.1.0", 500).await
}
```

### 3. 编译和运行 / Build and Run

```bash
# 编译
cargo build --example plugin_my_example

# 运行
cargo run --example plugin_my_example -- --socket ./plugins/my.sock --log-level debug
```

---

## 📊 插件优先级参考 / Plugin Priority Reference

| 优先级 / Priority | 用途 / Usage | 示例 / Example |
|------------------|-------------|---------------|
| 1000 | 最高优先级 / Highest | 认证、鉴权 / Auth |
| 900 | 很高 / Very High | 存储 / Storage |
| 800 | 高 / High | 过滤、审核 / Filter |
| 500 | 中等 / Medium | AI、业务逻辑 / AI, Business |
| 200 | 低 / Low | 通知、日志 / Notification, Logging |
| 100 | 最低 / Lowest | 统计、分析 / Stats, Analytics |

---

## 🔧 调试技巧 / Debugging Tips

### 1. 启用调试模式 / Enable Debug Mode

```bash
cargo run --example plugin_ai_example -- --socket ./plugins/ai.sock --debug
```

### 2. 设置日志级别 / Set Log Level

```bash
cargo run --example plugin_ai_example -- --log-level trace
```

### 3. 查看插件通信 / View Plugin Communication

```bash
# 监听 socket 文件
socat - UNIX-CONNECT:./plugins/ai.sock
```

---

## 📖 更多资源 / More Resources

- **插件开发文档**: `/docs/plugin/`
- **API 参考**: `/docs/api-reference/`
- **完整示例**: `/v-plugins-hub/`

---

## 💡 常见问题 / FAQ

### Q: 如何选择使用 trait 还是手动分发？
**A**: 如果你的插件有标准的事件集（如存储插件），使用 trait 可以减少样板代码。如果是自定义事件，手动 match 分发更灵活。

### Q: 插件如何与主服务通信？
**A**: 插件通过 Unix Socket 与主服务通信，使用 JSON 格式交换数据。

### Q: 如何测试插件？
**A**: 可以编写单元测试 mock `Context`，或者使用集成测试与真实服务交互。

### Q: 插件可以调用其他插件吗？
**A**: 可以，通过主服务的事件系统进行插件间通信。

---

**最后更新 / Last Updated**: 2025-12-06  
**版本 / Version**: 1.0.0  
**维护者 / Maintainer**: VGO Team
