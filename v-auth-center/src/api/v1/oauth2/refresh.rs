use crate::config::sa_token_conf::init_sa_token_oath2;
use actix_web::{http::header, web, HttpRequest, Responder};
use serde::Deserialize;
use tracing::{debug, warn};
use validator::Validate;

/// 刷新令牌端点注册（GET）
/// Register refresh token endpoint (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(oauth2_refresh_token_handle)));
}

/// 刷新令牌请求参数
/// Refresh token request parameters
#[derive(Deserialize, Validate)]
pub struct OAuth2RefreshTokenHandleReq {
    /// 客户端标识
    /// OAuth2 client identifier
    #[validate(length(min = 3, max = 50, message = "client_id长度必须在3-50个字符之间"))]
    client_id: String,

    /// 客户端密钥
    /// OAuth2 client secret
    #[validate(length(min = 3, max = 50, message = "client_secret长度必须在3-50个字符之间"))]
    client_secret: String,
}
/// 使用刷新令牌获取新的访问令牌
/// Refresh an access token using a refresh token
pub async fn oauth2_refresh_token_handle(
    q: web::Query<OAuth2RefreshTokenHandleReq>,
    req: HttpRequest,
) -> impl Responder {
    // 参数校验 / Validate request params
    if let Err(e) = q.validate() {
        return v::response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            format!("{}", e),
        );
    }
    // 解析 Authorization: Bearer <refresh_token>
    // Parse Authorization: Bearer <refresh_token>
    let authorize_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if authorize_header.is_empty() {
        return v::response::respond_any(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "缺少授权头 / Missing Authorization header",
        );
    }
    let refresh_token = if let Some(token) = authorize_header.strip_prefix("Bearer ") {
        token
    } else {
        warn!(header = %authorize_header, "Authorization 格式错误 / Malformed Authorization header");
        return v::response::respond_any(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Authorization 格式错误，应为 Bearer <token> / Malformed Authorization header, expected Bearer <token>",
        );
    };
    let oauth2 = match init_sa_token_oath2().await {
        Ok(s) => s,
        Err(e) => {
            return v::response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sa-Token OAuth2 initialization failed: {}", e),
            )
        }
    };
    debug!("尝试刷新访问令牌 / Attempting to refresh access token");
    let new_access_token = oauth2
        .refresh_access_token(refresh_token, &q.client_id, &q.client_secret)
        .await;
    match new_access_token {
        Ok(access_token) => v::response::respond_any(actix_web::http::StatusCode::OK, access_token),
        Err(e) => {
            return v::response::respond_any(
                actix_web::http::StatusCode::BAD_REQUEST,
                format!("Sa-Token OAuth2 refresh access token failed: {}", e),
            )
        }
    }
}
