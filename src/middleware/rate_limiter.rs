use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
    body::{BoxBody, MessageBody},
};
use futures_util::future::{ready, Ready, LocalBoxFuture};
use std::rc::Rc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tracing::{warn, debug};

/// 限流算法类型
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RateLimitAlgorithm {
    /// 固定窗口算法
    FixedWindow,
    /// 滑动窗口算法
    SlidingWindow,
    /// 令牌桶算法
    TokenBucket,
    /// 漏桶算法
    LeakyBucket,
}

/// 限流配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RateLimitConfig {
    /// 每个时间窗口允许的最大请求数
    pub max_requests: u32,
    /// 时间窗口大小（秒）
    pub window_size: u64,
    /// 限流算法
    pub algorithm: RateLimitAlgorithm,
    /// 是否启用限流
    pub enabled: bool,
    /// 白名单IP列表
    pub whitelist: Vec<String>,
    /// 黑名单IP列表
    pub blacklist: Vec<String>,
    /// 自定义错误消息
    pub error_message: String,
    /// 是否在响应头中包含限流信息
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_size: 60,
            algorithm: RateLimitAlgorithm::FixedWindow,
            enabled: true,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            error_message: "请求过于频繁，请稍后再试".to_string(),
            include_headers: true,
        }
    }
}

/// 请求记录
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RequestRecord {
    count: u32,
    window_start: Instant,
    last_request: Instant,
    tokens: f64,
}

#[allow(dead_code)]
impl RequestRecord {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            count: 0,
            window_start: now,
            last_request: now,
            tokens: 0.0, // 初始令牌数，将在令牌桶算法中动态设置
        }
    }

    fn new_with_tokens(max_tokens: f64) -> Self {
        let now = Instant::now();
        Self {
            count: 0,
            window_start: now,
            last_request: now,
            tokens: max_tokens, // 令牌桶初始时应该是满的
        }
    }
}

/// 限流器存储
type RateLimitStore = Arc<Mutex<HashMap<String, RequestRecord>>>;

/// 限流器
#[derive(Clone)]
#[allow(dead_code)]
pub struct RateLimiter {
    store: RateLimitStore,
    config: RateLimitConfig,
}

#[allow(dead_code)]
impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// 检查IP是否在白名单中
    fn is_whitelisted(&self, ip: &str) -> bool {
        self.config.whitelist.iter().any(|pattern| {
            if pattern.contains('*') {
                // 简单的通配符匹配
                let pattern = pattern.replace('*', ".*");
                if let Ok(regex) = regex::Regex::new(&pattern) {
                    regex.is_match(ip)
                } else {
                    false
                }
            } else {
                pattern == ip
            }
        })
    }

    /// 检查IP是否在黑名单中
    fn is_blacklisted(&self, ip: &str) -> bool {
        self.config.blacklist.iter().any(|pattern| {
            if pattern.contains('*') {
                // 简单的通配符匹配
                let pattern = pattern.replace('*', ".*");
                if let Ok(regex) = regex::Regex::new(&pattern) {
                    regex.is_match(ip)
                } else {
                    false
                }
            } else {
                pattern == ip
            }
        })
    }

    /// 检查是否允许请求
    pub fn check_rate_limit(&self, client_ip: &str) -> Result<RateLimitInfo, String> {
        if !self.config.enabled {
            return Ok(RateLimitInfo {
                allowed: true,
                remaining: self.config.max_requests,
                reset_time: Instant::now() + Duration::from_secs(self.config.window_size),
                retry_after: None,
            });
        }

        // 检查黑名单
        if self.is_blacklisted(client_ip) {
            return Err("IP地址被禁止访问".to_string());
        }

        // 检查白名单
        if self.is_whitelisted(client_ip) {
            return Ok(RateLimitInfo {
                allowed: true,
                remaining: self.config.max_requests,
                reset_time: Instant::now() + Duration::from_secs(self.config.window_size),
                retry_after: None,
            });
        }

        let mut store = self.store.lock().unwrap();
        let now = Instant::now();
        
        let record = store.entry(client_ip.to_string()).or_insert_with(|| {
            match self.config.algorithm {
                RateLimitAlgorithm::TokenBucket => {
                    // 令牌桶初始时应该是满的
                    RequestRecord::new_with_tokens(self.config.max_requests as f64)
                },
                _ => RequestRecord::new()
            }
        });

        match self.config.algorithm {
            RateLimitAlgorithm::FixedWindow => self.check_fixed_window(record, now),
            RateLimitAlgorithm::SlidingWindow => self.check_sliding_window(record, now),
            RateLimitAlgorithm::TokenBucket => self.check_token_bucket(record, now),
            RateLimitAlgorithm::LeakyBucket => self.check_leaky_bucket(record, now),
        }
    }

    /// 固定窗口算法
    fn check_fixed_window(&self, record: &mut RequestRecord, now: Instant) -> Result<RateLimitInfo, String> {
        let window_duration = Duration::from_secs(self.config.window_size);
        
        // 检查是否需要重置窗口
        if now.duration_since(record.window_start) >= window_duration {
            record.count = 0;
            record.window_start = now;
        }

        if record.count >= self.config.max_requests {
            let reset_time = record.window_start + window_duration;
            let retry_after = reset_time.duration_since(now);
            
            return Ok(RateLimitInfo {
                allowed: false,
                remaining: 0,
                reset_time,
                retry_after: Some(retry_after),
            });
        }

        record.count += 1;
        record.last_request = now;

        Ok(RateLimitInfo {
            allowed: true,
            remaining: self.config.max_requests - record.count,
            reset_time: record.window_start + window_duration,
            retry_after: None,
        })
    }

    /// 滑动窗口算法（简化版本）
    fn check_sliding_window(&self, record: &mut RequestRecord, now: Instant) -> Result<RateLimitInfo, String> {
        // 简化实现，实际应该维护请求时间戳队列
        self.check_fixed_window(record, now)
    }

    /// 令牌桶算法
    fn check_token_bucket(&self, record: &mut RequestRecord, now: Instant) -> Result<RateLimitInfo, String> {
        let time_passed = now.duration_since(record.last_request).as_secs_f64();
        let tokens_to_add = time_passed * (self.config.max_requests as f64 / self.config.window_size as f64);
        
        record.tokens = (record.tokens + tokens_to_add).min(self.config.max_requests as f64);
        record.last_request = now;

        if record.tokens >= 1.0 {
            record.tokens -= 1.0;
            Ok(RateLimitInfo {
                allowed: true,
                remaining: record.tokens as u32,
                reset_time: now + Duration::from_secs(self.config.window_size),
                retry_after: None,
            })
        } else {
            let retry_after = Duration::from_secs_f64(1.0 - record.tokens);
            Ok(RateLimitInfo {
                allowed: false,
                remaining: 0,
                reset_time: now + retry_after,
                retry_after: Some(retry_after),
            })
        }
    }

    /// 漏桶算法
    fn check_leaky_bucket(&self, record: &mut RequestRecord, now: Instant) -> Result<RateLimitInfo, String> {
        // 简化实现，实际应该维护请求队列
        self.check_token_bucket(record, now)
    }

    /// 清理过期记录
    pub fn cleanup_expired(&self) {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.config.window_size * 2); // 保留2个窗口的数据
        
        store.retain(|_, record| {
            now.duration_since(record.last_request) < window_duration
        });
    }
}

/// 限流信息
#[derive(Debug)]
#[allow(dead_code)]
pub struct RateLimitInfo {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_time: Instant,
    pub retry_after: Option<Duration>,
}

/// 限流中间件
#[allow(dead_code)]
pub struct RateLimitMiddleware {
    limiter: RateLimiter,
}

#[allow(dead_code)]
impl RateLimitMiddleware {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: RateLimiter::new(config),
        }
    }

    /// 获取客户端IP地址
    fn get_client_ip(&self, req: &ServiceRequest) -> String {
        // 尝试从各种头部获取真实IP
        if let Some(forwarded_for) = req.headers().get("X-Forwarded-For") {
            if let Ok(forwarded_for_str) = forwarded_for.to_str() {
                if let Some(first_ip) = forwarded_for_str.split(',').next() {
                    return first_ip.trim().to_string();
                }
            }
        }

        if let Some(real_ip) = req.headers().get("X-Real-IP") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                return real_ip_str.to_string();
            }
        }

        if let Some(forwarded) = req.headers().get("Forwarded") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // 解析 Forwarded 头部格式: for=192.0.2.60;proto=http;by=203.0.113.43
                for part in forwarded_str.split(';') {
                    if part.trim().starts_with("for=") {
                        let ip = part.trim().strip_prefix("for=").unwrap_or("");
                        return ip.trim_matches('"').to_string();
                    }
                }
            }
        }

        // 回退到连接信息
        req.connection_info().peer_addr().unwrap_or("unknown").to_string()
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
            limiter: self.limiter.clone(),
        }))
    }
}

#[allow(dead_code)]
pub struct RateLimitMiddlewareService<S> {
    service: Rc<S>,
    limiter: RateLimiter,
}

impl<S> Clone for RateLimitMiddlewareService<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            limiter: RateLimiter {
                store: self.limiter.store.clone(),
                config: self.limiter.config.clone(),
            },
        }
    }
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let limiter = self.limiter.clone();

        Box::pin(async move {
            let client_ip = RateLimitMiddleware { limiter: limiter.clone() }.get_client_ip(&req);
            
            debug!("限流检查: IP {} 访问 {}", client_ip, req.path());

            match limiter.check_rate_limit(&client_ip) {
                Ok(info) => {
                    if info.allowed {
                        debug!("限流通过: IP {} 剩余 {} 次请求", client_ip, info.remaining);
                        
                        let response = service.call(req).await?;
                        
                        // 简化处理，不添加header以避免类型问题
                        Ok(response.map_into_boxed_body())
                    } else {
                        warn!("限流拒绝: IP {} 超过限制", client_ip);
                        
                        let error_response = HttpResponse::TooManyRequests()
                            .json(serde_json::json!({
                                "success": false,
                                "error": {
                                    "code": 429,
                                    "message": limiter.config.error_message,
                                    "details": format!("请求过于频繁，请稍后重试")
                                }
                            }));
                        
                        Ok(req.into_response(error_response))
                    }
                }
                Err(error) => {
                    warn!("限流错误: IP {} - {}", client_ip, error);
                    let error_response = HttpResponse::Forbidden()
                        .json(serde_json::json!({
                            "success": false,
                            "error": {
                                "code": 403,
                                "message": "访问被拒绝",
                                "details": error
                            }
                        }));
                    Ok(req.into_response(error_response))
                }
            }
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
    async fn test_rate_limit_allows_requests_within_limit() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_size: 60,
            ..Default::default()
        };

        let app = test::init_service(
            App::new()
                .wrap(RateLimitMiddleware::new(config))
                .route("/test", web::get().to(test_handler))
        ).await;

        // 发送5个请求，都应该成功
        for _ in 0..5 {
            let req = test::TestRequest::get().uri("/test").to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }
    }

    #[actix_web::test]
    async fn test_rate_limit_blocks_excess_requests() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_size: 60,
            ..Default::default()
        };

        let app = test::init_service(
            App::new()
                .wrap(RateLimitMiddleware::new(config))
                .route("/test", web::get().to(test_handler))
        ).await;

        // 发送2个请求，应该成功
        for _ in 0..2 {
            let req = test::TestRequest::get().uri("/test").to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }

        // 第3个请求应该被限流
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 429);
    }

    #[tokio::test]
    async fn test_whitelist_bypass() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_size: 60,
            whitelist: vec!["127.0.0.1".to_string()],
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        
        // 白名单IP应该总是被允许
        for _ in 0..10 {
            let result = limiter.check_rate_limit("127.0.0.1");
            assert!(result.is_ok());
            assert!(result.unwrap().allowed);
        }
    }

    #[tokio::test]
    async fn test_blacklist_block() {
        let config = RateLimitConfig {
            max_requests: 100,
            window_size: 60,
            blacklist: vec!["192.168.1.100".to_string()],
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        
        // 黑名单IP应该被拒绝
        let result = limiter.check_rate_limit("192.168.1.100");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_bucket_algorithm() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_size: 10, // 每秒1个令牌
            algorithm: RateLimitAlgorithm::TokenBucket,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        
        // 初始应该有令牌
        let result = limiter.check_rate_limit("192.168.1.1");
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }
}