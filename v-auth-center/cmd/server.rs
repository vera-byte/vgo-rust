use anyhow::Result;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use sa_token_core::SaTokenConfig;
use sa_token_storage_memory::MemoryStorage;
use std::sync::Arc;

mod event { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/event/mod.rs")); }
mod controller_registry { include!(concat!(env!("OUT_DIR"), "/controller_registry.rs")); }

#[tokio::main]
async fn main() -> Result<()> {
    let host: String = v::get_config("server.host").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: i64 = v::get_config("server.port").unwrap_or(3000_i64);
    let workers: Option<i64> = v::get_config("server.workers").ok();

    let storage_type: String = v::get_config("sa_token.storage").unwrap_or_else(|_| "memory".to_string());
    let timeout_seconds: i64 = v::get_config("sa_token.timeout_seconds").unwrap_or(7200_i64);
    let token_name: String = v::get_config("sa_token.token_name").unwrap_or_else(|_| "satoken".to_string());

    let addr = format!("{}:{}", host, port);

    let mut builder = SaTokenConfig::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .timeout(timeout_seconds)
        .token_name(token_name);

    builder = builder.register_listener(Arc::new(event::MyListener));
    builder.build();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .configure(controller_registry::configure)
    });

    let server = if let Some(w) = workers { if w > 0 { server.workers(w as usize) } else { server } } else { server };

    server.bind(addr)?.run().await?;
    Ok(())
}
