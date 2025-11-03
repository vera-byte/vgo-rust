use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

/// 性能报告响应模型
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricsReport {
    pub report: String,
}

/// 重置操作确认响应模型
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResetAck {
    pub code: i32,
    pub message: String,
}

/// 健康检查响应模型（使用完整性能指标以简化文档）
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub metrics: crate::middleware::metrics::PerformanceMetrics,
}

/// OpenAPI 文档聚合
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::metrics::get_metrics,
        crate::api::metrics::get_performance_report,
        crate::api::metrics::reset_metrics,
        crate::api::metrics::health_check_with_metrics,
    ),
    components(
        schemas(
            crate::middleware::metrics::PerformanceMetrics,
            MetricsReport,
            ResetAck,
            HealthResponse,
        )
    ),
    tags(
        (name = "Metrics", description = "系统性能指标与健康检查相关接口")
    )
)]
pub struct ApiDoc;