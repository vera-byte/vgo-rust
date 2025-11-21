use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use thiserror::Error;
use tracing::info;
use v::db::connection::{check_health, get_pool};

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
    // 打印路由信息 / Print route information
    api_registry::print_routes(&addr, &["Logger", "SaTokenMiddleware"]);
    info!(
        "starting {} v{} on {}-{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let server = HttpServer::new(move || {
        App::new()
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
