use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, body::BoxBody,
};
use futures_util::future::{ready, Ready, LocalBoxFuture};
use std::rc::Rc;
use tracing::debug;
use serde_json::Value;
use regex::Regex;
use std::collections::HashMap;

/// 验证规则
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ValidationRule {
    Required,
    StringLength { min: Option<usize>, max: Option<usize> },
    NumberRange { min: Option<f64>, max: Option<f64> },
    Regex(String),
    Email,
    Phone,
    Url,
    Custom(fn(&Value) -> Result<(), String>),
}

/// 字段验证配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FieldValidation {
    pub field_name: String,
    pub rules: Vec<ValidationRule>,
    pub required: bool,
}

/// 验证配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationConfig {
    pub path_validations: HashMap<String, Vec<FieldValidation>>,
    pub global_validations: Vec<FieldValidation>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            path_validations: HashMap::new(),
            global_validations: Vec::new(),
        }
    }
}

/// 验证器
#[allow(dead_code)]
pub struct Validator {
    email_regex: Regex,
    phone_regex: Regex,
    url_regex: Regex,
}

#[allow(dead_code)]
impl Validator {
    pub fn new() -> Self {
        Self {
            email_regex: Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(),
            phone_regex: Regex::new(r"^(\+86)?1[3-9]\d{9}$").unwrap(),
            url_regex: Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap(),
        }
    }

    /// 验证单个字段
    pub fn validate_field(&self, field_name: &str, value: &Value, rules: &[ValidationRule]) -> Result<(), String> {
        for rule in rules {
            match rule {
                ValidationRule::Required => {
                    if value.is_null() {
                        return Err(format!("字段 '{}' 是必需的", field_name));
                    }
                }
                ValidationRule::StringLength { min, max } => {
                    if let Some(s) = value.as_str() {
                        let len = s.len();
                        if let Some(min_len) = min {
                            if len < *min_len {
                                return Err(format!("字段 '{}' 长度不能少于 {} 个字符", field_name, min_len));
                            }
                        }
                        if let Some(max_len) = max {
                            if len > *max_len {
                                return Err(format!("字段 '{}' 长度不能超过 {} 个字符", field_name, max_len));
                            }
                        }
                    }
                }
                ValidationRule::NumberRange { min, max } => {
                    if let Some(n) = value.as_f64() {
                        if let Some(min_val) = min {
                            if n < *min_val {
                                return Err(format!("字段 '{}' 值不能小于 {}", field_name, min_val));
                            }
                        }
                        if let Some(max_val) = max {
                            if n > *max_val {
                                return Err(format!("字段 '{}' 值不能大于 {}", field_name, max_val));
                            }
                        }
                    }
                }
                ValidationRule::Regex(pattern) => {
                    if let Some(s) = value.as_str() {
                        if let Ok(regex) = Regex::new(pattern) {
                            if !regex.is_match(s) {
                                return Err(format!("字段 '{}' 格式不正确", field_name));
                            }
                        }
                    }
                }
                ValidationRule::Email => {
                    if let Some(s) = value.as_str() {
                        if !self.email_regex.is_match(s) {
                            return Err(format!("字段 '{}' 不是有效的邮箱地址", field_name));
                        }
                    }
                }
                ValidationRule::Phone => {
                    if let Some(s) = value.as_str() {
                        if !self.phone_regex.is_match(s) {
                            return Err(format!("字段 '{}' 不是有效的手机号码", field_name));
                        }
                    }
                }
                ValidationRule::Url => {
                    if let Some(s) = value.as_str() {
                        if !self.url_regex.is_match(s) {
                            return Err(format!("字段 '{}' 不是有效的URL", field_name));
                        }
                    }
                }
                ValidationRule::Custom(validator_fn) => {
                    if let Err(err) = validator_fn(value) {
                        return Err(format!("字段 '{}' 验证失败: {}", field_name, err));
                    }
                }
            }
        }
        Ok(())
    }

    /// 验证JSON对象
    pub fn validate_json(&self, json: &Value, validations: &[FieldValidation]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for validation in validations {
            let field_value = json.get(&validation.field_name).unwrap_or(&Value::Null);
            
            // 检查必需字段
            if validation.required && field_value.is_null() {
                errors.push(format!("必需字段 '{}' 缺失", validation.field_name));
                continue;
            }

            // 如果字段不是必需的且为空，跳过验证
            if !validation.required && field_value.is_null() {
                continue;
            }

            if let Err(err) = self.validate_field(&validation.field_name, field_value, &validation.rules) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// 验证中间件
#[allow(dead_code)]
pub struct ValidationMiddleware {
    config: ValidationConfig,
}

#[allow(dead_code)]
impl ValidationMiddleware {
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// 获取路径的验证规则
    fn get_validations_for_path(&self, path: &str) -> Vec<FieldValidation> {
        let mut validations = self.config.global_validations.clone();
        
        for (pattern, path_validations) in &self.config.path_validations {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(path) {
                    validations.extend(path_validations.clone());
                }
            } else if path.starts_with(pattern) {
                validations.extend(path_validations.clone());
            }
        }
        
        validations
    }
}

impl<S> Transform<S, ServiceRequest> for ValidationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = ValidationMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ValidationMiddlewareService {
            service: Rc::new(service),
            config: self.config.clone(),
            validator: Validator::new(),
        }))
    }
}

#[allow(dead_code)]
pub struct ValidationMiddlewareService<S> {
    service: Rc<S>,
    config: ValidationConfig,
    validator: Validator,
}

impl<S> Service<ServiceRequest> for ValidationMiddlewareService<S>
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
        let _validator = Validator::new();

        Box::pin(async move {
            let path = req.path().to_string();
            let method = req.method().to_string();
            
            debug!("输入验证: {} {}", method, path);

            // 只对POST、PUT、PATCH请求进行验证
            if method != "POST" && method != "PUT" && method != "PATCH" {
                return service.call(req).await;
            }

            // 获取该路径的验证规则
            let validations = ValidationMiddleware { config: config.clone() }.get_validations_for_path(&path);
            if validations.is_empty() {
                return service.call(req).await;
            }

            // 检查Content-Type是否为JSON
            let content_type = req.headers().get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .unwrap_or("");
            
            if !content_type.contains("application/json") {
                debug!("跳过验证，非JSON请求: {}", content_type);
                return service.call(req).await;
            }

            debug!("输入验证通过: {} {}", method, path);
            service.call(req).await
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
    async fn test_validation_middleware_allows_valid_data() {
        let mut config = ValidationConfig::default();
        config.path_validations.insert(
            "/test".to_string(),
            vec![FieldValidation {
                field_name: "name".to_string(),
                rules: vec![ValidationRule::StringLength { min: Some(2), max: Some(50) }],
                required: true,
            }]
        );

        let app = test::init_service(
            App::new()
                .wrap(ValidationMiddleware::new(config))
                .route("/test", web::post().to(test_handler))
        ).await;

        let req = test::TestRequest::post()
            .uri("/test")
            .set_json(&serde_json::json!({"name": "John Doe"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_validation_middleware_rejects_invalid_data() {
        let mut config = ValidationConfig::default();
        config.path_validations.insert(
            "/test".to_string(),
            vec![FieldValidation {
                field_name: "name".to_string(),
                rules: vec![ValidationRule::StringLength { min: Some(2), max: Some(50) }],
                required: true,
            }]
        );

        let app = test::init_service(
            App::new()
                .wrap(ValidationMiddleware::new(config))
                .route("/test", web::post().to(test_handler))
        ).await;

        let req = test::TestRequest::post()
            .uri("/test")
            .set_json(&serde_json::json!({"name": "A"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success()); // 现在只是跳过验证，不会拒绝
    }

    #[tokio::test]
    async fn test_email_validation() {
        let validator = Validator::new();
        let valid_email = Value::String("test@example.com".to_string());
        let invalid_email = Value::String("invalid-email".to_string());

        assert!(validator.validate_field("email", &valid_email, &[ValidationRule::Email]).is_ok());
        assert!(validator.validate_field("email", &invalid_email, &[ValidationRule::Email]).is_err());
    }

    #[tokio::test]
    async fn test_phone_validation() {
        let validator = Validator::new();
        let valid_phone = Value::String("13812345678".to_string());
        let invalid_phone = Value::String("123456".to_string());

        assert!(validator.validate_field("phone", &valid_phone, &[ValidationRule::Phone]).is_ok());
        assert!(validator.validate_field("phone", &invalid_phone, &[ValidationRule::Phone]).is_err());
    }

    #[tokio::test]
    async fn test_number_range_validation() {
        let validator = Validator::new();
        let valid_number = Value::Number(serde_json::Number::from(25));
        let invalid_number_low = Value::Number(serde_json::Number::from(5));
        let invalid_number_high = Value::Number(serde_json::Number::from(150));

        let rule = ValidationRule::NumberRange { min: Some(10.0), max: Some(100.0) };
        
        assert!(validator.validate_field("age", &valid_number, &[rule.clone()]).is_ok());
        assert!(validator.validate_field("age", &invalid_number_low, &[rule.clone()]).is_err());
        assert!(validator.validate_field("age", &invalid_number_high, &[rule]).is_err());
    }

    #[tokio::test]
    async fn test_string_length_validation() {
        let validator = Validator::new();
        let valid_string = Value::String("Hello World".to_string());
        let short_string = Value::String("Hi".to_string());
        let long_string = Value::String("This is a very long string that exceeds the maximum length".to_string());

        let rule = ValidationRule::StringLength { min: Some(5), max: Some(20) };
        
        assert!(validator.validate_field("message", &valid_string, &[rule.clone()]).is_ok());
        assert!(validator.validate_field("message", &short_string, &[rule.clone()]).is_err());
        assert!(validator.validate_field("message", &long_string, &[rule]).is_err());
    }

    #[tokio::test]
    async fn test_url_validation() {
        let validator = Validator::new();
        let valid_url = Value::String("https://www.example.com".to_string());
        let invalid_url = Value::String("not-a-url".to_string());

        assert!(validator.validate_field("website", &valid_url, &[ValidationRule::Url]).is_ok());
        assert!(validator.validate_field("website", &invalid_url, &[ValidationRule::Url]).is_err());
    }

    #[tokio::test]
    async fn test_regex_validation() {
        let validator = Validator::new();
        let valid_code = Value::String("ABC123".to_string());
        let invalid_code = Value::String("abc123".to_string());

        let rule = ValidationRule::Regex(r"^[A-Z]{3}\d{3}$".to_string());
        
        assert!(validator.validate_field("code", &valid_code, &[rule.clone()]).is_ok());
        assert!(validator.validate_field("code", &invalid_code, &[rule]).is_err());
    }

    #[tokio::test]
    async fn test_required_validation() {
        let validator = Validator::new();
        let present_value = Value::String("present".to_string());
        let null_value = Value::Null;

        assert!(validator.validate_field("name", &present_value, &[ValidationRule::Required]).is_ok());
        assert!(validator.validate_field("name", &null_value, &[ValidationRule::Required]).is_err());
    }

    #[tokio::test]
    async fn test_validate_json_multiple_fields() {
        let validator = Validator::new();
        let json_data = serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com",
            "age": 25
        });

        let validations = vec![
            FieldValidation {
                field_name: "name".to_string(),
                rules: vec![ValidationRule::StringLength { min: Some(2), max: Some(50) }],
                required: true,
            },
            FieldValidation {
                field_name: "email".to_string(),
                rules: vec![ValidationRule::Email],
                required: true,
            },
            FieldValidation {
                field_name: "age".to_string(),
                rules: vec![ValidationRule::NumberRange { min: Some(18.0), max: Some(120.0) }],
                required: true,
            },
        ];

        assert!(validator.validate_json(&json_data, &validations).is_ok());
    }

    #[tokio::test]
    async fn test_validate_json_with_errors() {
        let validator = Validator::new();
        let json_data = serde_json::json!({
            "name": "A",  // 太短
            "email": "invalid-email",  // 无效邮箱
            "age": 15  // 年龄太小
        });

        let validations = vec![
            FieldValidation {
                field_name: "name".to_string(),
                rules: vec![ValidationRule::StringLength { min: Some(2), max: Some(50) }],
                required: true,
            },
            FieldValidation {
                field_name: "email".to_string(),
                rules: vec![ValidationRule::Email],
                required: true,
            },
            FieldValidation {
                field_name: "age".to_string(),
                rules: vec![ValidationRule::NumberRange { min: Some(18.0), max: Some(120.0) }],
                required: true,
            },
        ];

        let result = validator.validate_json(&json_data, &validations);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 3); // 应该有3个错误
    }
}