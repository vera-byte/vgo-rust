use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow;
use sa_token_plugin_actix_web::{SaTokenMiddleware, SaTokenState};
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, instrument, warn};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::comm::config::get_global_config_manager;
use crate::comm::port::{available_port, is_port_available_sync};
use crate::conf::init_sa_token;
use crate::error::{AppError, AppResult};
use crate::middleware::metrics::{MetricsMiddleware, PerformanceMonitor};
use crate::route_registry::configure_global_routes;
use std::sync::Arc;

/// 应用配置结构体
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    #[allow(dead_code)]
    pub debug: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            workers: Some(8),
            debug: false,
        }
    }
}

/// 应用启动器
pub struct AppBootstrap {
    config: Option<AppConfig>,
}

impl AppBootstrap {
    /// 创建新的应用启动器
    pub fn new() -> Self {
        Self { config: None }
    }

    /// 设置配置
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 设置主机地址
    #[allow(dead_code)]
    pub fn with_host(mut self, host: String) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.host = host;
        self.config = Some(config);
        self
    }

    /// 设置端口
    #[allow(dead_code)]
    pub fn with_port(mut self, port: u16) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.port = port;
        self.config = Some(config);
        self
    }

    /// 设置工作线程数
    #[allow(dead_code)]
    pub fn with_workers(mut self, workers: usize) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.workers = Some(workers);
        self.config = Some(config);
        self
    }

    /// 运行应用
    /// 运行应用服务器
    #[instrument(skip(self))]
    pub async fn run(self) -> AppResult<()> {
        let config = self.config.clone().unwrap_or_default();
        info!("启动应用服务器，配置: {:?}", config);

        // 创建配置管理器（使用全局单例）
        let config_manager = get_global_config_manager().map_err(|e| {
            AppError::Config(crate::comm::config::ConfigError::InitializationError {
                message: e.to_string(),
            })
        })?;

        // 打印配置源信息
        config_manager.print_sources_info();

        // 使用便捷方法
        info!(
            "日志级别: {}",
            config_manager
                .get_string("logging.level")
                .unwrap_or("info".to_string())
        );

        // 初始化日志
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let formatting_layer = BunyanFormattingLayer::new("vgo-rust".into(), std::io::stdout);
        let subscriber = Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer);
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

        // 初始化 Sa-Token（带超时和重试）
        let sa_token_manager = self.init_sa_token_with_retry().await?;
        let sa_token_state = SaTokenState {
            manager: sa_token_manager.clone(),
        };
        let sa_token_data = web::Data::new(sa_token_state.clone());

        // 检查端口可用性并获取可用端口
        let server_port = if is_port_available_sync(config.port) {
            config.port
        } else {
            warn!("端口 {} 不可用，正在寻找可用端口...", config.port);
            available_port(config.port)
        };

        info!("服务器将在端口 {} 上启动", server_port);

        // 启动 HTTP 服务器
        let server_result = self
            .start_http_server(config, server_port, sa_token_data)
            .await;

        match server_result {
            Ok(_) => {
                info!("服务器成功启动");
                Ok(())
            }
            Err(e) => {
                error!("服务器启动失败: {}", e);
                Err(e)
            }
        }
    }

    /// 带重试机制的Sa-Token初始化
    async fn init_sa_token_with_retry(
        &self,
    ) -> AppResult<std::sync::Arc<sa_token_core::SaTokenManager>> {
        const MAX_RETRIES: u32 = 3;
        const TIMEOUT_DURATION: Duration = Duration::from_secs(30);

        for attempt in 1..=MAX_RETRIES {
            info!("Sa-Token初始化尝试 {}/{}", attempt, MAX_RETRIES);

            let init_result = timeout(TIMEOUT_DURATION, init_sa_token(None)).await;

            match init_result {
                Ok(Ok(manager)) => {
                    info!("Sa-Token初始化成功");
                    return Ok(manager);
                }
                Ok(Err(e)) => {
                    warn!("Sa-Token初始化失败 (尝试 {}): {}", attempt, e);
                    if attempt == MAX_RETRIES {
                        return Err(AppError::external_service("sa-token", e.to_string()));
                    }
                }
                Err(_) => {
                    warn!("Sa-Token初始化超时 (尝试 {})", attempt);
                    if attempt == MAX_RETRIES {
                        return Err(AppError::timeout("sa-token initialization"));
                    }
                }
            }

            // 指数退避
            let delay = Duration::from_millis(1000 * 2_u64.pow(attempt - 1));
            info!("等待 {:?} 后重试", delay);
            sleep(delay).await;
        }

        unreachable!()
    }

    /// 启动HTTP服务器
    async fn start_http_server(
        &self,
        config: AppConfig,
        server_port: u16,
        sa_token_data: web::Data<SaTokenState>,
    ) -> AppResult<()> {
        // 初始化性能监控器（全局共享）
        let monitor = Arc::new(PerformanceMonitor::new());

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .app_data(sa_token_data.clone())
                .wrap(SaTokenMiddleware::new(sa_token_data.get_ref().clone()))
                // 注入性能监控器到应用状态
                .app_data(web::Data::new(monitor.clone()))
                // 接入性能监控中间件
                .wrap(MetricsMiddleware::new(monitor.clone()))
                // 集成 Swagger UI 文档（使用通配路径以兼容静态资源与尾随斜杠）
                .service(SwaggerUi::new("/swagger-ui/{_:.*}").url(
                    "/api-doc/openapi.json",
                    crate::api::swagger::ApiDoc::openapi(),
                ))
                // 配置全局路由
                .configure(configure_global_routes)
        });
        if let Some(workers) = config.workers {
            server = server.workers(workers);
        }

        server
            .bind(format!("{}:{}", config.host, server_port))
            .map_err(|e| AppError::Internal(anyhow::Error::new(e)))?
            .run()
            .await
            .map_err(|e| AppError::Internal(anyhow::Error::new(e)))?;

        Ok(())
    }
}

impl Default for AppBootstrap {
    fn default() -> Self {
        Self::new()
    }
}
