//! # 存储事件监听器 / Storage Event Listener
//!
//! 定义存储插件的事件监听器 trait
//! Defines the event listener trait for storage plugins

use anyhow::Result;
use async_trait::async_trait;

use crate::plugin::pdk::Context;

// ============================================================================
// 存储事件监听器 Trait / Storage Event Listener Trait
// ============================================================================

/// 存储事件监听器 trait / Storage event listener trait
///
/// 定义所有存储相关事件的处理方法
/// Defines all storage-related event handler methods
///
/// # 使用示例 / Usage Example
///
/// ```ignore
/// use v::plugin::pdk::{Context, StorageEventListener};
/// use async_trait::async_trait;
///
/// pub struct MyStorageListener {
///     // 你的字段
/// }
///
/// #[async_trait]
/// impl StorageEventListener for MyStorageListener {
///     async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()> {
///         // 你的实现
///         Ok(())
///     }
///     // ... 实现其他方法
/// }
/// ```
#[async_trait]
pub trait StorageEventListener: Send + Sync {
    /// 保存消息到持久化存储 / Save message to persistent storage
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含消息数据 / Plugin context containing message data
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_message_save(&mut self, ctx: &mut Context) -> Result<()>;

    /// 保存离线消息 / Save offline message
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含离线消息数据 / Plugin context containing offline message data
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_offline_save(&mut self, ctx: &mut Context) -> Result<()>;

    /// 拉取用户的离线消息 / Pull user's offline messages
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID和拉取限制 / Plugin context containing user ID and limit
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_offline_pull(&mut self, ctx: &mut Context) -> Result<()>;

    /// 确认离线消息已读 / Acknowledge offline messages as read
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含消息ID列表 / Plugin context containing message ID list
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_offline_ack(&mut self, ctx: &mut Context) -> Result<()>;

    /// 统计用户的离线消息数量 / Count user's offline messages
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID / Plugin context containing user ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_offline_count(&mut self, ctx: &mut Context) -> Result<()>;

    /// 添加房间成员 / Add room member
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含房间ID和用户ID / Plugin context containing room ID and user ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_room_add_member(&mut self, ctx: &mut Context) -> Result<()>;

    /// 移除房间成员 / Remove room member
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含房间ID和用户ID / Plugin context containing room ID and user ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_room_remove_member(&mut self, ctx: &mut Context) -> Result<()>;

    /// 列出房间的所有成员 / List all members of a room
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含房间ID / Plugin context containing room ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_room_list_members(&mut self, ctx: &mut Context) -> Result<()>;

    /// 列出所有房间 / List all rooms
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文 / Plugin context
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_room_list(&mut self, ctx: &mut Context) -> Result<()>;

    /// 记录消息已读回执 / Record message read receipt
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID和消息ID / Plugin context containing user ID and message ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_read_record(&mut self, ctx: &mut Context) -> Result<()>;

    /// 查询历史消息 / Query message history
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含查询条件 / Plugin context containing query conditions
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_message_history(&mut self, ctx: &mut Context) -> Result<()>;

    /// 获取存储统计信息 / Get storage statistics
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文 / Plugin context
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn storage_stats(&mut self, ctx: &mut Context) -> Result<()>;

    /// 自动事件分发 / Auto event dispatch
    ///
    /// 根据事件类型自动调用对应的处理方法
    /// Automatically calls the corresponding handler method based on event type
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文 / Plugin context
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn dispatch(&mut self, ctx: &mut Context) -> Result<()> {
        let event_type = ctx.event_type();

        match event_type {
            "storage.message.save" => self.storage_message_save(ctx).await,
            "storage.offline.save" => self.storage_offline_save(ctx).await,
            "storage.offline.pull" => self.storage_offline_pull(ctx).await,
            "storage.offline.ack" => self.storage_offline_ack(ctx).await,
            "storage.offline.count" => self.storage_offline_count(ctx).await,
            "storage.room.add_member" => self.storage_room_add_member(ctx).await,
            "storage.room.remove_member" => self.storage_room_remove_member(ctx).await,
            "storage.room.list_members" => self.storage_room_list_members(ctx).await,
            "storage.room.list" => self.storage_room_list(ctx).await,
            "storage.read.record" => self.storage_read_record(ctx).await,
            "storage.message.history" => self.storage_message_history(ctx).await,
            "storage.stats" => self.storage_stats(ctx).await,
            _ => {
                crate::warn!(
                    "⚠️  未知的存储事件类型 / Unknown storage event type: {}",
                    event_type
                );
                ctx.reply(serde_json::json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
                Ok(())
            }
        }
    }
}
