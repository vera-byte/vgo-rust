//! 插件事件处理器 trait 定义 / Plugin event handler trait definition
//!
//! 提供基于 trait 的事件处理机制，替代大量 match 分支
//! Provides trait-based event handling mechanism to replace massive match branches

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// 插件事件上下文 / Plugin event context
///
/// 封装事件处理所需的上下文信息
/// Encapsulates context information needed for event handling
pub struct Context {
    /// 事件类型 / Event type
    event_type: String,
    /// 事件载荷 / Event payload
    payload: Value,
    /// 响应数据 / Response data
    response: Option<Value>,
}

impl Context {
    /// 创建新的上下文 / Create new context
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
            response: None,
        }
    }

    /// 获取事件类型 / Get event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// 获取载荷数据 / Get payload data
    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// 获取载荷中的字段 / Get field from payload
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T> {
        self.payload
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Missing field: {}", key))
            .and_then(|v| serde_json::from_value(v.clone()).map_err(Into::into))
    }

    /// 设置响应数据 / Set response data
    pub fn reply(&mut self, response: Value) -> Result<()> {
        self.response = Some(response);
        Ok(())
    }

    /// 获取响应数据 / Get response data
    pub fn response(&self) -> Option<&Value> {
        self.response.as_ref()
    }

    /// 消费上下文并返回响应 / Consume context and return response
    pub fn into_response(self) -> Option<Value> {
        self.response
    }
}

/// 存储事件处理器 trait / Storage event handler trait
///
/// 定义存储相关事件的处理方法
/// Defines handler methods for storage-related events
#[async_trait]
pub trait StorageEventHandler: Send + Sync {
    /// 处理消息保存事件 / Handle message save event
    ///
    /// 事件类型: storage.message.save
    /// Event type: storage.message.save
    async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理离线消息保存事件 / Handle offline message save event
    ///
    /// 事件类型: storage.offline.save
    /// Event type: storage.offline.save
    async fn on_offline_save(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理离线消息拉取事件 / Handle offline message pull event
    ///
    /// 事件类型: storage.offline.pull
    /// Event type: storage.offline.pull
    async fn on_offline_pull(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理离线消息确认事件 / Handle offline message ack event
    ///
    /// 事件类型: storage.offline.ack
    /// Event type: storage.offline.ack
    async fn on_offline_ack(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理离线消息计数事件 / Handle offline message count event
    ///
    /// 事件类型: storage.offline.count
    /// Event type: storage.offline.count
    async fn on_offline_count(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理房间添加成员事件 / Handle room add member event
    ///
    /// 事件类型: storage.room.add_member
    /// Event type: storage.room.add_member
    async fn on_room_add_member(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理房间移除成员事件 / Handle room remove member event
    ///
    /// 事件类型: storage.room.remove_member
    /// Event type: storage.room.remove_member
    async fn on_room_remove_member(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理房间成员列表事件 / Handle room list members event
    ///
    /// 事件类型: storage.room.list_members
    /// Event type: storage.room.list_members
    async fn on_room_list_members(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理房间列表事件 / Handle room list event
    ///
    /// 事件类型: storage.room.list
    /// Event type: storage.room.list
    async fn on_room_list(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理已读记录事件 / Handle read record event
    ///
    /// 事件类型: storage.read.record
    /// Event type: storage.read.record
    async fn on_read_record(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理消息历史事件 / Handle message history event
    ///
    /// 事件类型: storage.message.history
    /// Event type: storage.message.history
    async fn on_message_history(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 处理统计事件 / Handle stats event
    ///
    /// 事件类型: storage.stats
    /// Event type: storage.stats
    async fn on_stats(&self, ctx: &mut Context) -> Result<()> {
        ctx.reply(serde_json::json!({
            "status": "error",
            "message": "Not implemented"
        }))?;
        Ok(())
    }

    /// 分发事件到对应的处理方法 / Dispatch event to corresponding handler
    ///
    /// 这是主要的事件分发方法，会根据事件类型调用对应的处理方法
    /// This is the main event dispatch method that calls corresponding handlers based on event type
    async fn dispatch(&self, ctx: &mut Context) -> Result<()> {
        use tracing::{debug, warn};

        let event_type = ctx.event_type();
        debug!("📨 收到存储事件 / Received storage event: {}", event_type);

        // 使用 trait 方法分发，替代 match 分支
        // Use trait methods for dispatch, replacing match branches
        match event_type {
            "storage.message.save" => self.on_message_save(ctx).await?,
            "storage.offline.save" => self.on_offline_save(ctx).await?,
            "storage.offline.pull" => self.on_offline_pull(ctx).await?,
            "storage.offline.ack" => self.on_offline_ack(ctx).await?,
            "storage.offline.count" => self.on_offline_count(ctx).await?,
            "storage.room.add_member" => self.on_room_add_member(ctx).await?,
            "storage.room.remove_member" => self.on_room_remove_member(ctx).await?,
            "storage.room.list_members" => self.on_room_list_members(ctx).await?,
            "storage.room.list" => self.on_room_list(ctx).await?,
            "storage.read.record" => self.on_read_record(ctx).await?,
            "storage.message.history" => self.on_message_history(ctx).await?,
            "storage.stats" => self.on_stats(ctx).await?,
            _ => {
                warn!(
                    "⚠️  未知的存储事件类型 / Unknown storage event type: {}",
                    event_type
                );
                ctx.reply(serde_json::json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
            }
        }

        Ok(())
    }
}

/// 认证事件处理器 trait / Authentication event handler trait
///
/// 定义认证相关事件的处理方法
/// Defines handler methods for authentication-related events
#[async_trait]
pub trait AuthEventHandler: Send + Sync {
    /// 用户登录事件 / User login event
    async fn on_login(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// 用户登出事件 / User logout event
    async fn on_logout(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// 用户被踢出事件 / User kick out event
    async fn on_kick_out(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Token 续期事件 / Token renew event
    async fn on_renew_timeout(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Token 被替换事件 / Token replaced event
    async fn on_replaced(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// 用户被封禁事件 / User banned event
    async fn on_banned(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// 分发认证事件 / Dispatch auth event
    async fn dispatch(&self, ctx: &mut Context) -> Result<()> {
        use tracing::{debug, warn};

        let event_type = ctx.event_type();
        debug!("🔐 收到认证事件 / Received auth event: {}", event_type);

        match event_type {
            "auth.login" => self.on_login(ctx).await?,
            "auth.logout" => self.on_logout(ctx).await?,
            "auth.kick_out" => self.on_kick_out(ctx).await?,
            "auth.renew_timeout" => self.on_renew_timeout(ctx).await?,
            "auth.replaced" => self.on_replaced(ctx).await?,
            "auth.banned" => self.on_banned(ctx).await?,
            _ => {
                warn!(
                    "⚠️  未知的认证事件类型 / Unknown auth event type: {}",
                    event_type
                );
                ctx.reply(serde_json::json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestStorageHandler;

    #[async_trait]
    impl StorageEventHandler for TestStorageHandler {
        async fn on_message_save(&self, ctx: &mut Context) -> Result<()> {
            let message_id: String = ctx.get("message_id")?;
            ctx.reply(json!({
                "status": "ok",
                "message_id": message_id
            }))?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_storage_event_dispatch() {
        let handler = TestStorageHandler;
        let mut ctx = Context::new(
            "storage.message.save",
            json!({
                "message_id": "msg_123",
                "content": "hello"
            }),
        );

        handler.dispatch(&mut ctx).await.unwrap();

        let response = ctx.response().unwrap();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["message_id"], "msg_123");
    }

    #[tokio::test]
    async fn test_unknown_event_type() {
        let handler = TestStorageHandler;
        let mut ctx = Context::new("storage.unknown", json!({}));

        handler.dispatch(&mut ctx).await.unwrap();

        let response = ctx.response().unwrap();
        assert_eq!(response["status"], "error");
    }
}
