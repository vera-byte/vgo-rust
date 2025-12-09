//! # Sled 存储事件监听器实现 / Sled Storage Event Listener Implementation
//!
//! 基于 Sled 嵌入式数据库的存储事件监听器实现
//! Storage event listener implementation based on Sled embedded database

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use v::plugin::pdk::StorageEventListener;
use v::plugin::protocol::*;
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

impl SledStorageConfig {
    /// 验证配置有效性 / Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.db_path.is_empty() {
            anyhow::bail!("db_path 不能为空 / db_path cannot be empty");
        }

        if self.max_offline_messages == 0 {
            anyhow::bail!(
                "max_offline_messages 必须大于 0 / max_offline_messages must be greater than 0"
            );
        }

        if self.max_offline_messages > 1_000_000 {
            warn!("⚠️  max_offline_messages 过大可能影响性能 / Large max_offline_messages may affect performance: {}", self.max_offline_messages);
        }

        Ok(())
    }
}

// ============================================================================
// 统计信息 / Statistics
// ============================================================================

/// 存储统计信息 / Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    /// 已保存消息数 / Messages saved
    pub messages_saved: u64,
    /// 已保存离线消息数 / Offline messages saved
    pub offline_saved: u64,
    /// 已拉取离线消息数 / Offline messages pulled
    pub offline_pulled: u64,
    /// 已确认离线消息数 / Offline messages acknowledged
    pub offline_acked: u64,
}

// ============================================================================
// 主结构 / Main Structure
// ============================================================================

/// Sled 存储事件监听器 / Sled storage event listener
pub struct SledStorageEventListener {
    /// WAL 树（消息日志）/ WAL tree (message log)
    wal: sled::Tree,
    /// 离线消息树 / Offline messages tree
    offline: sled::Tree,
    /// 房间成员树 / Room members tree
    rooms: sled::Tree,
    /// 配置 / Configuration
    pub config: SledStorageConfig,
    /// 统计信息 / Statistics
    stats: StorageStats,
}

impl SledStorageEventListener {
    /// 创建新实例 / Create new instance
    pub fn new(config: SledStorageConfig) -> Result<Self> {
        info!("🚀 初始化 Sled 存储 / Initializing Sled storage");

        // 打开数据库 / Open database
        let db = sled::open(&config.db_path)?;

        // 打开树 / Open trees
        let wal = db.open_tree("wal")?;
        let offline = db.open_tree("offline")?;
        let rooms = db.open_tree("rooms")?;

        info!(
            "✅ Sled 存储初始化完成 / Sled storage initialized: {}",
            config.db_path
        );

        Ok(Self {
            wal,
            offline,
            rooms,
            config,
            stats: StorageStats::default(),
        })
    }

    /// 统计离线消息数量 / Count offline messages
    fn count_offline_messages(&self, uid: &str) -> Result<usize> {
        let prefix = format!("{}:", uid);
        Ok(self.offline.scan_prefix(prefix.as_bytes()).count())
    }

    /// 移除最旧的离线消息 / Remove oldest offline messages
    fn remove_oldest_offline(&self, uid: &str, count: usize) -> Result<()> {
        let prefix = format!("{}:", uid);
        let keys: Vec<_> = self
            .offline
            .scan_prefix(prefix.as_bytes())
            .take(count)
            .filter_map(|r| r.ok().map(|(k, _)| k))
            .collect();

        for key in keys {
            self.offline.remove(key)?;
        }

        Ok(())
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
    async fn storage_message_save(
        &mut self,
        req: &SaveMessageRequest,
    ) -> Result<SaveMessageResponse> {
        debug!(
            "💾 保存消息 / Saving message: {} from {} to {}",
            req.message_id, req.from_uid, req.to_uid
        );

        // 构建键：timestamp:message_id / Build key: timestamp:message_id
        let key = format!("{}:{}", req.timestamp, req.message_id);

        // 序列化消息数据 / Serialize message data
        let value = serde_json::json!({
            "message_id": req.message_id,
            "from_uid": req.from_uid,
            "to_uid": req.to_uid,
            "content": req.content,
            "timestamp": req.timestamp,
            "msg_type": req.msg_type,
        });
        let val = serde_json::to_vec(&value)?;

        // 保存到 WAL / Save to WAL
        self.wal.insert(key.as_bytes(), val)?;
        self.wal.flush()?;

        self.stats.messages_saved += 1;

        info!("✅ 消息已保存 / Message saved: {}", req.message_id);

        Ok(SaveMessageResponse {
            status: STATUS_OK.to_string(),
            message_id: req.message_id.clone(),
        })
    }

    /// 保存离线消息 / Save offline message
    async fn storage_offline_save(
        &mut self,
        req: &SaveOfflineMessageRequest,
    ) -> Result<SaveOfflineMessageResponse> {
        debug!(
            "💾 保存离线消息 / Saving offline message: {} for {}",
            req.message_id, req.to_uid
        );

        // 检查离线消息数量限制 / Check offline message limit
        let count = self.count_offline_messages(&req.to_uid)?;
        if count >= self.config.max_offline_messages {
            warn!(
                "⚠️  用户 {} 的离线消息已达上限 / User {} reached offline message limit",
                req.to_uid, req.to_uid
            );
            self.remove_oldest_offline(&req.to_uid, 1)?;
        }

        // 构建键：to_uid:timestamp:message_id / Build key
        let key = format!("{}:{}:{}", req.to_uid, req.timestamp, req.message_id);

        // 序列化消息数据 / Serialize message data
        let value = serde_json::json!({
            "message_id": req.message_id,
            "to_uid": req.to_uid,
            "from_uid": req.from_uid,
            "content": req.content,
            "timestamp": req.timestamp,
        });
        let val = serde_json::to_vec(&value)?;

        // 保存到离线消息树 / Save to offline tree
        self.offline.insert(key.as_bytes(), val)?;
        self.offline.flush()?;

        self.stats.offline_saved += 1;

        info!(
            "✅ 离线消息已保存 / Offline message saved: {} for {}",
            req.message_id, req.to_uid
        );

        Ok(SaveOfflineMessageResponse {
            status: STATUS_OK.to_string(),
            message_id: req.message_id.clone(),
        })
    }

    /// 拉取离线消息 / Pull offline messages
    async fn storage_offline_pull(
        &mut self,
        req: &PullOfflineMessagesRequest,
    ) -> Result<PullOfflineMessagesResponse> {
        debug!(
            "📤 拉取离线消息 / Pulling offline messages for {}, limit: {}",
            req.uid, req.limit
        );

        let prefix = format!("{}:", req.uid);
        let messages: Vec<OfflineMessage> = self
            .offline
            .scan_prefix(prefix.as_bytes())
            .take(req.limit as usize)
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| {
                serde_json::from_slice::<serde_json::Value>(&v)
                    .ok()
                    .and_then(|val| {
                        Some(OfflineMessage {
                            message_id: val.get("message_id")?.as_str()?.to_string(),
                            from_uid: val.get("from_uid")?.as_str()?.to_string(),
                            content: val.get("content")?.as_str()?.to_string(),
                            timestamp: val.get("timestamp")?.as_i64()?,
                        })
                    })
            })
            .collect();

        let total = messages.len() as i32;
        self.stats.offline_pulled += total as u64;

        info!(
            "✅ 已拉取 {} 条离线消息 / Pulled {} offline messages for {}",
            total, total, req.uid
        );

        Ok(PullOfflineMessagesResponse {
            status: STATUS_OK.to_string(),
            messages,
            total,
        })
    }

    /// 确认离线消息 / Acknowledge offline messages
    async fn storage_offline_ack(
        &mut self,
        req: &AckOfflineMessagesRequest,
    ) -> Result<AckOfflineMessagesResponse> {
        debug!(
            "✅ 确认离线消息 / Acknowledging offline messages for {}: {} messages",
            req.uid,
            req.message_ids.len()
        );

        let mut count = 0;
        for message_id in &req.message_ids {
            // 查找并删除消息 / Find and delete message
            let prefix = format!("{}:", req.uid);
            for result in self.offline.scan_prefix(prefix.as_bytes()) {
                if let Ok((key, _)) = result {
                    if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                        if key_str.ends_with(message_id) {
                            self.offline.remove(&key)?;
                            count += 1;
                            break;
                        }
                    }
                }
            }
        }

        self.offline.flush()?;
        self.stats.offline_acked += count as u64;

        info!(
            "✅ 已确认 {} 条离线消息 / Acknowledged {} offline messages for {}",
            count, count, req.uid
        );

        Ok(AckOfflineMessagesResponse {
            status: STATUS_OK.to_string(),
            count,
        })
    }

    /// 统计离线消息数量 / Count offline messages
    async fn storage_offline_count(
        &mut self,
        req: &CountOfflineMessagesRequest,
    ) -> Result<CountOfflineMessagesResponse> {
        debug!(
            "📊 统计离线消息 / Counting offline messages for: {}",
            req.uid
        );

        let count = self.count_offline_messages(&req.uid)? as i32;

        info!(
            "✅ 离线消息数量 / Offline message count: {} for {}",
            count, req.uid
        );

        Ok(CountOfflineMessagesResponse {
            status: STATUS_OK.to_string(),
            count,
        })
    }

    /// 添加房间成员 / Add room member
    async fn storage_room_add_member(
        &mut self,
        req: &AddRoomMemberRequest,
    ) -> Result<AddRoomMemberResponse> {
        debug!(
            "➕ 添加房间成员 / Adding member {} to room {}",
            req.uid, req.room_id
        );

        let key = format!("{}:members", req.room_id);

        // 获取现有成员列表 / Get existing members
        let mut members: HashSet<String> = if let Some(data) = self.rooms.get(key.as_bytes())? {
            serde_json::from_slice(&data).unwrap_or_default()
        } else {
            HashSet::new()
        };

        // 添加新成员 / Add new member
        members.insert(req.uid.clone());

        // 保存更新后的成员列表 / Save updated members
        let val = serde_json::to_vec(&members)?;
        self.rooms.insert(key.as_bytes(), val)?;
        self.rooms.flush()?;

        info!(
            "✅ 成员已添加 / Member added: {} to room {}",
            req.uid, req.room_id
        );

        Ok(AddRoomMemberResponse {
            status: STATUS_OK.to_string(),
        })
    }

    /// 移除房间成员 / Remove room member
    async fn storage_room_remove_member(
        &mut self,
        req: &RemoveRoomMemberRequest,
    ) -> Result<RemoveRoomMemberResponse> {
        debug!(
            "➖ 移除房间成员 / Removing member {} from room {}",
            req.uid, req.room_id
        );

        let key = format!("{}:members", req.room_id);

        // 获取现有成员列表 / Get existing members
        let mut members: HashSet<String> = if let Some(data) = self.rooms.get(key.as_bytes())? {
            serde_json::from_slice(&data).unwrap_or_default()
        } else {
            HashSet::new()
        };

        // 移除成员 / Remove member
        members.remove(&req.uid);

        // 保存更新后的成员列表 / Save updated members
        let val = serde_json::to_vec(&members)?;
        self.rooms.insert(key.as_bytes(), val)?;
        self.rooms.flush()?;

        info!(
            "✅ 成员已移除 / Member removed: {} from room {}",
            req.uid, req.room_id
        );

        Ok(RemoveRoomMemberResponse {
            status: STATUS_OK.to_string(),
        })
    }

    /// 获取房间成员列表 / Get room members
    async fn storage_room_list_members(
        &mut self,
        req: &GetRoomMembersRequest,
    ) -> Result<GetRoomMembersResponse> {
        debug!("📋 获取房间成员 / Getting members of room: {}", req.room_id);

        let key = format!("{}:members", req.room_id);

        // 获取成员列表 / Get members
        let members: Vec<String> = if let Some(data) = self.rooms.get(key.as_bytes())? {
            let set: HashSet<String> = serde_json::from_slice(&data).unwrap_or_default();
            set.into_iter().collect()
        } else {
            Vec::new()
        };

        info!(
            "✅ 房间成员数量 / Room member count: {} for room {}",
            members.len(),
            req.room_id
        );

        Ok(GetRoomMembersResponse {
            status: STATUS_OK.to_string(),
            members,
        })
    }
}
