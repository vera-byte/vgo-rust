//! # 简化存储插件示例 / Simple Storage Plugin Example
//!
//! 演示如何使用 StorageEventListener trait 创建存储插件
//! Demonstrates how to create a storage plugin using StorageEventListener trait
//!
//! ## 功能 / Features
//! - ✅ 使用内存存储（HashMap）
//! - ✅ 实现 StorageEventListener trait
//! - ✅ 自动事件分发
//! - ✅ 零样板代码
//!
//! ## 运行方式 / How to Run
//! ```bash
//! cargo run --example plugin_storage_simple_example -- --socket ./plugins/storage-simple.sock
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use v::plugin::pdk::{Context, Plugin, StorageEventListener};
use v::{debug, info};

// ============================================================================
// 插件配置 / Plugin Configuration
// ============================================================================

/// 简单存储配置 / Simple storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SimpleStorageConfig {
    /// 最大存储消息数 / Max stored messages
    #[serde(default = "default_max_messages")]
    max_messages: usize,
}

fn default_max_messages() -> usize {
    1000
}

// ============================================================================
// 存储监听器实现 / Storage Listener Implementation
// ============================================================================

/// 简单内存存储监听器 / Simple memory storage listener
///
/// 使用 HashMap 在内存中存储数据（仅用于演示）
/// Uses HashMap to store data in memory (for demonstration only)
pub struct SimpleStorageListener {
    /// 配置 / Configuration
    pub config: SimpleStorageConfig,

    /// 消息存储 / Message storage
    messages: HashMap<String, serde_json::Value>,

    /// 离线消息存储 / Offline message storage
    offline_messages: HashMap<String, Vec<serde_json::Value>>,

    /// 房间成员存储 / Room members storage
    room_members: HashMap<String, Vec<String>>,
}

impl SimpleStorageListener {
    /// 创建新的存储监听器 / Create new storage listener
    pub fn new(config: SimpleStorageConfig) -> Result<Self> {
        info!("💾 初始化简单存储监听器 / Initializing simple storage listener");

        Ok(Self {
            config,
            messages: HashMap::new(),
            offline_messages: HashMap::new(),
            room_members: HashMap::new(),
        })
    }

    /// 获取配置的可变引用 / Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut SimpleStorageConfig {
        &mut self.config
    }
}

// ============================================================================
// 实现 StorageEventListener Trait / Implement StorageEventListener Trait
// ============================================================================

#[async_trait]
impl StorageEventListener for SimpleStorageListener {
    /// 保存消息 / Save message
    async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");

        debug!("💾 保存消息 / Saving message: {}", message_id);

        // 保存到内存 / Save to memory
        self.messages
            .insert(message_id.to_string(), ctx.payload.clone());

        // 限制存储数量 / Limit storage size
        if self.messages.len() > self.config.max_messages {
            // 移除最旧的消息（简化实现）/ Remove oldest message (simplified)
            if let Some(first_key) = self.messages.keys().next().cloned() {
                self.messages.remove(&first_key);
            }
        }

        ctx.reply(json!({
            "status": "ok",
            "saved": true,
            "message_id": message_id,
            "total_messages": self.messages.len()
        }))?;

        info!("✅ 消息已保存 / Message saved: {}", message_id);
        Ok(())
    }

    /// 保存离线消息 / Save offline message
    async fn storage_offline_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");

        debug!(
            "💾 保存离线消息 / Saving offline message: {} for {}",
            message_id, to_uid
        );

        // 保存到离线消息列表 / Save to offline messages list
        self.offline_messages
            .entry(to_uid.to_string())
            .or_insert_with(Vec::new)
            .push(ctx.payload.clone());

        ctx.reply(json!({
            "status": "ok",
            "saved": true,
            "message_id": message_id
        }))?;

        info!("✅ 离线消息已保存 / Offline message saved");
        Ok(())
    }

    /// 拉取离线消息 / Pull offline messages
    async fn storage_offline_pull(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let limit = ctx
            .payload
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        debug!("📤 拉取离线消息 / Pulling offline messages for {}", to_uid);

        let messages = self
            .offline_messages
            .get(to_uid)
            .map(|msgs| msgs.iter().take(limit).cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        ctx.reply(json!({
            "status": "ok",
            "messages": messages,
            "count": messages.len()
        }))?;

        info!(
            "✅ 拉取了 {} 条离线消息 / Pulled {} offline messages",
            messages.len(),
            messages.len()
        );
        Ok(())
    }

    /// 确认离线消息 / Acknowledge offline messages
    async fn storage_offline_ack(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");

        debug!(
            "✔️  确认离线消息 / Acknowledging offline messages for {}",
            to_uid
        );

        // 清空该用户的离线消息 / Clear offline messages for user
        let removed = self
            .offline_messages
            .remove(to_uid)
            .map(|v| v.len())
            .unwrap_or(0);

        ctx.reply(json!({
            "status": "ok",
            "removed": removed
        }))?;

        info!(
            "✅ 已确认 {} 条离线消息 / Acknowledged {} offline messages",
            removed, removed
        );
        Ok(())
    }

    /// 统计离线消息数量 / Count offline messages
    async fn storage_offline_count(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");

        let count = self
            .offline_messages
            .get(to_uid)
            .map(|v| v.len())
            .unwrap_or(0);

        ctx.reply(json!({
            "status": "ok",
            "count": count
        }))?;

        Ok(())
    }

    /// 添加房间成员 / Add room member
    async fn storage_room_add_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");
        let uid = ctx.get_payload_str("uid").unwrap_or("");

        debug!(
            "👥 添加房间成员 / Adding room member: {} to {}",
            uid, room_id
        );

        self.room_members
            .entry(room_id.to_string())
            .or_insert_with(Vec::new)
            .push(uid.to_string());

        ctx.reply(json!({"status": "ok"}))?;

        info!("✅ 已添加房间成员 / Room member added");
        Ok(())
    }

    /// 移除房间成员 / Remove room member
    async fn storage_room_remove_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");
        let uid = ctx.get_payload_str("uid").unwrap_or("");

        debug!(
            "👥 移除房间成员 / Removing room member: {} from {}",
            uid, room_id
        );

        if let Some(members) = self.room_members.get_mut(room_id) {
            members.retain(|m| m != uid);
        }

        ctx.reply(json!({"status": "ok"}))?;

        info!("✅ 已移除房间成员 / Room member removed");
        Ok(())
    }

    /// 列出房间成员 / List room members
    async fn storage_room_list_members(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");

        let members = self.room_members.get(room_id).cloned().unwrap_or_default();

        ctx.reply(json!({
            "status": "ok",
            "members": members,
            "count": members.len()
        }))?;

        Ok(())
    }

    /// 列出所有房间 / List all rooms
    async fn storage_room_list(&mut self, ctx: &mut Context) -> Result<()> {
        let rooms: Vec<String> = self.room_members.keys().cloned().collect();

        ctx.reply(json!({
            "status": "ok",
            "rooms": rooms,
            "count": rooms.len()
        }))?;

        Ok(())
    }

    /// 记录已读回执 / Record read receipt
    async fn storage_read_record(&mut self, ctx: &mut Context) -> Result<()> {
        // 简化实现，仅返回成功 / Simplified implementation, just return success
        ctx.reply(json!({"status": "ok"}))?;
        Ok(())
    }

    /// 查询历史消息 / Query message history
    async fn storage_message_history(&mut self, ctx: &mut Context) -> Result<()> {
        let limit = ctx
            .payload
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let messages: Vec<serde_json::Value> =
            self.messages.values().take(limit).cloned().collect();

        ctx.reply(json!({
            "status": "ok",
            "messages": messages,
            "count": messages.len()
        }))?;

        info!(
            "✅ 查询到 {} 条历史消息 / Found {} history messages",
            messages.len(),
            messages.len()
        );
        Ok(())
    }

    /// 获取统计信息 / Get statistics
    async fn storage_stats(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.reply(json!({
            "status": "ok",
            "stats": {
                "total_messages": self.messages.len(),
                "total_offline_users": self.offline_messages.len(),
                "total_rooms": self.room_members.len()
            }
        }))?;

        Ok(())
    }
}

// ============================================================================
// 插件主结构 / Plugin Main Structure
// ============================================================================

/// 简单存储插件 / Simple storage plugin
struct SimpleStoragePlugin {
    /// 存储监听器 / Storage listener
    listener: SimpleStorageListener,
}

impl Plugin for SimpleStoragePlugin {
    type Config = SimpleStorageConfig;

    fn new() -> Self {
        info!("🗄️  初始化简单存储插件 / Initializing Simple Storage Plugin");

        let config = SimpleStorageConfig::default();
        let listener = SimpleStorageListener::new(config)
            .expect("无法创建存储监听器 / Failed to create storage listener");

        info!("✅ 简单存储插件初始化完成 / Simple Storage Plugin initialized");

        Self { listener }
    }

    fn config(&self) -> Option<&Self::Config> {
        Some(&self.listener.config)
    }

    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        Some(self.listener.config_mut())
    }

    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        info!("📝 配置已更新 / Config updated: {:?}", config);
        *self.listener.config_mut() = config;
        Ok(())
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["storage".into()]
    }

    /// 接收并处理存储事件 / Receive and handle storage events
    ///
    /// 使用 trait 的自动分发方法，零样板代码！
    /// Use trait's auto dispatch method, zero boilerplate!
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.listener.dispatch(ctx))
            // 自动分发！/ Auto dispatch!
        })
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 简单存储插件程序入口点 / Simple storage plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 插件元信息 / Plugin metadata
    const PLUGIN_NO: &str = "v.plugin.storage-simple";
    const VERSION: &str = "0.1.0";
    const PRIORITY: i32 = 900;

    info!("🚀 启动简单存储插件示例 / Starting Simple Storage Plugin Example");

    // 启动插件服务器 / Start plugin server
    v::plugin::pdk::run_server::<SimpleStoragePlugin>(PLUGIN_NO, VERSION, PRIORITY).await
}
