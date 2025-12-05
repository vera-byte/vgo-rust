use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::server::VConnectIMServer;

/// 注册插件间通信接口路由 / Register inter-plugin communication endpoint routes
pub fn register(cfg: &mut web::ServiceConfig, path: &str) {
    cfg.service(
        web::resource(path)
            .route(web::post().to(plugin_call))
            .route(web::put().to(plugin_send_message))
            .route(web::patch().to(plugin_broadcast)),
    );
}

/// 插件调用请求 / Plugin call request
#[derive(Debug, Deserialize)]
pub struct PluginCallRequest {
    /// 发送方插件名称 / Sender plugin name
    pub from_plugin: String,
    /// 接收方插件名称 / Receiver plugin name
    pub to_plugin: String,
    /// 调用的方法名 / Method name
    pub method: String,
    /// 方法参数 / Method parameters
    pub params: serde_json::Value,
}

/// 插件消息请求 / Plugin message request
#[derive(Debug, Deserialize)]
pub struct PluginMessageRequest {
    /// 发送方插件名称 / Sender plugin name
    pub from_plugin: String,
    /// 接收方插件名称 / Receiver plugin name
    pub to_plugin: String,
    /// 消息内容 / Message content
    pub message: serde_json::Value,
}

/// 插件广播请求 / Plugin broadcast request
#[derive(Debug, Deserialize)]
pub struct PluginBroadcastRequest {
    /// 发送方插件名称 / Sender plugin name
    pub from_plugin: String,
    /// 广播消息内容 / Broadcast message content
    pub message: serde_json::Value,
    /// 可选的能力过滤器 / Optional capability filter
    pub filter_capabilities: Option<Vec<String>>,
}

/// 插件调用响应 / Plugin call response
#[derive(Debug, Serialize)]
pub struct PluginCallResponse {
    /// 状态 / Status
    pub status: String,
    /// 响应数据 / Response data
    pub response: Option<serde_json::Value>,
    /// 错误信息 / Error message
    pub error: Option<String>,
}

/// 插件消息响应 / Plugin message response
#[derive(Debug, Serialize)]
pub struct PluginMessageResponse {
    /// 状态 / Status
    pub status: String,
    /// 是否送达 / Delivered
    pub delivered: bool,
    /// 错误信息 / Error message
    pub error: Option<String>,
}

/// 插件广播响应 / Plugin broadcast response
#[derive(Debug, Serialize)]
pub struct PluginBroadcastResponse {
    /// 状态 / Status
    pub status: String,
    /// 响应插件数量 / Number of responding plugins
    pub response_count: usize,
    /// 插件响应列表 / Plugin responses
    pub responses: Vec<PluginResponseInfo>,
}

/// 插件响应信息 / Plugin response info
#[derive(Debug, Serialize)]
pub struct PluginResponseInfo {
    /// 插件名称 / Plugin name
    pub plugin_name: String,
    /// 响应内容 / Response content
    pub response: serde_json::Value,
}

/// 插件 RPC 调用 / Plugin RPC call
///
/// POST /v1/plugins/inter-communication
///
/// 插件 A 直接调用插件 B 的方法
/// Plugin A directly calls Plugin B's method
async fn plugin_call(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<PluginCallRequest>,
) -> impl Responder {
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool
            .plugin_call(&req.from_plugin, &req.to_plugin, &req.method, &req.params)
            .await
        {
            Ok(Some(response)) => HttpResponse::Ok().json(PluginCallResponse {
                status: "ok".to_string(),
                response: Some(response),
                error: None,
            }),
            Ok(None) => HttpResponse::NotFound().json(PluginCallResponse {
                status: "error".to_string(),
                response: None,
                error: Some(format!("Target plugin not connected: {}", req.to_plugin)),
            }),
            Err(e) => HttpResponse::InternalServerError().json(PluginCallResponse {
                status: "error".to_string(),
                response: None,
                error: Some(format!("Plugin call failed: {}", e)),
            }),
        }
    } else {
        HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "error": "Plugin connection pool not available"
        }))
    }
}

/// 插件点对点消息 / Plugin point-to-point message
///
/// PUT /v1/plugins/inter-communication
///
/// 插件 A 向插件 B 发送消息
/// Plugin A sends message to Plugin B
async fn plugin_send_message(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<PluginMessageRequest>,
) -> impl Responder {
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool
            .plugin_send_message(&req.from_plugin, &req.to_plugin, &req.message)
            .await
        {
            Ok(delivered) => HttpResponse::Ok().json(PluginMessageResponse {
                status: "ok".to_string(),
                delivered,
                error: None,
            }),
            Err(e) => HttpResponse::InternalServerError().json(PluginMessageResponse {
                status: "error".to_string(),
                delivered: false,
                error: Some(format!("Failed to send message: {}", e)),
            }),
        }
    } else {
        HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "error": "Plugin connection pool not available"
        }))
    }
}

/// 插件广播消息 / Plugin broadcast message
///
/// PATCH /v1/plugins/inter-communication
///
/// 插件 A 向其他插件广播消息
/// Plugin A broadcasts message to other plugins
async fn plugin_broadcast(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<PluginBroadcastRequest>,
) -> impl Responder {
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool
            .plugin_broadcast(
                &req.from_plugin,
                &req.message,
                req.filter_capabilities.clone(),
            )
            .await
        {
            Ok(responses) => {
                let plugin_responses: Vec<PluginResponseInfo> = responses
                    .into_iter()
                    .map(|(name, response)| PluginResponseInfo {
                        plugin_name: name,
                        response,
                    })
                    .collect();

                HttpResponse::Ok().json(PluginBroadcastResponse {
                    status: "ok".to_string(),
                    response_count: plugin_responses.len(),
                    responses: plugin_responses,
                })
            }
            Err(e) => HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "error": format!("Broadcast failed: {}", e)
            })),
        }
    } else {
        HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "error": "Plugin connection pool not available"
        }))
    }
}
