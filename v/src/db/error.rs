use thiserror::Error;

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("配置错误: {0}")]
    Config(String),
    #[error("连接池错误: {0}")]
    Pool(String),
    #[error("SQLx 错误: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("未找到记录")]
    NotFound,
    #[error("事务错误: {0}")]
    Tx(String),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
}

// 保留统一错误描述函数，避免在各层重复构建错误字符串

/// 获取详细错误描述（中英文） / Get detailed error description (CN/EN)
pub fn describe_error(e: &DbError) -> String {
    match e {
        DbError::Config(msg) => format!("配置错误 / Config error: {}", msg),
        DbError::Pool(msg) => format!("连接池错误 / Pool error: {}", msg),
        DbError::Sqlx(err) => format!("SQLx 错误 / SQLx error: {}", err),
        DbError::NotFound => "未找到记录 / Record not found".to_string(),
        DbError::Tx(msg) => format!("事务错误 / Transaction error: {}", msg),
        DbError::Serde(msg) => format!("序列化错误 / Serialization error: {}", msg),
    }
}
