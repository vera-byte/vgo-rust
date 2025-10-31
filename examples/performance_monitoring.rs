use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger};
use std::sync::Arc;
use vgo_rust::middleware::metrics::{MetricsMiddleware, PerformanceMonitor};
use vgo_rust::api::metrics::{get_metrics, get_performance_report, reset_metrics, health_check_with_metrics};

/// 示例 API 端点
async fn hello() -> Result<HttpResponse> {
    // 模拟一些处理时间
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello, World!",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// 慢速 API 端点（用于测试性能监控）
async fn slow_endpoint() -> Result<HttpResponse> {
    // 模拟慢速处理
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "This is a slow endpoint",
        "processing_time": "2000ms"
    })))
}

/// 错误端点（用于测试错误监控）
async fn error_endpoint() -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
        "error": "Simulated error for testing",
        "code": 500
    })))
}

/// CPU 密集型端点（用于测试 CPU 监控）
async fn cpu_intensive() -> Result<HttpResponse> {
    // 模拟 CPU 密集型任务
    let start = std::time::Instant::now();
    let mut sum = 0u64;
    for i in 0..1_000_000 {
        sum += i;
    }
    let duration = start.elapsed();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "CPU intensive task completed",
        "result": sum,
        "duration_ms": duration.as_millis()
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init();
    
    // 创建性能监控器
    let performance_monitor = Arc::new(PerformanceMonitor::new());
    
    println!("🚀 启动性能监控示例服务器");
    println!("📊 性能指标端点: http://localhost:8080/api/metrics");
    println!("📈 性能报告端点: http://localhost:8080/api/metrics/report");
    println!("🔄 重置指标端点: http://localhost:8080/api/metrics/reset");
    println!("💚 健康检查端点: http://localhost:8080/health");
    println!("🧪 测试端点:");
    println!("   - 普通端点: http://localhost:8080/hello");
    println!("   - 慢速端点: http://localhost:8080/slow");
    println!("   - 错误端点: http://localhost:8080/error");
    println!("   - CPU密集型: http://localhost:8080/cpu");
    
    HttpServer::new(move || {
        App::new()
            // 添加性能监控中间件
            .wrap(MetricsMiddleware::new(performance_monitor.clone()))
            // 添加日志中间件
            .wrap(Logger::default())
            // 共享性能监控器数据
            .app_data(web::Data::new(performance_monitor.clone()))
            // API 路由
            .service(
                web::scope("/api")
                    .route("/metrics", web::get().to(get_metrics))
                    .route("/metrics/report", web::get().to(get_performance_report))
                    .route("/metrics/reset", web::post().to(reset_metrics))
            )
            // 健康检查
            .route("/health", web::get().to(health_check_with_metrics))
            // 测试端点
            .route("/hello", web::get().to(hello))
            .route("/slow", web::get().to(slow_endpoint))
            .route("/error", web::get().to(error_endpoint))
            .route("/cpu", web::get().to(cpu_intensive))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_performance_monitoring_integration() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        let app = test::init_service(
            App::new()
                .wrap(MetricsMiddleware::new(monitor.clone()))
                .app_data(web::Data::new(monitor.clone()))
                .route("/hello", web::get().to(hello))
                .route("/api/metrics", web::get().to(get_metrics))
        ).await;

        // 发送几个请求
        for _ in 0..5 {
            let req = test::TestRequest::get().uri("/hello").to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }

        // 检查指标
        let req = test::TestRequest::get().uri("/api/metrics").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        // 验证指标已记录
        let metrics = monitor.get_metrics();
        assert!(metrics.total_requests >= 5);
    }

    #[actix_web::test]
    async fn test_error_monitoring() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        let app = test::init_service(
            App::new()
                .wrap(MetricsMiddleware::new(monitor.clone()))
                .route("/error", web::get().to(error_endpoint))
        ).await;

        // 发送错误请求
        let req = test::TestRequest::get().uri("/error").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
        
        // 验证错误被记录
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.failed_requests, 1);
    }
}