use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// 性能指标数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 请求总数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 最大响应时间（毫秒）
    pub max_response_time_ms: u64,
    /// 最小响应时间（毫秒）
    pub min_response_time_ms: u64,
    /// 每秒请求数（QPS）
    pub requests_per_second: f64,
    /// 按状态码分组的请求数
    pub status_code_counts: HashMap<u16, u64>,
    /// 按路径分组的请求数
    pub path_counts: HashMap<String, u64>,
    /// 内存使用情况（字节）
    pub memory_usage_bytes: u64,
    /// CPU 使用率（百分比）
    pub cpu_usage_percent: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            max_response_time_ms: 0,
            min_response_time_ms: u64::MAX,
            requests_per_second: 0.0,
            status_code_counts: HashMap::new(),
            path_counts: HashMap::new(),
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
        }
    }
}

/// 请求记录
#[derive(Debug)]
struct RequestRecord {
    start_time: Instant,
    path: String,
    method: String,
}

/// 原子计数器结构，用于高频更新的指标
#[derive(Debug)]
struct AtomicCounters {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    max_response_time_ms: AtomicU64,
    min_response_time_ms: AtomicU64,
    total_response_time_ms: AtomicU64,
}

impl AtomicCounters {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            max_response_time_ms: AtomicU64::new(0),
            min_response_time_ms: AtomicU64::new(u64::MAX),
            total_response_time_ms: AtomicU64::new(0),
        }
    }

    fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.max_response_time_ms.store(0, Ordering::Relaxed);
        self.min_response_time_ms.store(u64::MAX, Ordering::Relaxed);
        self.total_response_time_ms.store(0, Ordering::Relaxed);
    }
}

/// 系统指标缓存，减少系统调用频率
#[derive(Debug, Clone)]
struct SystemMetricsCache {
    memory_usage_bytes: u64,
    cpu_usage_percent: f64,
    last_update: Instant,
    update_interval: Duration,
}

impl SystemMetricsCache {
    fn new() -> Self {
        Self {
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            last_update: Instant::now() - Duration::from_secs(60), // 强制首次更新
            update_interval: Duration::from_secs(5), // 每5秒更新一次系统指标
        }
    }

    fn should_update(&self) -> bool {
        self.last_update.elapsed() >= self.update_interval
    }

    fn update(&mut self) {
        if let Ok(mem_info) = sys_info::mem_info() {
            self.memory_usage_bytes = (mem_info.total - mem_info.avail) * 1024;
        }

        if let Ok(load_avg) = sys_info::loadavg() {
            // 使用1分钟平均负载作为CPU使用率的近似值
            self.cpu_usage_percent = (load_avg.one * 100.0).min(100.0);
        }

        self.last_update = Instant::now();
    }
}

/// 性能监控器 - 优化版本
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    atomic_counters: Arc<AtomicCounters>,
    start_time: Instant,
    // 使用RwLock替代Mutex，允许并发读取
    status_code_counts: Arc<RwLock<HashMap<u16, u64>>>,
    path_counts: Arc<RwLock<HashMap<String, u64>>>,
    system_metrics_cache: Arc<Mutex<SystemMetricsCache>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            atomic_counters: Arc::new(AtomicCounters::new()),
            start_time: Instant::now(),
            status_code_counts: Arc::new(RwLock::new(HashMap::new())),
            path_counts: Arc::new(RwLock::new(HashMap::new())),
            system_metrics_cache: Arc::new(Mutex::new(SystemMetricsCache::new())),
        }
    }

    /// 记录请求开始
    pub fn record_request_start(&self, path: &str, method: &str) -> RequestRecord {
        RequestRecord {
            start_time: Instant::now(),
            path: path.to_string(),
            method: method.to_string(),
        }
    }

    /// 记录请求完成 - 优化版本，减少锁争用
    pub fn record_request_end(&self, record: RequestRecord, status_code: u16) {
        let response_time = record.start_time.elapsed();
        let response_time_ms = response_time.as_millis() as u64;

        // 使用原子操作更新基本计数器，无需锁
        self.atomic_counters.total_requests.fetch_add(1, Ordering::Relaxed);
        self.atomic_counters.total_response_time_ms.fetch_add(response_time_ms, Ordering::Relaxed);

        if status_code >= 200 && status_code < 400 {
            self.atomic_counters.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.atomic_counters.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        // 原子更新最大响应时间
        let mut current_max = self.atomic_counters.max_response_time_ms.load(Ordering::Relaxed);
        while response_time_ms > current_max {
            match self.atomic_counters.max_response_time_ms.compare_exchange_weak(
                current_max,
                response_time_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // 原子更新最小响应时间
        let mut current_min = self.atomic_counters.min_response_time_ms.load(Ordering::Relaxed);
        while response_time_ms < current_min {
            match self.atomic_counters.min_response_time_ms.compare_exchange_weak(
                current_min,
                response_time_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // 批量更新HashMap，减少锁持有时间
        self.update_maps_batch(&record.path, status_code);

        // 记录性能日志（仅对慢请求）
        if response_time_ms > 1000 {
            warn!(
                "慢请求检测: {} {} 耗时 {}ms",
                record.method, record.path, response_time_ms
            );
        }
    }

    /// 批量更新HashMap，减少锁持有时间
    fn update_maps_batch(&self, path: &str, status_code: u16) {
        // 快速更新状态码计数
        {
            let mut status_counts = self.status_code_counts.write().unwrap();
            *status_counts.entry(status_code).or_insert(0) += 1;
        }

        // 快速更新路径计数
        {
            let mut path_counts = self.path_counts.write().unwrap();
            *path_counts.entry(path.to_string()).or_insert(0) += 1;
        }
    }

    /// 获取性能指标 - 优化版本
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let total_requests = self.atomic_counters.total_requests.load(Ordering::Relaxed);
        let successful_requests = self.atomic_counters.successful_requests.load(Ordering::Relaxed);
        let failed_requests = self.atomic_counters.failed_requests.load(Ordering::Relaxed);
        let max_response_time_ms = self.atomic_counters.max_response_time_ms.load(Ordering::Relaxed);
        let min_response_time_ms = self.atomic_counters.min_response_time_ms.load(Ordering::Relaxed);
        let total_response_time_ms = self.atomic_counters.total_response_time_ms.load(Ordering::Relaxed);

        // 计算平均响应时间
        let avg_response_time_ms = if total_requests > 0 {
            total_response_time_ms as f64 / total_requests as f64
        } else {
            0.0
        };

        // 计算QPS
        let elapsed_seconds = self.start_time.elapsed().as_secs_f64();
        let requests_per_second = if elapsed_seconds > 0.0 {
            total_requests as f64 / elapsed_seconds
        } else {
            0.0
        };

        // 读取HashMap数据
        let status_code_counts = self.status_code_counts.read().unwrap().clone();
        let path_counts = self.path_counts.read().unwrap().clone();

        // 更新系统指标（如果需要）
        let (memory_usage_bytes, cpu_usage_percent) = {
            let mut cache = self.system_metrics_cache.lock().unwrap();
            if cache.should_update() {
                cache.update();
            }
            (cache.memory_usage_bytes, cache.cpu_usage_percent)
        };

        PerformanceMetrics {
            total_requests,
            successful_requests,
            failed_requests,
            avg_response_time_ms,
            max_response_time_ms,
            min_response_time_ms: if min_response_time_ms == u64::MAX { 0 } else { min_response_time_ms },
            requests_per_second,
            status_code_counts,
            path_counts,
            memory_usage_bytes,
            cpu_usage_percent,
        }
    }

    /// 重置性能指标
    pub fn reset_metrics(&self) {
        self.atomic_counters.reset();
        self.status_code_counts.write().unwrap().clear();
        self.path_counts.write().unwrap().clear();
    }

    /// 生成性能报告
    pub fn generate_report(&self) -> String {
        let metrics = self.get_metrics();
        
        let success_rate = if metrics.total_requests > 0 {
            (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
        } else {
            100.0
        };

        let error_rate = if metrics.total_requests > 0 {
            (metrics.failed_requests as f64 / metrics.total_requests as f64) * 100.0
        } else {
            0.0
        };

        let mut popular_paths: Vec<_> = metrics.path_counts.iter().collect();
        popular_paths.sort_by(|a, b| b.1.cmp(a.1));
        let top_paths: Vec<String> = popular_paths
            .iter()
            .take(5)
            .map(|(path, count)| format!("{}: {}", path, count))
            .collect();

        format!(
            "性能监控报告\n\
            ================\n\
            总请求数: {}\n\
            成功请求: {} ({:.2}%)\n\
            失败请求: {} ({:.2}%)\n\
            平均响应时间: {:.2}ms\n\
            最大响应时间: {}ms\n\
            最小响应时间: {}ms\n\
            每秒请求数: {:.2}\n\
            内存使用: {:.2}MB\n\
            CPU使用率: {:.2}%\n\
            热门路径:\n{}\n\
            状态码分布: {:?}",
            metrics.total_requests,
            metrics.successful_requests, success_rate,
            metrics.failed_requests, error_rate,
            metrics.avg_response_time_ms,
            metrics.max_response_time_ms,
            metrics.min_response_time_ms,
            metrics.requests_per_second,
            metrics.memory_usage_bytes as f64 / 1024.0 / 1024.0,
            metrics.cpu_usage_percent,
            top_paths.join("\n"),
            metrics.status_code_counts
        )
    }
}

/// 性能监控中间件
pub struct MetricsMiddleware {
    monitor: Arc<PerformanceMonitor>,
}

impl MetricsMiddleware {
    pub fn new(monitor: Arc<PerformanceMonitor>) -> Self {
        Self { monitor }
    }
}

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = MetricsMiddlewareService<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(MetricsMiddlewareService {
            service: Rc::new(service),
            monitor: self.monitor.clone(),
        }))
    }
}

pub struct MetricsMiddlewareService<S> {
    service: Rc<S>,
    monitor: Arc<PerformanceMonitor>,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let monitor = self.monitor.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // 记录请求开始
            let path = req.path().to_string();
            let method = req.method().to_string();
            let record = monitor.record_request_start(&path, &method);

            // 调用下一个服务
            let res = service.call(req).await?;

            // 记录请求结束
            let status_code = res.status().as_u16();
            monitor.record_request_end(record, status_code);

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({"message": "success"}))
    }

    #[actix_web::test]
    async fn test_metrics_middleware() {
        let monitor = Arc::new(PerformanceMonitor::new());
        let monitor_clone = monitor.clone();

        let app = test::init_service(
            App::new()
                .wrap(MetricsMiddleware::new(monitor))
                .route("/test", web::get().to(test_handler))
        ).await;

        // 发送几个测试请求
        for _ in 0..5 {
            let req = test::TestRequest::get().uri("/test").to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }

        // 检查指标
        let metrics = monitor_clone.get_metrics();
        assert_eq!(metrics.total_requests, 5);
        assert_eq!(metrics.successful_requests, 5);
        assert_eq!(metrics.failed_requests, 0);
        assert!(metrics.avg_response_time_ms >= 0.0);
    }

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();
        
        // 模拟请求
        let record = monitor.record_request_start("/test", "GET");
        std::thread::sleep(std::time::Duration::from_millis(10));
        monitor.record_request_end(record, 200);
        
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
        assert!(metrics.avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_metrics_reset() {
        let monitor = PerformanceMonitor::new();
        
        // 添加一些数据
        let record = monitor.record_request_start("/test", "GET");
        monitor.record_request_end(record, 200);
        
        // 重置指标
        monitor.reset_metrics();
        
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[test]
    fn test_report_generation() {
        let monitor = PerformanceMonitor::new();
        
        // 添加一些测试数据
        let record1 = monitor.record_request_start("/api/test", "GET");
        monitor.record_request_end(record1, 200);
        
        let record2 = monitor.record_request_start("/api/error", "POST");
        monitor.record_request_end(record2, 500);
        
        let report = monitor.generate_report();
        assert!(report.contains("总请求数: 2"));
        assert!(report.contains("成功请求: 1"));
        assert!(report.contains("失败请求: 1"));
    }

    #[test]
    fn test_atomic_counters_performance() {
        let monitor = Arc::new(PerformanceMonitor::new());
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let monitor = monitor.clone();
                std::thread::spawn(move || {
                    for j in 0..100 {
                        let record = monitor.record_request_start(&format!("/test/{}", i), "GET");
                        monitor.record_request_end(record, if j % 10 == 0 { 500 } else { 200 });
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 1000);
        assert_eq!(metrics.successful_requests, 900);
        assert_eq!(metrics.failed_requests, 100);
    }

    #[test]
    fn test_system_metrics_cache() {
        let mut cache = SystemMetricsCache::new();
        
        // 首次应该需要更新
        assert!(cache.should_update());
        
        cache.update();
        
        // 刚更新过，不应该需要更新
        assert!(!cache.should_update());
        
        // 模拟时间过去
        cache.last_update = Instant::now() - Duration::from_secs(10);
        assert!(cache.should_update());
    }
}