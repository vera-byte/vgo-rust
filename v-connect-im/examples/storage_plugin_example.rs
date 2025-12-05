//! 存储插件示例 - 使用事件处理器 trait / Storage plugin example using event handler trait
//!
//! 演示如何使用 StorageEventHandler trait 实现存储插件
//! Demonstrates how to implement a storage plugin using StorageEventHandler trait
//!
//! 运行方式 / Run with:
//! ```bash
//! cargo run --example storage_plugin_example
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// 模拟导入 / Mock imports
// 在实际项目中，这些应该从 v-connect-im 导入
// In actual project, these should be imported from v-connect-im

/// 插件事件上下文 / Plugin event context
pub struct Context {
    event_type: String,
    payload: serde_json::Value,
    response: Option<serde_json::Value>,
}

impl Context {
    pub fn new(event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
            response: None,
        }
    }

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T> {
        self.payload
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Missing field: {}", key))
            .and_then(|v| serde_json::from_value(v.clone()).map_err(Into::into))
    }

    pub fn reply(&mut self, response: serde_json::Value) -> Result<()> {
        self.response = Some(response);
        Ok(())
    }

    pub fn response(&self) -> Option<&serde_json::Value> {
        self.response.as_ref()
    }
}

/// 存储事件处理器 trait / Storage event handler trait
#[async_trait]
pub trait StorageEventHandler: Send + Sync {
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()>;
    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()>;
    async fn on_offline_pull(&self, ctx: &mut Context) -> Result<()>;
    async fn on_offline_ack(&self, ctx: &mut Context) -> Result<()>;
    async fn on_stats(&self, ctx: &mut Context) -> Result<()>;

    async fn dispatch(&self, ctx: &mut Context) -> Result<()> {
        match ctx.event_type() {
            "storage.message.save" => self.on_message_save(ctx).await?,
            "storage.offline.save" => self.on_offline_save(ctx).await?,
            "storage.offline.pull" => self.on_offline_pull(ctx).await?,
            "storage.offline.ack" => self.on_offline_ack(ctx).await?,
            "storage.stats" => self.on_stats(ctx).await?,
            _ => {
                ctx.reply(json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", ctx.event_type())
                }))?;
            }
        }
        Ok(())
    }
}

// ============================================================================
// 存储插件实现示例 / Storage plugin implementation example
// ============================================================================

/// 内存存储插件 / In-memory storage plugin
///
/// 使用 HashMap 实现简单的内存存储
/// Uses HashMap for simple in-memory storage
pub struct MemoryStoragePlugin {
    /// 消息存储 / Message storage
    messages: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 离线消息存储 / Offline message storage
    offline_messages: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    /// 统计信息 / Statistics
    stats: Arc<RwLock<StorageStats>>,
}

#[derive(Default)]
struct StorageStats {
    total_messages: usize,
    total_offline: usize,
}

impl MemoryStoragePlugin {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
            offline_messages: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(StorageStats::default())),
        }
    }
}

#[async_trait]
impl StorageEventHandler for MemoryStoragePlugin {
    /// 处理消息保存 / Handle message save
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        println!("💾 处理消息保存事件 / Handling message save event");

        // 从上下文中提取数据 / Extract data from context
        let message_id: String = ctx.get("message_id")?;
        let from_uid: String = ctx.get("from_uid")?;
        let to_uid: String = ctx.get("to_uid")?;
        let _content = ctx.payload().get("content").cloned().unwrap_or(json!({}));

        println!(
            "  消息ID / Message ID: {}\n  发送者 / From: {}\n  接收者 / To: {}",
            message_id, from_uid, to_uid
        );

        // 保存消息 / Save message
        let mut messages = self.messages.write().await;
        messages.insert(message_id.clone(), ctx.payload().clone());

        // 更新统计 / Update stats
        let mut stats = self.stats.write().await;
        stats.total_messages += 1;

        // 返回成功响应 / Return success response
        ctx.reply(json!({
            "status": "ok",
            "message_id": message_id,
            "saved_at": chrono::Utc::now().timestamp()
        }))?;

        println!("✅ 消息保存成功 / Message saved successfully");
        Ok(())
    }

    /// 处理离线消息保存 / Handle offline message save
    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()> {
        println!("📥 处理离线消息保存事件 / Handling offline message save event");

        let to_uid: String = ctx.get("to_uid")?;
        let message_id: String = ctx.get("message_id")?;

        println!(
            "  接收者 / Recipient: {}\n  消息ID / Message ID: {}",
            to_uid, message_id
        );

        // 保存离线消息 / Save offline message
        let mut offline = self.offline_messages.write().await;
        offline
            .entry(to_uid.clone())
            .or_insert_with(Vec::new)
            .push(ctx.payload().clone());

        // 更新统计 / Update stats
        let mut stats = self.stats.write().await;
        stats.total_offline += 1;

        ctx.reply(json!({
            "status": "ok",
            "to_uid": to_uid,
            "message_id": message_id
        }))?;

        println!("✅ 离线消息保存成功 / Offline message saved successfully");
        Ok(())
    }

    /// 处理离线消息拉取 / Handle offline message pull
    async fn on_offline_pull(&self, ctx: &mut Context) -> Result<()> {
        println!("📤 处理离线消息拉取事件 / Handling offline message pull event");

        let to_uid: String = ctx.get("to_uid")?;
        let limit: usize = ctx.get("limit").unwrap_or(100);

        println!("  用户 / User: {}\n  限制 / Limit: {}", to_uid, limit);

        // 获取离线消息 / Get offline messages
        let offline = self.offline_messages.read().await;
        let messages = offline
            .get(&to_uid)
            .map(|msgs| msgs.iter().take(limit).cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        let count = messages.len();

        ctx.reply(json!({
            "status": "ok",
            "messages": messages,
            "count": count
        }))?;

        println!(
            "✅ 返回 {} 条离线消息 / Returned {} offline messages",
            count, count
        );
        Ok(())
    }

    /// 处理离线消息确认 / Handle offline message ack
    async fn on_offline_ack(&self, ctx: &mut Context) -> Result<()> {
        println!("✔️  处理离线消息确认事件 / Handling offline message ack event");

        let to_uid: String = ctx.get("to_uid")?;
        let message_ids: Vec<String> = ctx.get("message_ids")?;

        println!(
            "  用户 / User: {}\n  确认消息数 / Ack count: {}",
            to_uid,
            message_ids.len()
        );

        // 删除已确认的离线消息 / Remove acknowledged offline messages
        let mut offline = self.offline_messages.write().await;
        if let Some(messages) = offline.get_mut(&to_uid) {
            messages.retain(|msg| {
                let msg_id = msg.get("message_id").and_then(|v| v.as_str()).unwrap_or("");
                !message_ids.contains(&msg_id.to_string())
            });
        }

        ctx.reply(json!({
            "status": "ok",
            "acked_count": message_ids.len()
        }))?;

        println!(
            "✅ 已确认 {} 条消息 / Acknowledged {} messages",
            message_ids.len(),
            message_ids.len()
        );
        Ok(())
    }

    /// 处理统计查询 / Handle stats query
    async fn on_stats(&self, ctx: &mut Context) -> Result<()> {
        println!("📊 处理统计查询事件 / Handling stats query event");

        let stats = self.stats.read().await;
        let messages_count = self.messages.read().await.len();
        let offline_count = self
            .offline_messages
            .read()
            .await
            .values()
            .map(|v| v.len())
            .sum::<usize>();

        ctx.reply(json!({
            "status": "ok",
            "stats": {
                "total_messages": stats.total_messages,
                "total_offline": stats.total_offline,
                "current_messages": messages_count,
                "current_offline": offline_count
            }
        }))?;

        println!("✅ 统计信息已返回 / Stats returned");
        Ok(())
    }
}

// ============================================================================
// 演示代码 / Demo code
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 存储插件事件处理器演示 / Storage Plugin Event Handler Demo");
    println!("{}", "=".repeat(70));

    // 创建存储插件实例 / Create storage plugin instance
    let plugin = MemoryStoragePlugin::new();

    // 演示 1: 保存消息 / Demo 1: Save message
    println!("\n📝 演示 1: 保存消息 / Demo 1: Save Message");
    println!("{}", "-".repeat(70));
    {
        let mut ctx = Context::new(
            "storage.message.save",
            json!({
                "message_id": "msg_001",
                "from_uid": "user_alice",
                "to_uid": "user_bob",
                "content": {
                    "text": "Hello, Bob!"
                },
                "timestamp": chrono::Utc::now().timestamp()
            }),
        );

        plugin.dispatch(&mut ctx).await?;
        println!(
            "响应 / Response: {}",
            serde_json::to_string_pretty(ctx.response().unwrap())?
        );
    }

    // 演示 2: 保存离线消息 / Demo 2: Save offline message
    println!("\n📝 演示 2: 保存离线消息 / Demo 2: Save Offline Message");
    println!("{}", "-".repeat(70));
    {
        let mut ctx = Context::new(
            "storage.offline.save",
            json!({
                "message_id": "msg_002",
                "from_uid": "user_alice",
                "to_uid": "user_charlie",
                "content": {
                    "text": "Hi Charlie, are you there?"
                },
                "timestamp": chrono::Utc::now().timestamp()
            }),
        );

        plugin.dispatch(&mut ctx).await?;
        println!(
            "响应 / Response: {}",
            serde_json::to_string_pretty(ctx.response().unwrap())?
        );
    }

    // 演示 3: 拉取离线消息 / Demo 3: Pull offline messages
    println!("\n📝 演示 3: 拉取离线消息 / Demo 3: Pull Offline Messages");
    println!("{}", "-".repeat(70));
    {
        let mut ctx = Context::new(
            "storage.offline.pull",
            json!({
                "to_uid": "user_charlie",
                "limit": 10
            }),
        );

        plugin.dispatch(&mut ctx).await?;
        println!(
            "响应 / Response: {}",
            serde_json::to_string_pretty(ctx.response().unwrap())?
        );
    }

    // 演示 4: 确认离线消息 / Demo 4: Acknowledge offline messages
    println!("\n📝 演示 4: 确认离线消息 / Demo 4: Acknowledge Offline Messages");
    println!("{}", "-".repeat(70));
    {
        let mut ctx = Context::new(
            "storage.offline.ack",
            json!({
                "to_uid": "user_charlie",
                "message_ids": ["msg_002"]
            }),
        );

        plugin.dispatch(&mut ctx).await?;
        println!(
            "响应 / Response: {}",
            serde_json::to_string_pretty(ctx.response().unwrap())?
        );
    }

    // 演示 5: 查询统计信息 / Demo 5: Query statistics
    println!("\n📝 演示 5: 查询统计信息 / Demo 5: Query Statistics");
    println!("{}", "-".repeat(70));
    {
        let mut ctx = Context::new("storage.stats", json!({}));

        plugin.dispatch(&mut ctx).await?;
        println!(
            "响应 / Response: {}",
            serde_json::to_string_pretty(ctx.response().unwrap())?
        );
    }

    println!("\n✅ 所有演示完成 / All demos completed");
    println!("{}", "=".repeat(70));

    Ok(())
}
