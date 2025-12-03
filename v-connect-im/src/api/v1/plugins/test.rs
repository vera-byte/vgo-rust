use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::server::VConnectIMServer;
/// 注册测试接口路由 / Register test endpoint route
pub fn register(cfg: &mut web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(test_message)));
}

/// 测试消息请求 / Test message request
#[derive(Debug, Deserialize)]
pub struct TestMessageRequest {
    /// 消息内容 / Message content
    pub content: String,
    /// 发送者 UID / Sender UID
    pub from_uid: Option<String>,
    /// 接收者 UID / Receiver UID
    pub to_uid: Option<String>,
}

/// 测试消息响应 / Test message response
#[derive(Debug, Serialize)]
pub struct TestMessageResponse {
    /// 状态 / Status
    pub status: String,
    /// 插件响应列表 / Plugin responses
    pub plugin_responses: Vec<PluginResponseInfo>,
}

/// 插件响应信息 / Plugin response info
#[derive(Debug, Serialize)]
pub struct PluginResponseInfo {
    /// 插件名称 / Plugin name
    pub plugin_name: String,
    /// 响应内容 / Response content
    pub response: serde_json::Value,
}

/// 测试插件消息分发 / Test plugin message dispatch
///
/// 发送测试消息给所有支持 message 能力的插件
/// Send test message to all plugins with message capability
async fn test_message(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<TestMessageRequest>,
) -> impl Responder {
    // 构建消息 payload / Build message payload
    let message = json!({
        "content": req.content,
        "from_uid": req.from_uid.as_deref().unwrap_or("test_user"),
        "to_uid": req.to_uid.as_deref().unwrap_or(""),
        "timestamp": chrono::Utc::now().timestamp(),
    });

    // 获取插件连接池 / Get plugin connection pool
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool.broadcast_message_event(&message).await {
            Ok(responses) => {
                let plugin_responses: Vec<PluginResponseInfo> = responses
                    .into_iter()
                    .map(|(name, response)| PluginResponseInfo {
                        plugin_name: name,
                        response,
                    })
                    .collect();

                HttpResponse::Ok().json(TestMessageResponse {
                    status: "ok".to_string(),
                    plugin_responses,
                })
            }
            Err(e) => {
                tracing::error!("Failed to broadcast message: {}", e);
                HttpResponse::InternalServerError().json(json!({
                    "status": "error",
                    "message": format!("Failed to broadcast message: {}", e)
                }))
            }
        }
    } else {
        HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": "Plugin connection pool not available"
        }))
    }
}
