use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubFieldError {
    pub resource: String,
    pub field: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubErrorBody {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<GitHubFieldError>>,
}

#[derive(Debug, Clone)]
pub enum HttpError {
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Validation(Vec<GitHubFieldError>),
    Conflict(String),
    TooManyRequests(String),
    Internal(String),
}

impl HttpError {
    pub fn status_code(&self) -> u16 {
        match self {
            HttpError::Unauthorized(_) => 401,
            HttpError::Forbidden(_) => 403,
            HttpError::NotFound(_) => 404,
            HttpError::Validation(_) => 422,
            HttpError::Conflict(_) => 409,
            HttpError::TooManyRequests(_) => 429,
            HttpError::Internal(_) => 500,
        }
    }

    pub fn to_body(&self, documentation_url: Option<String>) -> GitHubErrorBody {
        match self {
            HttpError::Unauthorized(msg)
            | HttpError::Forbidden(msg)
            | HttpError::NotFound(msg)
            | HttpError::Conflict(msg)
            | HttpError::TooManyRequests(msg)
            | HttpError::Internal(msg) => GitHubErrorBody {
                message: msg.clone(),
                documentation_url,
                errors: None,
            },
            HttpError::Validation(errors) => GitHubErrorBody {
                message: "Validation Failed".to_string(),
                documentation_url,
                errors: Some(errors.clone()),
            },
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            HttpError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            HttpError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            HttpError::Validation(_) => write!(f, "Validation Failed"),
            HttpError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            HttpError::TooManyRequests(msg) => write!(f, "Too Many Requests: {}", msg),
            HttpError::Internal(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_body_serialization() {
        let errs = vec![GitHubFieldError {
            resource: "User".to_string(),
            field: "email".to_string(),
            code: "invalid".to_string(),
            message: Some("must be email".to_string()),
        }];
        let e = HttpError::Validation(errs.clone());
        let body = e.to_body(Some("https://docs.example".to_string()));
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("Validation Failed"));
        assert!(json.contains("documentation_url"));
        assert!(json.contains("email"));
        assert_eq!(e.status_code(), 422);
        assert_eq!(body.errors.unwrap().len(), 1);
    }
}
