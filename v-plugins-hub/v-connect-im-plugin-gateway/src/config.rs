//! 网关配置模块 / Gateway Configuration Module

use serde::{Deserialize, Serialize};

/// 网关配置 / Gateway Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// 监听地址 / Listen host
    pub host: String,

    /// 监听端口 / Listen port
    pub port: u16,

    /// 工作线程数 / Worker threads
    pub workers: usize,

    /// 是否启用 OpenAPI 文档 / Enable OpenAPI documentation
    pub enable_openapi: bool,

    /// 主服务地址 / Main service address
    pub main_service_url: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 4,
            enable_openapi: true,
            main_service_url: "http://localhost:9000".to_string(),
        }
    }
}
