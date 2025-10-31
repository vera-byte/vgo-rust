use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use crate::middleware::metrics::PerformanceMonitor;
use std::sync::Arc;

/// 获取性能指标
pub async fn get_metrics(monitor: web::Data<Arc<PerformanceMonitor>>) -> Result<HttpResponse> {
    let metrics = monitor.get_metrics();
    
    Ok(HttpResponse::Ok().json(json!({
        "code": 0,
        "message": "success",
        "data": metrics
    })))
}

/// 获取性能报告
pub async fn get_performance_report(monitor: web::Data<Arc<PerformanceMonitor>>) -> Result<HttpResponse> {
    let report = monitor.generate_report();
    
    Ok(HttpResponse::Ok().json(json!({
        "code": 0,
        "message": "success",
        "data": {
            "report": report
        }
    })))
}

/// 重置性能指标
pub async fn reset_metrics(monitor: web::Data<Arc<PerformanceMonitor>>) -> Result<HttpResponse> {
    monitor.reset_metrics();
    
    Ok(HttpResponse::Ok().json(json!({
        "code": 0,
        "message": "性能指标已重置",
        "data": null
    })))
}

/// 健康检查端点（包含基本性能信息）
pub async fn health_check_with_metrics(monitor: web::Data<Arc<PerformanceMonitor>>) -> Result<HttpResponse> {
    let metrics = monitor.get_metrics();
    
    // 判断系统健康状态
    let is_healthy = metrics.avg_response_time_ms < 1000.0 
        && metrics.cpu_usage_percent < 80.0
        && (metrics.total_requests == 0 || 
            (metrics.successful_requests as f64 / metrics.total_requests as f64) > 0.95);
    
    let status = if is_healthy { "healthy" } else { "unhealthy" };
    let status_code = if is_healthy { 200 } else { 503 };
    
    Ok(HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
        .json(json!({
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metrics": {
                "total_requests": metrics.total_requests,
                "success_rate": if metrics.total_requests > 0 {
                    (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
                } else { 100.0 },
                "avg_response_time_ms": metrics.avg_response_time_ms,
                "requests_per_second": metrics.requests_per_second,
                "memory_usage_mb": metrics.memory_usage_bytes as f64 / 1024.0 / 1024.0,
                "cpu_usage_percent": metrics.cpu_usage_percent
            }
        })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::middleware::metrics::PerformanceMonitor;
    use std::sync::Arc;

    #[actix_web::test]
    async fn test_get_metrics() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(monitor.clone()))
                .route("/metrics", web::get().to(get_metrics))
        ).await;

        let req = test::TestRequest::get().uri("/metrics").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_health_check_with_metrics() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(monitor.clone()))
                .route("/health", web::get().to(health_check_with_metrics))
        ).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_reset_metrics() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        // 添加一些测试数据
        let record = monitor.record_request_start("/test", "GET");
        monitor.record_request_end(record, 200);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(monitor.clone()))
                .route("/metrics/reset", web::post().to(reset_metrics))
        ).await;

        let req = test::TestRequest::post().uri("/metrics/reset").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
        
        // 验证指标已重置
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 0);
    }
}