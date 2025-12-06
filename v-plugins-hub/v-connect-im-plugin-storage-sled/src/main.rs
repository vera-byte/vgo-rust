//! # v-connect-im 存储插件 / v-connect-im Storage Plugin
//!
//! 基于 Sled 实现的高性能存储插件
//! High-performance storage plugin based on Sled
//!
//! ## 功能特性 / Features
//!
//! - ✅ 消息持久化 / Message persistence
//! - ✅ 离线消息管理 / Offline message management
//! - ✅ 房间成员管理 / Room member management
//! - ✅ 已读回执存储 / Read receipt storage
//! - ✅ 高性能嵌入式数据库 / High-performance embedded database
//!
//! ## 设计模式 / Design Pattern
//!
//! 本插件采用新的事件处理器模式：
//! This plugin uses the new event handler pattern:
//!
//! - 使用 `on_*` 方法命名规范 / Use `on_*` method naming convention
//! - 通过 `dispatch_event` 方法统一分发 / Unified dispatch via `dispatch_event`
//! - 清晰的事件处理流程 / Clear event handling flow
//! - 易于维护和扩展 / Easy to maintain and extend

// ============================================================================
// 依赖导入 / Dependencies
// ============================================================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use v::plugin::pdk::{Context, Plugin};
use v::{debug, info, warn};

// ============================================================================
// 插件元信息 / Plugin Metadata
// ============================================================================

/// 插件唯一标识符 / Plugin unique identifier
const PLUGIN_NO: &str = "v.plugin.storage-sled";

/// 插件版本号 / Plugin version
const VERSION: &str = "0.1.0";

/// 插件优先级 / Plugin priority
/// 存储插件应该有较高优先级以确保数据及时保存
/// Storage plugin should have high priority to ensure data is saved promptly
const PRIORITY: i32 = 900;

// ============================================================================
// 常量定义 / Constants
// ============================================================================

/// 成功响应状态 / Success response status
const STATUS_OK: &str = "ok";

/// 错误响应状态 / Error response status
const STATUS_ERROR: &str = "error";

// ============================================================================
// 插件配置结构 / Plugin Configuration Structure
// ============================================================================

/// 存储插件配置 / Storage plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StorageConfig {
    /// 数据库路径 / Database path
    #[serde(default = "default_db_path")]
    db_path: String,

    /// 离线消息最大数量 / Max offline messages
    #[serde(default = "default_max_offline")]
    max_offline_messages: usize,

    /// 是否启用压缩 / Enable compression
    #[serde(default)]
    enable_compression: bool,
}

fn default_db_path() -> String {
    "./data/plugin-storage".to_string()
}

fn default_max_offline() -> usize {
    10000
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            max_offline_messages: default_max_offline(),
            enable_compression: false,
        }
    }
}

// ============================================================================
// 插件主结构 / Plugin Main Structure
// ============================================================================

/// 存储插件主结构 / Storage plugin main structure
struct StoragePlugin {
    /// 插件配置 / Plugin configuration
    config: StorageConfig,

    /// Sled 数据库实例 / Sled database instance
    db: sled::Db,

    /// WAL (Write-Ahead Log) 树 / WAL tree
    wal: sled::Tree,

    /// 离线消息树 / Offline messages tree
    offline: sled::Tree,

    /// 房间成员树 / Room members tree
    room_members: sled::Tree,

    /// 已读回执树 / Read receipts tree
    reads: sled::Tree,

    /// 统计信息 / Statistics
    stats: PluginStats,
}

/// 插件统计信息 / Plugin statistics
#[derive(Debug, Default)]
struct PluginStats {
    /// 保存的消息总数 / Total messages saved
    messages_saved: u64,

    /// 保存的离线消息总数 / Total offline messages saved
    offline_saved: u64,

    /// 拉取的离线消息总数 / Total offline messages pulled
    offline_pulled: u64,

    /// 确认的离线消息总数 / Total offline messages acknowledged
    offline_acked: u64,
}

impl Plugin for StoragePlugin {
    type Config = StorageConfig;

    fn new() -> Self {
        info!("🗄️  初始化存储插件 / Initializing Storage Plugin");

        let config = StorageConfig::default();
        let db = sled::open(&config.db_path).expect("无法打开数据库 / Failed to open database");

        let wal = db
            .open_tree("wal")
            .expect("无法打开 WAL 树 / Failed to open WAL tree");
        let offline = db
            .open_tree("offline")
            .expect("无法打开离线消息树 / Failed to open offline tree");
        let room_members = db
            .open_tree("room_members")
            .expect("无法打开房间成员树 / Failed to open room_members tree");
        let reads = db
            .open_tree("reads")
            .expect("无法打开已读回执树 / Failed to open reads tree");

        info!("✅ 存储插件初始化完成 / Storage Plugin initialized");
        info!("📁 数据库路径 / Database path: {}", config.db_path);

        Self {
            config,
            db,
            wal,
            offline,
            room_members,
            reads,
            stats: PluginStats::default(),
        }
    }

    fn config(&self) -> Option<&Self::Config> {
        Some(&self.config)
    }

    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        Some(&mut self.config)
    }

    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        info!("📝 配置已更新 / Config updated: {:?}", config);

        // 如果数据库路径改变，需要重新打开数据库
        // If database path changed, need to reopen database
        if config.db_path != self.config.db_path {
            warn!("⚠️  数据库路径已改变，需要重启插件 / Database path changed, plugin restart required");
        }

        self.config = config;
        Ok(())
    }

    /// 声明插件能力 / Declare plugin capabilities
    ///
    /// 存储插件声明 "storage" 能力，服务器会将 storage.* 事件路由到此插件
    /// Storage plugin declares "storage" capability, server routes storage.* events to this plugin
    fn capabilities(&self) -> Vec<String> {
        vec!["storage".into()]
    }

    /// 接收并处理存储事件 / Receive and handle storage events
    ///
    /// 使用新的事件处理器模式进行分发
    /// Use new event handler pattern for dispatch
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        self.dispatch_event(ctx)
    }
}

// ============================================================================
// 事件分发宏 / Event Dispatch Macro
// ============================================================================

/// 事件分发宏 / Event dispatch macro
///
/// 自动生成事件路由逻辑，避免重复的 match 分支
/// Automatically generates event routing logic, avoiding repetitive match branches
macro_rules! dispatch_events {
    ($self:ident, $ctx:ident, {
        $($event_name:literal => $handler:ident),* $(,)?
    }) => {{
        let event_type = $ctx.event_type();
        debug!("📨 收到存储事件 / Received storage event: {}", event_type);

        match event_type {
            $($event_name => $self.$handler($ctx),)*
            _ => {
                warn!("⚠️  未知的存储事件类型 / Unknown storage event type: {}", event_type);
                $ctx.reply(json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
                Ok(())
            }
        }
    }};
}

// ============================================================================
// 事件分发器 / Event Dispatcher
// ============================================================================

impl StoragePlugin {
    /// 事件分发方法 / Event dispatch method
    ///
    /// 使用宏自动生成分发逻辑，避免重复代码
    /// Use macro to auto-generate dispatch logic, avoiding code duplication
    fn dispatch_event(&mut self, ctx: &mut Context) -> Result<()> {
        dispatch_events!(self, ctx, {
            "storage.message.save" => on_message_save,
            "storage.offline.save" => on_offline_save,
            "storage.offline.pull" => on_offline_pull,
            "storage.offline.ack" => on_offline_ack,
            "storage.offline.count" => on_offline_count,
            "storage.room.add_member" => on_room_add_member,
            "storage.room.remove_member" => on_room_remove_member,
            "storage.room.list_members" => on_room_list_members,
            "storage.room.list" => on_room_list,
            "storage.read.record" => on_read_record,
            "storage.message.history" => on_message_history,
            "storage.stats" => on_stats,
        })
    }
}

// ============================================================================
// 事件处理方法 / Event Handler Methods
// ============================================================================

impl StoragePlugin {
    /// 处理消息保存事件 / Handle message save event
    ///
    /// 将消息保存到 WAL (Write-Ahead Log)
    /// Save message to WAL (Write-Ahead Log)
    fn on_message_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");
        let timestamp = ctx.payload.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);

        debug!("💾 保存消息 / Saving message: {} at {}", message_id, timestamp);

        // 构建键：timestamp:message_id / Build key: timestamp:message_id
        let key = format!("{}:{}", timestamp, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        // 保存到 WAL / Save to WAL
        self.wal.insert(key.as_bytes(), val)?;
        self.wal.flush()?;

        self.stats.messages_saved += 1;

        ctx.reply(Self::ok_response_with(json!({
            "saved": true,
            "message_id": message_id
        })))?;

        info!("✅ 消息已保存 / Message saved: {}", message_id);
        Ok(())
    }

    /// 处理离线消息保存事件 / Handle offline message save event
    fn on_offline_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let timestamp = ctx.payload.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);

        debug!("💾 保存离线消息 / Saving offline message: {} for {}", message_id, to_uid);

        // 检查离线消息数量限制 / Check offline message limit
        let count = self.count_offline_messages(to_uid)?;
        if count >= self.config.max_offline_messages {
            warn!("⚠️  用户 {} 的离线消息已达上限 / User {} reached offline message limit", to_uid);
            self.remove_oldest_offline(to_uid, 1)?;
        }

        // 构建键：to_uid:timestamp:message_id / Build key
        let key = format!("{}:{}:{}", to_uid, timestamp, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        // 保存到离线消息树 / Save to offline tree
        self.offline.insert(key.as_bytes(), val)?;
        self.offline.flush()?;

        self.stats.offline_saved += 1;

        ctx.reply(Self::ok_response_with(json!({
            "saved": true,
            "message_id": message_id
        })))?;

        info!("✅ 离线消息已保存 / Offline message saved: {} for {}", message_id, to_uid);
        Ok(())
    }

    /// 处理离线消息拉取事件 / Handle offline message pull event
    fn on_offline_pull(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let limit = ctx.payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        debug!("📤 拉取离线消息 / Pulling offline messages for {}, limit: {}", to_uid, limit);

        let messages: Vec<serde_json::Value> = self.offline
            .scan_prefix(Self::user_prefix(to_uid).as_bytes())
            .take(limit)
            .filter_map(|item| item.ok())
            .filter_map(|(_, v)| serde_json::from_slice(&v).ok())
            .collect();

        self.stats.offline_pulled += messages.len() as u64;

        ctx.reply(json!({
            "status": "ok",
            "messages": messages,
            "count": messages.len()
        }))?;

        info!("✅ 拉取了 {} 条离线消息 / Pulled {} offline messages for {}", messages.len(), to_uid);
        Ok(())
    }

    /// 处理离线消息确认事件 / Handle offline message acknowledgment event
    fn on_offline_ack(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let message_ids: Vec<String> = ctx
            .payload
            .get("message_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        debug!("✔️  确认离线消息 / Acknowledging offline messages for {}: {:?}", to_uid, message_ids);

        let mut removed = 0;
        for item in self.offline.iter() {
            let (k, v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if ks.starts_with(&format!("{}:", to_uid)) {
                let msg: serde_json::Value = serde_json::from_slice(&v)?;
                if let Some(msg_id) = msg.get("message_id").and_then(|v| v.as_str()) {
                    if message_ids.iter().any(|id| id == msg_id) {
                        self.offline.remove(k)?;
                        removed += 1;
                    }
                }
            }
        }

        if removed > 0 {
            self.offline.flush()?;
        }

        self.stats.offline_acked += removed;

        ctx.reply(Self::ok_response_with(json!({"removed": removed})))?;

        info!("✅ 已确认 {} 条离线消息 / Acknowledged {} offline messages for {}", removed, to_uid);
        Ok(())
    }

    /// 处理离线消息计数事件 / Handle offline message count event
    fn on_offline_count(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let count = self.count_offline_messages(to_uid)?;

        ctx.reply(Self::ok_response_with(json!({"count": count})))?;

        Ok(())
    }

    /// 处理添加房间成员事件 / Handle add room member event
    fn on_room_add_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");
        let uid = ctx.get_payload_str("uid").unwrap_or("");

        debug!("👥 添加房间成员 / Adding room member: {} to {}", uid, room_id);

        let key = Self::room_member_key(room_id, uid);
        self.room_members.insert(key.as_bytes(), b"1")?;
        self.room_members.flush()?;

        ctx.reply(Self::ok_response())?;

        info!("✅ 已添加房间成员 / Room member added: {} to {}", uid, room_id);
        Ok(())
    }

    /// 处理移除房间成员事件 / Handle remove room member event
    fn on_room_remove_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");
        let uid = ctx.get_payload_str("uid").unwrap_or("");

        debug!("👥 移除房间成员 / Removing room member: {} from {}", uid, room_id);

        let key = Self::room_member_key(room_id, uid);
        self.room_members.remove(key.as_bytes())?;
        self.room_members.flush()?;

        ctx.reply(Self::ok_response())?;

        info!("✅ 已移除房间成员 / Room member removed: {} from {}", uid, room_id);
        Ok(())
    }

    /// 处理列出房间成员事件 / Handle list room members event
    fn on_room_list_members(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");

        debug!("📋 列出房间成员 / Listing room members for {}", room_id);

        let members: Vec<String> = self.room_members
            .scan_prefix(Self::user_prefix(room_id).as_bytes())
            .filter_map(|item| item.ok())
            .filter_map(|(k, _)| String::from_utf8(k.to_vec()).ok())
            .filter_map(|ks| ks.split_once(':').map(|(_, uid)| uid.to_string()))
            .collect();

        ctx.reply(Self::ok_response_with(json!({
            "members": members,
            "count": members.len()
        })))?;

        Ok(())
    }

    /// 处理列出所有房间事件 / Handle list all rooms event
    fn on_room_list(&mut self, ctx: &mut Context) -> Result<()> {
        debug!("📋 列出所有房间 / Listing all rooms");

        use std::collections::HashSet;
        let rooms: HashSet<String> = self.room_members.iter()
            .filter_map(|item| item.ok())
            .filter_map(|(k, _)| String::from_utf8(k.to_vec()).ok())
            .filter_map(|ks| ks.split_once(':').map(|(rid, _)| rid.to_string()))
            .collect();

        let room_list: Vec<String> = rooms.into_iter().collect();

        ctx.reply(Self::ok_response_with(json!({
            "rooms": room_list,
            "count": room_list.len()
        })))?;

        Ok(())
    }

    /// 处理记录已读回执事件 / Handle record read receipt event
    fn on_read_record(&mut self, ctx: &mut Context) -> Result<()> {
        let uid = ctx.get_payload_str("uid").unwrap_or("");
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");

        debug!("✔️  记录已读回执 / Recording read receipt: {} by {}", message_id, uid);

        let key = format!("{}:{}", uid, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        self.reads.insert(key.as_bytes(), val)?;
        self.reads.flush()?;

        ctx.reply(Self::ok_response())?;

        Ok(())
    }

    /// 处理历史消息查询事件 / Handle message history query event
    fn on_message_history(&mut self, ctx: &mut Context) -> Result<()> {
        let limit = ctx.payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
        let since_ts = ctx.payload.get("since_ts").and_then(|v| v.as_i64());
        let until_ts = ctx.payload.get("until_ts").and_then(|v| v.as_i64());

        debug!("📜 查询历史消息 / Querying message history, limit: {}", limit);

        let mut messages = Vec::new();

        // 遍历 WAL 树获取消息 / Iterate WAL tree to get messages
        for item in self.wal.iter() {
            if messages.len() >= limit {
                break;
            }

            let (k, v) = item?;
            let key_str = String::from_utf8(k.to_vec()).unwrap_or_default();

            // 键格式: timestamp:message_id / Key format: timestamp:message_id
            if let Some((ts_str, _)) = key_str.split_once(':') {
                if let Ok(ts) = ts_str.parse::<i64>() {
                    // 时间范围过滤 / Time range filter
                    if let Some(since) = since_ts {
                        if ts < since {
                            continue;
                        }
                    }
                    if let Some(until) = until_ts {
                        if ts > until {
                            continue;
                        }
                    }

                    // 解析消息 / Parse message
                    if let Ok(msg) = serde_json::from_slice::<serde_json::Value>(&v) {
                        messages.push(msg);
                    }
                }
            }
        }

        ctx.reply(json!({
            "status": "ok",
            "messages": messages,
            "count": messages.len()
        }))?;

        info!("✅ 查询到 {} 条历史消息 / Found {} history messages", messages.len());
        Ok(())
    }

    /// 处理统计信息查询事件 / Handle stats query event
    fn on_stats(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.reply(Self::ok_response_with(json!({
            "stats": {
                "messages_saved": self.stats.messages_saved,
                "offline_saved": self.stats.offline_saved,
                "offline_pulled": self.stats.offline_pulled,
                "offline_acked": self.stats.offline_acked,
                "db_size": self.db.size_on_disk().unwrap_or(0)
            }
        })))?;

        Ok(())
    }
}

// ============================================================================
// 辅助方法 / Helper Methods
// ============================================================================

impl StoragePlugin {
    /// 构建用户前缀 / Build user prefix
    #[inline]
    fn user_prefix(uid: &str) -> String {
        format!("{}:", uid)
    }

    /// 构建房间成员键 / Build room member key
    #[inline]
    fn room_member_key(room_id: &str, uid: &str) -> String {
        format!("{}:{}", room_id, uid)
    }

    /// 构建成功响应 / Build success response
    #[inline]
    fn ok_response() -> serde_json::Value {
        json!({"status": STATUS_OK})
    }

    /// 构建带数据的成功响应 / Build success response with data
    #[inline]
    fn ok_response_with(data: serde_json::Value) -> serde_json::Value {
        let mut resp = json!({"status": STATUS_OK});
        if let Some(obj) = resp.as_object_mut() {
            if let Some(data_obj) = data.as_object() {
                obj.extend(data_obj.clone());
            }
        }
        resp
    }

    /// 统计用户的离线消息数量 / Count offline messages for user
    fn count_offline_messages(&self, to_uid: &str) -> Result<usize> {
        Ok(self.offline.scan_prefix(Self::user_prefix(to_uid).as_bytes()).count())
    }

    /// 删除最旧的离线消息 / Remove oldest offline messages
    fn remove_oldest_offline(&self, to_uid: &str, count: usize) -> Result<usize> {
        let prefix = Self::user_prefix(to_uid);
        let keys_to_remove: Vec<_> = self.offline
            .scan_prefix(prefix.as_bytes())
            .take(count)
            .filter_map(|item| item.ok().map(|(k, _)| k))
            .collect();

        let removed = keys_to_remove.len();
        for key in keys_to_remove {
            self.offline.remove(key)?;
        }

        if removed > 0 {
            self.offline.flush()?;
        }

        Ok(removed)
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 存储插件程序入口点 / Storage plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 启动存储插件服务器 / Start storage plugin server
    v::plugin::pdk::run_server::<StoragePlugin>(PLUGIN_NO, VERSION, PRIORITY).await
}
