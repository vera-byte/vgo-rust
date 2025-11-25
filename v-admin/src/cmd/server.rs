use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
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
    api_registry::print_routes(&addr, &["Logger"]);
    info!(
        "starting {} v{} on {}-{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let server_builder = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .configure(api_registry::configure)
    })
    .bind(addr.clone())?;

    let server_builder = if let Some(w) = workers {
        if w > 0 {
            server_builder.workers(w as usize)
        } else {
            server_builder
        }
    } else {
        server_builder
    };
    let server = server_builder.shutdown_timeout(5).run();

    let pool = get_pool("default").await?;
    let _ = check_health(&pool).await?;
    info!("database group=default healthy");

    info!(
        "starting http server: bind={} workers={}",
        addr,
        workers.unwrap_or(0)
    );

    let handle = server.handle();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        handle.stop(true).await;
    });
    server.await?;
    Ok(())
}
