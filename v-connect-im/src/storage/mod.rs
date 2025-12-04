//! 存储模块 - 数据结构定义
//! Storage Module - Data Structure Definitions
//!
//! ⚠️ 存储功能已完全迁移到存储插件
//! ⚠️ Storage functionality has been fully migrated to storage plugin
//!
//! 此模块仅保留数据结构定义，用于类型共享和序列化
//! This module only retains data structure definitions for type sharing and serialization
//!
//! 使用存储功能请调用 `PluginConnectionPool::storage_*` 方法
//! To use storage functionality, call `PluginConnectionPool::storage_*` methods

// ============================================================================
// 数据结构定义 / Data Structure Definitions
// ============================================================================

/// 消息记录 / Message Record
///
/// 用于消息持久化和 Raft 日志
/// Used for message persistence and Raft logs
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub from_client_id: String,
    pub to_client_id: String,
    pub content: serde_json::Value,
    pub timestamp: i64,
    pub msg_type: String,
    pub room_id: Option<String>,
}

/// 离线消息记录 / Offline Message Record
///
/// 用于离线消息队列
/// Used for offline message queue
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OfflineRecord {
    pub message_id: String,
    pub from_uid: Option<String>,
    pub to_uid: String,
    pub room_id: Option<String>,
    pub content: serde_json::Value,
    pub timestamp: i64,
    pub msg_type: String,
}

/// 已读回执 / Read Receipt
///
/// 用于消息已读状态跟踪
/// Used for message read status tracking
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReadReceipt {
    pub message_id: String,
    pub uid: String,
    pub timestamp: i64,
}
