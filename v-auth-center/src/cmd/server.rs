use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use diesel::RunQueryDsl;
use sa_token_plugin_actix_web::{SaTokenMiddleware, SaTokenState};
use tracing::info;
use v::db::database::{DatabaseManager, DbPool};
use v_auth_center::config::sa_token_conf::{init_sa_token, RedisConfig};
mod api_registry {
    include!(concat!(env!("OUT_DIR"), "/api_registry.rs"));
}

#[tokio::main]
async fn main() -> Result<()> {
    v::init_tracing()?;

    let host: String = v::get_config("server.host").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: i64 = v::get_config("server.port").unwrap_or(3000_i64);
    let workers: Option<i64> = v::get_config("server.workers").ok();

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
    let redis_config = v::get_config_safe::<RedisConfig>("redis").ok();
    let sa_token_manager = init_sa_token(redis_config.as_ref())
        .await
        .expect("Sa-Token initialization failed"); // Sa-Token initialization failed ｜Sa-Token 初始化失败

    // 创建 Sa-Token 状态
    // Create Sa-Token state
    let sa_token_state = SaTokenState {
        manager: sa_token_manager.clone(),
    };

    let sa_token_data = web::Data::new(sa_token_state.clone());

    tracing::info!(" Sa-Token initialized successfully"); // Sa-Token initialized successfully | Sa-Token 初始化成功

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

    match DatabaseManager::get_any_pool_by_group("default").await {
        Ok(pool) => {
            match pool {
                DbPool::Postgres(p) => {
                    let mut ok = false;
                    let r = tokio::task::spawn_blocking(move || {
                        let mut conn = p.get();
                        match conn {
                            Ok(mut c) => {
                                let _ = diesel::sql_query("SELECT 1").execute(&mut c);
                                ok = true;
                            }
                            Err(_) => {}
                        }
                        ok
                    })
                    .await
                    .unwrap_or(false);
                    if !r {
                        anyhow::bail!("database default not healthy");
                    }
                }
                _ => {}
            }
            info!("database group=default healthy");
        }
        Err(e) => {
            anyhow::bail!("database init failed: {}", e);
        }
    }

    info!(
        "starting http server: bind={} workers={}",
        addr,
        workers.unwrap_or(0)
    );
    server.bind(addr)?.run().await?;
    Ok(())
}
