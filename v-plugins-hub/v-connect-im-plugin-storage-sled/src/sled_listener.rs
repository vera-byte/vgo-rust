//! # Sled 存储事件监听器实现 / Sled Storage Event Listener Implementation
//!
//! 基于 Sled 嵌入式数据库的存储事件监听器实现
//! Storage event listener implementation based on Sled embedded database

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use v::plugin::pdk::{Context, StorageEventListener};
use v::{debug, info, warn};

// ============================================================================
// 常量定义 / Constants
// ============================================================================

/// 成功响应状态 / Success response status
const STATUS_OK: &str = "ok";

// ============================================================================
// 配置结构 / Configuration Structure
// ============================================================================

/// Sled 存储配置 / Sled storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SledStorageConfig {
    /// 数据库路径 / Database path
    #[serde(default = "default_db_path")]
    pub db_path: String,

    /// 离线消息最大数量 / Max offline messages
    #[serde(default = "default_max_offline")]
    pub max_offline_messages: usize,

    /// 是否启用压缩 / Enable compression
    #[serde(default)]
    pub enable_compression: bool,
}

fn default_db_path() -> String {
    "./data/plugin-storage".to_string()
}

fn default_max_offline() -> usize {
    10000
}

impl Default for SledStorageConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            max_offline_messages: default_max_offline(),
            enable_compression: false,
        }
    }
}

// ============================================================================
// 统计信息 / Statistics
// ============================================================================

/// 存储统计信息 / Storage statistics
#[derive(Debug, Default)]
pub struct StorageStats {
    /// 保存的消息总数 / Total messages saved
    pub messages_saved: u64,

    /// 保存的离线消息总数 / Total offline messages saved
    pub offline_saved: u64,

    /// 拉取的离线消息总数 / Total offline messages pulled
    pub offline_pulled: u64,

    /// 确认的离线消息总数 / Total offline messages acknowledged
    pub offline_acked: u64,
}

// ============================================================================
// Sled 存储事件监听器 / Sled Storage Event Listener
// ============================================================================

/// Sled 存储事件监听器 / Sled storage event listener
///
/// 使用 Sled 嵌入式数据库实现存储功能
/// Implements storage functionality using Sled embedded database
pub struct SledStorageEventListener {
    /// 配置 / Configuration
    pub config: SledStorageConfig,

    /// Sled 数据库实例 / Sled database instance
    pub db: sled::Db,

    /// WAL (Write-Ahead Log) 树 / WAL tree
    pub wal: sled::Tree,

    /// 离线消息树 / Offline messages tree
    pub offline: sled::Tree,

    /// 房间成员树 / Room members tree
    pub room_members: sled::Tree,

    /// 已读回执树 / Read receipts tree
    pub reads: sled::Tree,

    /// 统计信息 / Statistics
    pub stats: StorageStats,
}

impl SledStorageEventListener {
    /// 创建新的 Sled 存储监听器 / Create new Sled storage listener
    ///
    /// # 参数 / Parameters
    /// - `config`: 存储配置 / Storage configuration
    ///
    /// # 返回 / Returns
    /// - `Result<Self>`: 监听器实例或错误 / Listener instance or error
    pub fn new(config: SledStorageConfig) -> Result<Self> {
        info!("🗄️  初始化 Sled 存储监听器 / Initializing Sled storage listener");
        info!("📁 数据库路径 / Database path: {}", config.db_path);

        // 打开数据库 / Open database
        let db = sled::open(&config.db_path)
            .map_err(|e| anyhow::anyhow!("无法打开数据库 / Failed to open database: {}", e))?;

        // 打开各个树 / Open trees
        let wal = db
            .open_tree("wal")
            .map_err(|e| anyhow::anyhow!("无法打开 WAL 树 / Failed to open WAL tree: {}", e))?;
        let offline = db.open_tree("offline").map_err(|e| {
            anyhow::anyhow!("无法打开离线消息树 / Failed to open offline tree: {}", e)
        })?;
        let room_members = db.open_tree("room_members").map_err(|e| {
            anyhow::anyhow!(
                "无法打开房间成员树 / Failed to open room_members tree: {}",
                e
            )
        })?;
        let reads = db.open_tree("reads").map_err(|e| {
            anyhow::anyhow!("无法打开已读回执树 / Failed to open reads tree: {}", e)
        })?;

        info!("✅ Sled 存储监听器初始化完成 / Sled storage listener initialized");

        Ok(Self {
            config,
            db,
            wal,
            offline,
            room_members,
            reads,
            stats: StorageStats::default(),
        })
    }

    /// 获取配置的可变引用 / Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut SledStorageConfig {
        &mut self.config
    }

    /// 获取统计信息 / Get statistics
    pub fn stats(&self) -> &StorageStats {
        &self.stats
    }
}

// ============================================================================
// 实现 StorageEventListener Trait / Implement StorageEventListener Trait
// ============================================================================

#[async_trait]
impl StorageEventListener for SledStorageEventListener {
    /// 保存消息到 WAL / Save message to WAL
    async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("").to_string();
        let timestamp = ctx
            .payload
            .get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        debug!(
            "💾 保存消息 / Saving message: {} at {}",
            message_id, timestamp
        );

        // 构建键：timestamp:message_id / Build key: timestamp:message_id
        let key = format!("{}:{}", timestamp, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        // 保存到 WAL / Save to WAL
        self.wal.insert(key.as_bytes(), val)?;
        self.wal.flush()?;

        self.stats.messages_saved += 1;

        ctx.reply(json!({
            "status": STATUS_OK,
            "saved": true,
            "message_id": message_id
        }))?;

        info!("✅ 消息已保存 / Message saved: {}", message_id);
        Ok(())
    }

    /// 保存离线消息 / Save offline message
    async fn storage_offline_save(&mut self, ctx: &mut Context) -> Result<()> {
        let message_id = ctx.get_payload_str("message_id").unwrap_or("").to_string();
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("").to_string();
        let timestamp = ctx
            .payload
            .get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        debug!(
            "💾 保存离线消息 / Saving offline message: {} for {}",
            message_id, to_uid
        );

        // 检查离线消息数量限制 / Check offline message limit
        let count = self.count_offline_messages(&to_uid)?;
        if count >= self.config.max_offline_messages {
            warn!(
                "⚠️  用户 {} 的离线消息已达上限 / User {} reached offline message limit",
                to_uid, to_uid
            );
            self.remove_oldest_offline(&to_uid, 1)?;
        }

        // 构建键：to_uid:timestamp:message_id / Build key
        let key = format!("{}:{}:{}", to_uid, timestamp, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        // 保存到离线消息树 / Save to offline tree
        self.offline.insert(key.as_bytes(), val)?;
        self.offline.flush()?;

        self.stats.offline_saved += 1;

        ctx.reply(json!({
            "status": STATUS_OK,
            "saved": true,
            "message_id": message_id
        }))?;

        info!(
            "✅ 离线消息已保存 / Offline message saved: {} for {}",
            message_id, to_uid
        );
        Ok(())
    }

    /// 拉取离线消息 / Pull offline messages
    async fn storage_offline_pull(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("").to_string();
        let limit = ctx
            .payload
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        debug!(
            "📤 拉取离线消息 / Pulling offline messages for {}, limit: {}",
            to_uid, limit
        );

        let messages: Vec<serde_json::Value> = self
            .offline
            .scan_prefix(Self::user_prefix(&to_uid).as_bytes())
            .take(limit)
            .filter_map(|item| item.ok())
            .filter_map(|(_, v)| serde_json::from_slice(&v).ok())
            .collect();

        self.stats.offline_pulled += messages.len() as u64;

        ctx.reply(json!({
            "status": STATUS_OK,
            "messages": messages,
            "count": messages.len()
        }))?;

        info!(
            "✅ 拉取了 {} 条离线消息 / Pulled {} offline messages for {}",
            messages.len(),
            messages.len(),
            to_uid
        );
        Ok(())
    }

    /// 确认离线消息 / Acknowledge offline messages
    async fn storage_offline_ack(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("").to_string();
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

        debug!(
            "✔️  确认离线消息 / Acknowledging offline messages for {}: {:?}",
            to_uid, message_ids
        );

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

        ctx.reply(json!({
            "status": STATUS_OK,
            "removed": removed
        }))?;

        info!(
            "✅ 已确认 {} 条离线消息 / Acknowledged {} offline messages for {}",
            removed, removed, to_uid
        );
        Ok(())
    }

    /// 统计离线消息数量 / Count offline messages
    async fn storage_offline_count(&mut self, ctx: &mut Context) -> Result<()> {
        let to_uid = ctx.get_payload_str("to_uid").unwrap_or("");
        let count = self.count_offline_messages(to_uid)?;

        ctx.reply(json!({
            "status": STATUS_OK,
            "count": count
        }))?;

        Ok(())
    }

    /// 添加房间成员 / Add room member
    async fn storage_room_add_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("").to_string();
        let uid = ctx.get_payload_str("uid").unwrap_or("").to_string();

        debug!(
            "👥 添加房间成员 / Adding room member: {} to {}",
            uid, room_id
        );

        let key = Self::room_member_key(&room_id, &uid);
        self.room_members.insert(key.as_bytes(), b"1")?;
        self.room_members.flush()?;

        ctx.reply(json!({"status": STATUS_OK}))?;

        info!(
            "✅ 已添加房间成员 / Room member added: {} to {}",
            uid, room_id
        );
        Ok(())
    }

    /// 移除房间成员 / Remove room member
    async fn storage_room_remove_member(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("").to_string();
        let uid = ctx.get_payload_str("uid").unwrap_or("").to_string();

        debug!(
            "👥 移除房间成员 / Removing room member: {} from {}",
            uid, room_id
        );

        let key = Self::room_member_key(&room_id, &uid);
        self.room_members.remove(key.as_bytes())?;
        self.room_members.flush()?;

        ctx.reply(json!({"status": STATUS_OK}))?;

        info!(
            "✅ 已移除房间成员 / Room member removed: {} from {}",
            uid, room_id
        );
        Ok(())
    }

    /// 列出房间成员 / List room members
    async fn storage_room_list_members(&mut self, ctx: &mut Context) -> Result<()> {
        let room_id = ctx.get_payload_str("room_id").unwrap_or("");

        debug!("📋 列出房间成员 / Listing room members for {}", room_id);

        let members: Vec<String> = self
            .room_members
            .scan_prefix(Self::user_prefix(room_id).as_bytes())
            .filter_map(|item| item.ok())
            .filter_map(|(k, _)| String::from_utf8(k.to_vec()).ok())
            .filter_map(|ks| ks.split_once(':').map(|(_, uid)| uid.to_string()))
            .collect();

        ctx.reply(json!({
            "status": STATUS_OK,
            "members": members,
            "count": members.len()
        }))?;

        Ok(())
    }

    /// 列出所有房间 / List all rooms
    async fn storage_room_list(&mut self, ctx: &mut Context) -> Result<()> {
        debug!("📋 列出所有房间 / Listing all rooms");

        let rooms: HashSet<String> = self
            .room_members
            .iter()
            .filter_map(|item| item.ok())
            .filter_map(|(k, _)| String::from_utf8(k.to_vec()).ok())
            .filter_map(|ks| ks.split_once(':').map(|(rid, _)| rid.to_string()))
            .collect();

        let room_list: Vec<String> = rooms.into_iter().collect();

        ctx.reply(json!({
            "status": STATUS_OK,
            "rooms": room_list,
            "count": room_list.len()
        }))?;

        Ok(())
    }

    /// 记录已读回执 / Record read receipt
    async fn storage_read_record(&mut self, ctx: &mut Context) -> Result<()> {
        let uid = ctx.get_payload_str("uid").unwrap_or("");
        let message_id = ctx.get_payload_str("message_id").unwrap_or("");

        debug!(
            "✔️  记录已读回执 / Recording read receipt: {} by {}",
            message_id, uid
        );

        let key = format!("{}:{}", uid, message_id);
        let val = serde_json::to_vec(&ctx.payload)?;

        self.reads.insert(key.as_bytes(), val)?;
        self.reads.flush()?;

        ctx.reply(json!({"status": STATUS_OK}))?;

        Ok(())
    }

    /// 查询历史消息 / Query message history
    async fn storage_message_history(&mut self, ctx: &mut Context) -> Result<()> {
        let limit = ctx
            .payload
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;
        let since_ts = ctx.payload.get("since_ts").and_then(|v| v.as_i64());
        let until_ts = ctx.payload.get("until_ts").and_then(|v| v.as_i64());

        debug!(
            "📜 查询历史消息 / Querying message history, limit: {}",
            limit
        );

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
            "status": STATUS_OK,
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
            "status": STATUS_OK,
            "stats": {
                "messages_saved": self.stats.messages_saved,
                "offline_saved": self.stats.offline_saved,
                "offline_pulled": self.stats.offline_pulled,
                "offline_acked": self.stats.offline_acked,
                "db_size": self.db.size_on_disk().unwrap_or(0)
            }
        }))?;

        Ok(())
    }
}

// ============================================================================
// 辅助方法 / Helper Methods
// ============================================================================

impl SledStorageEventListener {
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

    /// 统计用户的离线消息数量 / Count offline messages for user
    fn count_offline_messages(&mut self, to_uid: &str) -> Result<usize> {
        Ok(self
            .offline
            .scan_prefix(Self::user_prefix(to_uid).as_bytes())
            .count())
    }

    /// 删除最旧的离线消息 / Remove oldest offline messages
    fn remove_oldest_offline(&mut self, to_uid: &str, count: usize) -> Result<usize> {
        let prefix = Self::user_prefix(to_uid);
        let keys_to_remove: Vec<_> = self
            .offline
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
