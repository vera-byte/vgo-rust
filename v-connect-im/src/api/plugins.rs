use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info};
use v::response::respond_any;

use crate::plugins::bridge::RemotePluginSummary;
use crate::VConnectIMServer;

#[derive(Deserialize)]
pub struct RegisterPayload {
    pub name: String,
    pub callback_url: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Deserialize)]
pub struct HeartbeatPayload {
    pub token: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/plugins")
            .route("/register", web::post().to(register_plugin))
            .route("/{plugin_id}/reconnect", web::post().to(reconnect_plugin))
            .route("/{plugin_id}/heartbeat", web::post().to(heartbeat_plugin))
            .route("/{plugin_id}/ack", web::post().to(ack_event))
            .route("/{plugin_id}/config", web::get().to(get_config))
            .route("/{plugin_id}/stop", web::post().to(stop_plugin))
            .route("", web::get().to(list_plugins)),
    );
}

/// 注册远程插件 / Register remote plugin
async fn register_plugin(
    server: web::Data<Arc<VConnectIMServer>>,
    payload: web::Json<RegisterPayload>,
) -> impl Responder {
    let info = server.remote_plugins.register(
        payload.name.clone(),
        payload.callback_url.clone(),
        payload.capabilities.clone(),
    );
    info!(
        "🔌 Plugin registered: id={} name={} callback={}",
        info.plugin_id, payload.name, payload.callback_url
    );
    respond_any(
        StatusCode::OK,
        serde_json::json!({
            "plugin_id": info.plugin_id,
            "token": info.token,
            "callback_url": info.callback_url,
        }),
    )
}

/// 插件重连请求体 / Reconnect plugin payload
#[derive(Deserialize)]
pub struct ReconnectPayload {
    pub token: String,
    pub callback_url: String,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
}

/// 插件重连（更新 callback_url）/ Reconnect plugin (update callback_url)
async fn reconnect_plugin(
    server: web::Data<Arc<VConnectIMServer>>,
    path: web::Path<String>,
    payload: web::Json<ReconnectPayload>,
) -> impl Responder {
    let plugin_id = path.into_inner();
    match server.remote_plugins.reconnect(
        &plugin_id,
        &payload.token,
        payload.callback_url.clone(),
        payload.capabilities.clone(),
    ) {
        Ok(info) => {
            info!(
                "🔄 Plugin reconnected: id={} callback={}",
                plugin_id, payload.callback_url
            );
            respond_any(
                StatusCode::OK,
                serde_json::json!({
                    "plugin_id": info.plugin_id,
                    "callback_url": info.callback_url,
                    "status": "reconnected",
                }),
            )
        }
        Err(e) => respond_any(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "error": e.to_string() }),
        ),
    }
}

/// 插件心跳 / Plugin heartbeat
async fn heartbeat_plugin(
    server: web::Data<Arc<VConnectIMServer>>,
    path: web::Path<String>,
    payload: web::Json<HeartbeatPayload>,
) -> impl Responder {
    let plugin_id = path.into_inner();
    match server.remote_plugins.heartbeat(&plugin_id, &payload.token) {
        Ok(_) => {
            debug!("💓 Plugin heartbeat: id={}", plugin_id);
            respond_any(StatusCode::OK, serde_json::json!({"status":"ok"}))
        }
        Err(e) => respond_any(
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": e.to_string()}),
        ),
    }
}

async fn list_plugins(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let list: Vec<RemotePluginSummary> = server.remote_plugins.list();
    respond_any(StatusCode::OK, serde_json::json!({ "plugins": list }))
}

#[derive(Deserialize)]
pub struct AckPayload {
    pub token: String,
    pub event_id: String,
}

/// 事件确认 / Event acknowledgment
async fn ack_event(
    server: web::Data<Arc<VConnectIMServer>>,
    path: web::Path<String>,
    payload: web::Json<AckPayload>,
) -> impl Responder {
    let plugin_id = path.into_inner();
    match server
        .remote_plugins
        .ack_event(&plugin_id, &payload.token, &payload.event_id)
    {
        Ok(event_type) => {
            debug!(
                "✅ Event acked: plugin={} event_id={} type={}",
                plugin_id, payload.event_id, event_type
            );
            respond_any(
                StatusCode::OK,
                serde_json::json!({ "status": "ok", "event_type": event_type }),
            )
        }
        Err(e) => respond_any(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "error": e.to_string() }),
        ),
    }
}

#[derive(Deserialize)]
pub struct ConfigQuery {
    pub token: String,
}

async fn get_config(
    server: web::Data<Arc<VConnectIMServer>>,
    path: web::Path<String>,
    query: web::Query<ConfigQuery>,
) -> impl Responder {
    match server
        .remote_plugins
        .validate(&path.into_inner(), &query.token)
    {
        Ok(_) => respond_any(
            StatusCode::OK,
            serde_json::json!({ "config": server.get_plugin_config() }),
        ),
        Err(e) => respond_any(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({ "error": e.to_string() }),
        ),
    }
}

#[derive(Deserialize)]
pub struct StopPayload {
    pub token: String,
}

/// 停止插件 / Stop plugin
async fn stop_plugin(
    server: web::Data<Arc<VConnectIMServer>>,
    path: web::Path<String>,
    payload: web::Json<StopPayload>,
) -> impl Responder {
    let plugin_id = path.into_inner();
    match server
        .remote_plugins
        .unregister(&plugin_id, &payload.token)
    {
        Ok(_) => {
            info!("🛑 Plugin stopped: id={}", plugin_id);
            respond_any(
                StatusCode::OK,
                serde_json::json!({ "status": "stopped", "plugin_id": plugin_id }),
            )
        }
        Err(e) => respond_any(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "error": e.to_string() }),
        ),
    }
}
