use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, body::BoxBody,
};
use futures_util::future::{ready, Ready, LocalBoxFuture};
use std::rc::Rc;
use tracing::{warn, debug};
use serde_json::Value;
use regex::Regex;
use std::collections::HashSet;
use urlencoding::decode;

/// 安全配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SecurityConfig {
    /// 启用XSS保护
    pub enable_xss_protection: bool,
    /// 启用SQL注入检测
    pub enable_sql_injection_detection: bool,
    /// 启用CSRF保护
    pub enable_csrf_protection: bool,
    /// 允许的Content-Type
    pub allowed_content_types: HashSet<String>,
    /// 最大请求大小
    pub max_request_size: usize,
    /// 启用安全头
    pub enable_security_headers: bool,
    /// CSRF头名称
    pub csrf_header_name: String,
    /// 受保护的路径
    pub protected_paths: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_types = HashSet::new();
        allowed_types.insert("application/json".to_string());
        allowed_types.insert("application/x-www-form-urlencoded".to_string());
        allowed_types.insert("multipart/form-data".to_string());
        allowed_types.insert("text/plain".to_string());

        Self {
            enable_xss_protection: true,
            enable_sql_injection_detection: true,
            enable_csrf_protection: true,
            allowed_content_types: allowed_types,
            max_request_size: 1024 * 1024, // 1MB
            enable_security_headers: true,
            csrf_header_name: "X-CSRF-Token".to_string(),
            protected_paths: vec!["/api/".to_string()],
        }
    }
}

/// 安全检测器
#[allow(dead_code)]
pub struct SecurityDetector {
    xss_patterns: Vec<Regex>,
    sql_injection_patterns: Vec<Regex>,
}

#[allow(dead_code)]
impl SecurityDetector {
    pub fn new() -> Self {
        let xss_patterns = vec![
            Regex::new(r"<script[^>]*>.*?</script>").unwrap(),
            Regex::new(r"javascript:").unwrap(),
            Regex::new(r"on\w+\s*=").unwrap(),
            Regex::new(r"<iframe[^>]*>").unwrap(),
            Regex::new(r"<object[^>]*>").unwrap(),
            Regex::new(r"<embed[^>]*>").unwrap(),
        ];

        let sql_injection_patterns = vec![
            Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute)").unwrap(),
            Regex::new(r"(?i)(or|and)\s+\d+\s*=\s*\d+").unwrap(),
            Regex::new(r"(?i)'\s*(or|and)\s*'").unwrap(),
            Regex::new(r"(?i)--").unwrap(),
            Regex::new(r"/\*.*?\*/").unwrap(),
        ];

        Self {
            xss_patterns,
            sql_injection_patterns,
        }
    }

    /// 检测XSS攻击
    pub fn detect_xss(&self, input: &str) -> bool {
        self.xss_patterns.iter().any(|pattern| pattern.is_match(input))
    }

    /// 检测SQL注入攻击
    pub fn detect_sql_injection(&self, input: &str) -> bool {
        self.sql_injection_patterns.iter().any(|pattern| pattern.is_match(input))
    }

    /// 扫描JSON值
    pub fn scan_json_value(&self, value: &Value, config: &SecurityConfig) -> Option<String> {
        match value {
            Value::String(s) => {
                if config.enable_xss_protection && self.detect_xss(s) {
                    return Some("检测到XSS攻击".to_string());
                }
                if config.enable_sql_injection_detection && self.detect_sql_injection(s) {
                    return Some("检测到SQL注入攻击".to_string());
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Some(threat) = self.scan_json_value(item, config) {
                        return Some(threat);
                    }
                }
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    if let Some(threat) = self.scan_json_value(v, config) {
                        return Some(threat);
                    }
                }
            }
            _ => {}
        }
        None
    }
}

/// 安全中间件
#[allow(dead_code)]
pub struct SecurityMiddleware {
    config: SecurityConfig,
}

#[allow(dead_code)]
impl SecurityMiddleware {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// 检查路径是否需要CSRF保护
    fn requires_csrf_protection(config: &SecurityConfig, path: &str) -> bool {
        config.protected_paths.iter().any(|pattern| {
            if let Ok(regex) = Regex::new(pattern) {
                regex.is_match(path)
            } else {
                path.starts_with(pattern)
            }
        })
    }

    /// 添加安全头
    fn add_security_headers(response: &mut ServiceResponse) {
        let headers = response.headers_mut();
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-content-type-options"),
            actix_web::http::header::HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-frame-options"),
            actix_web::http::header::HeaderValue::from_static("DENY"),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-xss-protection"),
            actix_web::http::header::HeaderValue::from_static("1; mode=block"),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("strict-transport-security"),
            actix_web::http::header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("referrer-policy"),
            actix_web::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
    }
}

impl<S> Transform<S, ServiceRequest> for SecurityMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = SecurityMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityMiddlewareService {
            service: Rc::new(service),
            config: self.config.clone(),
            detector: SecurityDetector::new(),
        }))
    }
}

#[allow(dead_code)]
pub struct SecurityMiddlewareService<S> {
    service: Rc<S>,
    config: SecurityConfig,
    detector: SecurityDetector,
}

impl<S> Service<ServiceRequest> for SecurityMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let config = self.config.clone();
        let detector = SecurityDetector::new();

        Box::pin(async move {
            let path = req.path();
            let method = req.method().clone();
            
            debug!("安全检查: {} {}", method, path);

            // 1. 检查Content-Type
            if let Some(content_type) = req.headers().get("content-type") {
                if let Ok(ct_str) = content_type.to_str() {
                    let ct_main = ct_str.split(';').next().unwrap_or("").trim();
                    if !config.allowed_content_types.contains(ct_main) && !ct_main.is_empty() {
                        warn!("不允许的Content-Type: {}", ct_str);
                        let error_response = HttpResponse::BadRequest()
                            .json(serde_json::json!({
                                "success": false,
                                "error": {
                                    "code": 400,
                                    "message": "不支持的内容类型",
                                    "details": format!("Content-Type '{}' 不被允许", ct_main)
                                }
                            }));
                        return Ok(req.into_response(error_response).map_into_boxed_body());
                    }
                }
            }

            // 2. CSRF保护检查
            if config.enable_csrf_protection 
                && (method == "POST" || method == "PUT" || method == "DELETE" || method == "PATCH")
                && SecurityMiddleware::requires_csrf_protection(&config, path) {
                
                if req.headers().get(&config.csrf_header_name).is_none() {
                    warn!("缺少CSRF令牌: {} {}", method, path);
                    let error_response = HttpResponse::Forbidden()
                        .json(serde_json::json!({
                            "success": false,
                            "error": {
                                "code": 403,
                                "message": "CSRF保护：缺少安全令牌",
                                "details": format!("请在请求头中包含 {}", config.csrf_header_name)
                            }
                        }));
                    return Ok(req.into_response(error_response).map_into_boxed_body());
                }
            }

            // 3. 检查查询参数
            let query_string = req.query_string();
            if !query_string.is_empty() {
                // 先尝试解码查询参数，然后检查原始和解码后的内容
                let decoded_query = decode(query_string).unwrap_or_else(|_| query_string.into());
                
                if config.enable_xss_protection && 
                   (detector.detect_xss(query_string) || detector.detect_xss(&decoded_query)) {
                    warn!("查询参数中检测到XSS攻击: {}", query_string);
                    let error_response = HttpResponse::BadRequest()
                        .json(serde_json::json!({
                            "success": false,
                            "error": {
                                "code": 400,
                                "message": "检测到恶意输入",
                                "details": "查询参数包含潜在的XSS攻击代码"
                            }
                        }));
                    return Ok(req.into_response(error_response).map_into_boxed_body());
                }

                if config.enable_sql_injection_detection && 
                   (detector.detect_sql_injection(query_string) || detector.detect_sql_injection(&decoded_query)) {
                    warn!("查询参数中检测到SQL注入攻击: {}", query_string);
                    let error_response = HttpResponse::BadRequest()
                        .json(serde_json::json!({
                            "success": false,
                            "error": {
                                "code": 400,
                                "message": "检测到恶意输入",
                                "details": "查询参数包含潜在的SQL注入代码"
                            }
                        }));
                    return Ok(req.into_response(error_response).map_into_boxed_body());
                }
            }

            // 4. 处理请求并添加安全头
            let mut response = service.call(req).await?;
            
            if config.enable_security_headers {
                SecurityMiddleware::add_security_headers(&mut response);
            }

            Ok(response)
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
    async fn test_security_middleware_allows_safe_requests() {
        let app = test::init_service(
            App::new()
                .wrap(SecurityMiddleware::new(SecurityConfig::default()))
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_security_middleware_blocks_xss() {
        let app = test::init_service(
            App::new()
                .wrap(SecurityMiddleware::new(SecurityConfig::default()))
                .route("/test", web::get().to(test_handler))
        ).await;

        // URL 编码 XSS 攻击字符串
        let req = test::TestRequest::get()
            .uri("/test?input=%3Cscript%3Ealert%28%27xss%27%29%3C%2Fscript%3E")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[test]
    async fn test_xss_detection() {
        let detector = SecurityDetector::new();
        assert!(detector.detect_xss("<script>alert('xss')</script>"));
        assert!(detector.detect_xss("javascript:alert('xss')"));
        assert!(!detector.detect_xss("normal text"));
    }

    #[test]
    async fn test_sql_injection_detection() {
        let detector = SecurityDetector::new();
        assert!(detector.detect_sql_injection("' OR 1=1 --"));
        assert!(detector.detect_sql_injection("UNION SELECT * FROM users"));
        assert!(!detector.detect_sql_injection("normal query"));
    }
}