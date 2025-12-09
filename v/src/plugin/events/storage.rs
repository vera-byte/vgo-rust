//! # 存储事件监听器 / Storage Event Listener
//!
//! 定义存储插件的事件监听器 trait
//! Defines the event listener trait for storage plugins

use anyhow::Result;
use async_trait::async_trait;

use crate::plugin::protocol::{
    AckOfflineMessagesRequest, AckOfflineMessagesResponse, AddRoomMemberRequest,
    AddRoomMemberResponse, CountOfflineMessagesRequest, CountOfflineMessagesResponse,
    GetRoomMembersRequest, GetRoomMembersResponse, PullOfflineMessagesRequest,
    PullOfflineMessagesResponse, RemoveRoomMemberRequest, RemoveRoomMemberResponse,
    SaveMessageRequest, SaveMessageResponse, SaveOfflineMessageRequest, SaveOfflineMessageResponse,
};

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
/// use v::plugin::pdk::StorageEventListener;
/// use v::plugin::protocol::*;
/// use async_trait::async_trait;
///
/// pub struct MyStorageListener {
///     // 你的字段
/// }
///
/// #[async_trait]
/// impl StorageEventListener for MyStorageListener {
///     async fn storage_message_save(&mut self, req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
///         // 类型安全的实现
///         Ok(SaveMessageResponse {
///             status: "ok".to_string(),
///             message_id: req.message_id.clone(),
///         })
///     }
///     // ... 实现其他方法
/// }
/// ```
#[async_trait]
pub trait StorageEventListener: Send + Sync {
    /// 保存消息到持久化存储 / Save message to persistent storage
    ///
    /// # 参数 / Parameters
    /// - `req`: 保存消息请求 / Save message request
    ///
    /// # 返回 / Returns
    /// - `Result<SaveMessageResponse>`: 保存消息响应 / Save message response
    async fn storage_message_save(
        &mut self,
        req: &SaveMessageRequest,
    ) -> Result<SaveMessageResponse>;

    /// 保存离线消息 / Save offline message
    ///
    /// # 参数 / Parameters
    /// - `req`: 保存离线消息请求 / Save offline message request
    ///
    /// # 返回 / Returns
    /// - `Result<SaveOfflineMessageResponse>`: 保存离线消息响应 / Save offline message response
    async fn storage_offline_save(
        &mut self,
        req: &SaveOfflineMessageRequest,
    ) -> Result<SaveOfflineMessageResponse>;

    /// 拉取用户的离线消息 / Pull user's offline messages
    ///
    /// # 参数 / Parameters
    /// - `req`: 拉取离线消息请求 / Pull offline messages request
    ///
    /// # 返回 / Returns
    /// - `Result<PullOfflineMessagesResponse>`: 拉取离线消息响应 / Pull offline messages response
    async fn storage_offline_pull(
        &mut self,
        req: &PullOfflineMessagesRequest,
    ) -> Result<PullOfflineMessagesResponse>;

    /// 确认离线消息已读 / Acknowledge offline messages as read
    ///
    /// # 参数 / Parameters
    /// - `req`: 确认离线消息请求 / Acknowledge offline messages request
    ///
    /// # 返回 / Returns
    /// - `Result<AckOfflineMessagesResponse>`: 确认离线消息响应 / Acknowledge offline messages response
    async fn storage_offline_ack(
        &mut self,
        req: &AckOfflineMessagesRequest,
    ) -> Result<AckOfflineMessagesResponse>;

    /// 统计用户的离线消息数量 / Count user's offline messages
    ///
    /// # 参数 / Parameters
    /// - `req`: 统计离线消息请求 / Count offline messages request
    ///
    /// # 返回 / Returns
    /// - `Result<CountOfflineMessagesResponse>`: 统计离线消息响应 / Count offline messages response
    async fn storage_offline_count(
        &mut self,
        req: &CountOfflineMessagesRequest,
    ) -> Result<CountOfflineMessagesResponse>;

    /// 添加房间成员 / Add room member
    ///
    /// # 参数 / Parameters
    /// - `req`: 添加房间成员请求 / Add room member request
    ///
    /// # 返回 / Returns
    /// - `Result<AddRoomMemberResponse>`: 添加房间成员响应 / Add room member response
    async fn storage_room_add_member(
        &mut self,
        req: &AddRoomMemberRequest,
    ) -> Result<AddRoomMemberResponse>;

    /// 移除房间成员 / Remove room member
    ///
    /// # 参数 / Parameters
    /// - `req`: 移除房间成员请求 / Remove room member request
    ///
    /// # 返回 / Returns
    /// - `Result<RemoveRoomMemberResponse>`: 移除房间成员响应 / Remove room member response
    async fn storage_room_remove_member(
        &mut self,
        req: &RemoveRoomMemberRequest,
    ) -> Result<RemoveRoomMemberResponse>;

    /// 列出房间的所有成员 / List all members of a room
    ///
    /// # 参数 / Parameters
    /// - `req`: 获取房间成员请求 / Get room members request
    ///
    /// # 返回 / Returns
    /// - `Result<GetRoomMembersResponse>`: 获取房间成员响应 / Get room members response
    async fn storage_room_list_members(
        &mut self,
        req: &GetRoomMembersRequest,
    ) -> Result<GetRoomMembersResponse>;
}
