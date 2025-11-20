use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use thiserror::Error;
use sa_token_plugin_actix_web::{SaTokenMiddleware, SaTokenState};
use tracing::info;
use v::db::connection::{check_health, get_pool};
use v_auth_center::config::sa_token_conf::init_sa_token;
mod api_registry {
    include!(concat!(env!("OUT_DIR"), "/api_registry.rs"));
}

#[derive(Debug, Error)]
enum AppError {
    #[error("配置错误: {0}")]
    Config(#[from] v::comm::config::ConfigError),
    #[error("数据库错误: {0}")]
    Db(#[from] v::db::error::DbError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cm = v::get_global_config_manager()?;
    cm.print_sources_info();
    v::init_tracing();

    let host: String = cm
        .get_string("server.host")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: i64 = cm.get_int("server.port").unwrap_or(3000_i64);
    let workers: Option<i64> = cm.get_int("server.workers").ok();

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
    // 获取 Redis 配置（可选） / Optional Redis config
    let sa_token_manager = init_sa_token()
        .await
        .expect("Sa-Token initialization failed"); // Sa-Token initialization failed ｜Sa-Token 初始化失败

    // 创建 Sa-Token 状态
    // Create Sa-Token state
    let sa_token_state = SaTokenState {
        manager: sa_token_manager.clone(),
    };

    let sa_token_data = web::Data::new(sa_token_state.clone());

    info!(" Sa-Token initialized successfully"); // Sa-Token initialized successfully | Sa-Token 初始化成功

    // 打印路由信息 / Print route information
    api_registry::print_routes(&addr, &["Logger", "SaTokenMiddleware"]);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(sa_token_data.clone()) // 注入 Sa-Token 到应用状态 / Inject Sa-Token into application state
            .wrap(SaTokenMiddleware::new(sa_token_state.clone()))
            .wrap(Logger::default())
            .configure(api_registry::configure)
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

    let pool = get_pool("default").await?;
    let _ = check_health(&pool).await?;
    info!("database group=default healthy");

    info!(
        "starting http server: bind={} workers={}",
        addr,
        workers.unwrap_or(0)
    );
    server.bind(addr)?.run().await?;
    Ok(())
}
