use crate::config::sa_token_conf::init_sa_token_oath2;
use actix_web::{web, HttpRequest, Responder};
use sa_token_plugin_actix_web::{
    LoggingListener, MemoryStorage, OAuth2Manager, RedisStorage, SaStorage, SaTokenConfig,
    SaTokenManager, TokenStyle,
};
use serde::Deserialize;
use tracing::{debug, info, warn};
use validator::Validate;
/// 令牌端点注册（GET）
/// Register token endpoint (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(oauth2_token_handle)));
}
/// 令牌请求参数
/// Token request parameters
#[derive(Deserialize, Validate)]
pub struct OAuth2TokenHandleReq {
    /// 授权类型，应为 "authorization_code"
    /// Grant type, expected to be "authorization_code"
    #[validate(length(min = 3, max = 50, message = "grant_type长度必须在3-50个字符之间"))]
    grant_type: String,

    /// 客户端标识
    /// OAuth2 client identifier
    #[validate(length(min = 3, max = 50, message = "client_id长度必须在3-50个字符之间"))]
    client_id: String,

    /// 客户端密钥
    /// OAuth2 client secret
    #[validate(length(min = 3, max = 50, message = "client_secret长度必须在3-50个字符之间"))]
    client_secret: String,

    /// 授权码
    /// Authorization code
    #[validate(length(min = 3, max = 50, message = "code长度必须在3-50个字符之间"))]
    code: String,
}

/// 根据授权码获取访问令牌
/// Exchange authorization code for access token
pub async fn oauth2_token_handle(
    q: web::Query<OAuth2TokenHandleReq>,
    req: HttpRequest,
) -> impl Responder {
    // 参数校验 / Validate request params
    if let Err(e) = q.validate() {
        return v::response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            format!("{}", e),
        );
    }

    // 仅支持 Authorization Code / Only support Authorization Code grant
    if q.grant_type.to_lowercase() != "authorization_code" {
        return v::response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            "grant_type 必须为 'authorization_code' / grant_type must be 'authorization_code'",
        );
    }
    let oauth2_server = match init_sa_token_oath2().await {
        Ok(s) => s,
        Err(e) => {
            return v::response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sa-Token OAuth2 initialization failed: {}", e),
            )
        }
    };

    // 获取客户端请求地址 / Build current request URL
    let conn_info = req.connection_info();
    debug!("conn_info: {:?}", conn_info);
    // 构建完整的 URL（用于校验重定向的一致性）
    // Build full URL (for validating redirect consistency)
    let redirect_uri = format!(
        "{}://{}{}",
        conn_info.scheme(),
        conn_info.host(),
        req.path()
    );
    debug!("redirect_uri: {}", redirect_uri);
    info!("步骤 3：用授权码换取访问令牌 / Step 3: Exchange code for token");
    let access_token = oauth2_server
        .exchange_code_for_token(&q.code, &q.client_id, &q.client_secret, &redirect_uri)
        .await;
    match access_token {
        Ok(token) => {
            debug!(
                access_token = %token.access_token,
                token_type = %token.token_type,
                expires_in = token.expires_in,
                scope = ?token.scope,
                "令牌签发成功 / Token issued"
            );
            v::response::respond_any(actix_web::http::StatusCode::OK, &token)
        }
        Err(e) => {
            warn!(error = %e, "换取访问令牌失败 / Failed to exchange token");
            return v::response::respond_any(
                actix_web::http::StatusCode::BAD_REQUEST,
                format!("换取访问令牌失败: {} / Failed to exchange token: {}", e, e),
            );
        }
    }
}
