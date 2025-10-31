use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use thiserror::Error;

/// 统一的应用错误类型
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("配置错误: {0}")]
    Config(#[from] crate::comm::config::ConfigError),
    
    #[error("认证错误: {message}")]
    Auth { message: String },
    
    #[error("权限错误: {message}")]
    Permission { message: String },
    
    #[error("验证错误: {field}: {message}")]
    Validation { field: String, message: String },
    
    #[error("网络错误: {0}")]
    Network(#[from] actix_web::Error),
    
    #[error("数据库错误: {message}")]
    Database { message: String },
    
    #[error("外部服务错误: {service}: {message}")]
    ExternalService { service: String, message: String },
    
    #[error("超时错误: {operation}")]
    Timeout { operation: String },
    
    #[error("资源未找到: {resource}")]
    NotFound { resource: String },
    
    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

#[allow(dead_code)]
impl AppError {
    /// 创建认证错误
    pub fn auth<T: Into<String>>(message: T) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }
    
    /// 创建权限错误
    pub fn permission<T: Into<String>>(message: T) -> Self {
        Self::Permission {
            message: message.into(),
        }
    }
    
    /// 创建验证错误
    pub fn validation<T: Into<String>, U: Into<String>>(field: T, message: U) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }
    
    /// 创建数据库错误
    pub fn database<T: Into<String>>(message: T) -> Self {
        Self::Database {
            message: message.into(),
        }
    }
    
    /// 创建外部服务错误
    pub fn external_service<T: Into<String>, U: Into<String>>(service: T, message: U) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }
    
    /// 创建超时错误
    pub fn timeout<T: Into<String>>(operation: T) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }
    
    /// 创建资源未找到错误
    pub fn not_found<T: Into<String>>(resource: T) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }
    
    /// 获取错误代码
    pub fn error_code(&self) -> i32 {
        match self {
            AppError::Config(_) => 1001,
            AppError::Auth { .. } => 1002,
            AppError::Permission { .. } => 1003,
            AppError::Validation { .. } => 1004,
            AppError::Network(_) => 1005,
            AppError::Database { .. } => 1006,
            AppError::ExternalService { .. } => 1007,
            AppError::Timeout { .. } => 1008,
            AppError::NotFound { .. } => 1009,
            AppError::Internal(_) => 1000,
        }
    }
    
    /// 获取HTTP状态码
    pub fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        
        match self {
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Auth { .. } => StatusCode::UNAUTHORIZED,
            AppError::Permission { .. } => StatusCode::FORBIDDEN,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            AppError::Network(_) => StatusCode::BAD_GATEWAY,
            AppError::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalService { .. } => StatusCode::BAD_GATEWAY,
            AppError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();
        
        // 记录错误日志
        match self {
            AppError::Internal(_) | AppError::Database { .. } => {
                tracing::error!("Internal error: {}", message);
            }
            AppError::ExternalService { .. } => {
                tracing::warn!("External service error: {}", message);
            }
            _ => {
                tracing::info!("Client error: {}", message);
            }
        }
        
        HttpResponse::build(status).json(json!({
            "success": false,
            "error": {
                "code": error_code,
                "message": message,
                "type": format!("{:?}", self).split('(').next().unwrap_or("Unknown")
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// 应用结果类型
pub type AppResult<T> = Result<T, AppError>;

/// 成功响应结构
#[derive(serde::Serialize)]
#[allow(dead_code)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub timestamp: String,
}

#[allow(dead_code)]
impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// 便捷宏：创建API成功响应
#[macro_export]
macro_rules! api_success {
    ($data:expr) => {
        Ok(actix_web::web::Json($crate::error::ApiResponse::success($data)))
    };
}

/// 便捷宏：创建API错误响应
#[macro_export]
macro_rules! api_error {
    ($error:expr) => {
        Err($crate::error::AppError::from($error))
    };
}