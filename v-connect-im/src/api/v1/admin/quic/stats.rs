use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(quic_stats_handle)));
}

pub async fn quic_stats_handle(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    respond_any(StatusCode::OK, serde_json::json!({
        "conn_count": server.quic_conn_count.load(std::sync::atomic::Ordering::Relaxed),
        "path_updates": server.quic_path_updates.load(std::sync::atomic::Ordering::Relaxed),
        "stream_sent": server.quic_stream_sent.load(std::sync::atomic::Ordering::Relaxed),
        "dgram_sent": server.quic_dgram_sent.load(std::sync::atomic::Ordering::Relaxed),
        "stream_recv": server.quic_stream_recv.load(std::sync::atomic::Ordering::Relaxed),
        "dgram_recv": server.quic_dgram_recv.load(std::sync::atomic::Ordering::Relaxed)
    }))
}

