use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::server::VConnectIMServer;

/// 注册事件总线接口路由 / Register event bus endpoint routes
pub fn register(cfg: &mut web::ServiceConfig, path: &str) {
    cfg.service(
        web::resource(path)
            .route(web::post().to(subscribe_event))
            .route(web::delete().to(unsubscribe_event))
            .route(web::put().to(publish_event)),
    );
}

/// 订阅事件请求 / Subscribe event request
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    /// 订阅者插件名称 / Subscriber plugin name
    pub subscriber: String,
    /// 事件类型模式 / Event type pattern
    pub event_pattern: String,
    /// 订阅优先级 / Subscription priority
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    10
}

/// 取消订阅请求 / Unsubscribe request
#[derive(Debug, Deserialize)]
pub struct UnsubscribeRequest {
    /// 订阅者插件名称 / Subscriber plugin name
    pub subscriber: String,
    /// 事件类型模式 / Event type pattern
    pub event_pattern: String,
}

/// 发布事件请求 / Publish event request
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    /// 发布者插件名称 / Publisher plugin name
    pub publisher: String,
    /// 事件类型 / Event type
    pub event_type: String,
    /// 事件载荷 / Event payload
    pub payload: serde_json::Value,
}

/// 订阅响应 / Subscribe response
#[derive(Debug, Serialize)]
pub struct SubscribeResponse {
    /// 状态 / Status
    pub status: String,
    /// 消息 / Message
    pub message: String,
}

/// 发布响应 / Publish response
#[derive(Debug, Serialize)]
pub struct PublishResponse {
    /// 状态 / Status
    pub status: String,
    /// 订阅者数量 / Subscriber count
    pub subscriber_count: usize,
    /// 订阅者响应 / Subscriber responses
    pub responses: Vec<SubscriberResponseInfo>,
}

/// 订阅者响应信息 / Subscriber response info
#[derive(Debug, Serialize)]
pub struct SubscriberResponseInfo {
    /// 订阅者名称 / Subscriber name
    pub subscriber: String,
    /// 响应内容 / Response content
    pub response: serde_json::Value,
}

/// 订阅事件 / Subscribe to event
///
/// POST /v1/plugins/event-bus
///
/// 插件订阅特定事件或事件模式
/// Plugin subscribes to specific event or event pattern
async fn subscribe_event(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<SubscribeRequest>,
) -> impl Responder {
    // 注意：这里需要在 VConnectIMServer 中添加 event_bus 字段
    // Note: Need to add event_bus field to VConnectIMServer
    // 暂时返回未实现错误 / Temporarily return not implemented error
    HttpResponse::NotImplemented().json(json!({
        "status": "error",
        "message": "Event bus not yet integrated with server. Please add event_bus field to VConnectIMServer."
    }))

    // 完整实现示例 / Full implementation example:
    // if let Some(event_bus) = server.event_bus.as_ref() {
    //     match event_bus
    //         .subscribe(&req.subscriber, &req.event_pattern, req.priority)
    //         .await
    //     {
    //         Ok(_) => HttpResponse::Ok().json(SubscribeResponse {
    //             status: "ok".to_string(),
    //             message: format!(
    //                 "Plugin {} subscribed to event pattern: {}",
    //                 req.subscriber, req.event_pattern
    //             ),
    //         }),
    //         Err(e) => HttpResponse::InternalServerError().json(json!({
    //             "status": "error",
    //             "message": format!("Subscribe failed: {}", e)
    //         })),
    //     }
    // } else {
    //     HttpResponse::ServiceUnavailable().json(json!({
    //         "status": "error",
    //         "message": "Event bus not available"
    //     }))
    // }
}

/// 取消订阅事件 / Unsubscribe from event
///
/// DELETE /v1/plugins/event-bus
///
/// 插件取消订阅特定事件
/// Plugin unsubscribes from specific event
async fn unsubscribe_event(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<UnsubscribeRequest>,
) -> impl Responder {
    HttpResponse::NotImplemented().json(json!({
        "status": "error",
        "message": "Event bus not yet integrated with server"
    }))
}

/// 发布事件 / Publish event
///
/// PUT /v1/plugins/event-bus
///
/// 插件发布事件给所有订阅者
/// Plugin publishes event to all subscribers
async fn publish_event(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<PublishRequest>,
) -> impl Responder {
    HttpResponse::NotImplemented().json(json!({
        "status": "error",
        "message": "Event bus not yet integrated with server"
    }))
}
