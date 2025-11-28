use crate::plugins::runtime::PluginRuntimeSummary;
use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use serde::Serialize;
use std::sync::Arc;
use v::response::respond_any;

/// 注册运行中插件列表接口（GET）/ Register runtime plugin list endpoint (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(runtime_plugins_list)));
}

#[derive(Serialize)]
struct RuntimePluginView {
    /// 插件名称 / Plugin name
    name: String,
    /// 插件版本（可能为空）/ Optional plugin version
    version: Option<String>,
    /// 当前运行状态 / Current runtime status string
    status: String,
}

/// 获取运行时插件列表 / Fetch runtime plugin list
async fn runtime_plugins_list(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    if let Some(manager) = &server.plugin_runtime_manager {
        let payload: Vec<RuntimePluginView> = manager
            .runtime_summaries()
            .into_iter()
            .map(plugin_summary_to_view)
            .collect();
        respond_any(StatusCode::OK, serde_json::json!({ "plugins": payload }))
    } else {
        respond_any(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "Plugin runtime manager not initialized" }),
        )
    }
}

fn plugin_summary_to_view(summary: PluginRuntimeSummary) -> RuntimePluginView {
    RuntimePluginView {
        name: summary.name,
        version: summary.version,
        status: format!("{:?}", summary.status),
    }
}

