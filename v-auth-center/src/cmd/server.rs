use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use sa_token_plugin_actix_web::{SaTokenMiddleware, SaTokenState};
use tracing::info;
use v_auth_center::config::sa_token_conf;
mod controller_registry {
    include!(concat!(env!("OUT_DIR"), "/controller_registry.rs"));
}

#[tokio::main]
async fn main() -> Result<()> {
    v::init_tracing()?;

    let host: String = v::get_config("server.host").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: i64 = v::get_config("server.port").unwrap_or(3000_i64);
    let workers: Option<i64> = v::get_config("server.workers").ok();

    let storage_type: String =
        v::get_config("sa_token.storage").unwrap_or_else(|_| "memory".to_string());
    let timeout_seconds: i64 = v::get_config("sa_token.timeout_seconds").unwrap_or(7200_i64);
    let token_name: String =
        v::get_config("sa_token.token_name").unwrap_or_else(|_| "satoken".to_string());

    let addr = format!("{}:{}", host, port);

    info!(
        "starting {} v{} on {}-{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    // 1. 初始化 Sa-Token (StpUtil会自动初始化)
    // 1. Initialize Sa-Token (StpUtil will be automatically initialized)
    let sa_token_manager = sa_token_conf::init_sa_token(None)
        .await
        .expect("Sa-Token initialization failed"); // Sa-Token initialization failed ｜Sa-Token 初始化失败

    // 创建 Sa-Token 状态
    // Create Sa-Token state
    let sa_token_state = SaTokenState {
        manager: sa_token_manager.clone(),
    };

    let sa_token_data = web::Data::new(sa_token_state.clone());

    tracing::info!(" Sa-Token initialized successfully"); // Sa-Token initialized successfully | Sa-Token 初始化成功

    info!(
        "sa-token initialized: storage={} timeout={} token={}",
        storage_type, timeout_seconds, token_name
    );

    let server = HttpServer::new(move || {
        App::new()
            .app_data(sa_token_data.clone()) // 注入 Sa-Token 到应用状态 / Inject Sa-Token into application state
            .wrap(SaTokenMiddleware::new(sa_token_state.clone()))
            .wrap(Logger::default())
            .configure(controller_registry::configure)
    });

    let server = if let Some(w) = workers {
        if w > 0 {
            server.workers(w as usize)
        } else {
            server
        }
    } else {
        server
    };

    info!(
        "starting http server: bind={} workers={}",
        addr,
        workers.unwrap_or(0)
    );
    server.bind(addr)?.run().await?;
    Ok(())
}
