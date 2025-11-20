use crate::service::oauth2_service::OAuth2Server;
use actix_web::{web, Responder};
use serde::Deserialize;
use v::response;
use validator::Validate;
/// 授权端点注册（GET）
/// Register authorization endpoint (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(oauth2_authorize_handle)));
}

/// 授权请求参数
/// Authorization request parameters
#[derive(Deserialize, Validate)]
pub struct OAuth2AuthorizeHandleReq {
    /// OAuth2 响应类型，应为 "code"
    /// OAuth2 response type, expected to be "code"
    #[validate(length(min = 3, max = 50, message = "response_type长度必须在3-50个字符之间"))]
    response_type: String,

    /// 客户端标识
    /// OAuth2 client identifier
    #[validate(length(min = 3, max = 50, message = "client_id长度必须在3-50个字符之间"))]
    client_id: String,

    /// 回调地址
    /// Redirect URI
    #[validate(url(message = "redirect_uri 格式不正确"))]
    redirect_uri: String,

    /// 授权范围（可选，空格分隔）
    /// Authorization scopes (optional, space-separated)
    scope: Option<String>,

    /// 状态（可选）
    /// State (optional)
    state: Option<String>,

    /// 是否重定向到回调地址（默认 true）
    /// Whether to redirect to callback URI (default true)
    is_redirect: Option<bool>,
}

/// 处理 OAuth2 授权请求
/// Handle OAuth2 authorization request
pub async fn oauth2_authorize_handle(path: web::Query<OAuth2AuthorizeHandleReq>) -> impl Responder {
    // 参数校验 / Validate request params
    if let Err(e) = path.validate() {
        return response::respond_any(actix_web::http::StatusCode::BAD_REQUEST, format!("{}", e));
    }
    let query = path.into_inner();
    // 仅支持 Authorization Code 流程 / Only support Authorization Code flow
    if query.response_type.to_lowercase() != "code" {
        return response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            "response_type 必须为 'code' / response_type must be 'code'",
        );
    }
    return match OAuth2Server::oauth2_authorize(
        query.scope.unwrap_or_default(),
        query.response_type.clone(),
        query.client_id.clone(),
        query.redirect_uri.clone(),
        query.state.clone().unwrap_or_default(),
    )
        .await
    {
        Ok(auth) => {
            // 构建重定向URL
            let mut redirect_url = format!("{}?code={}", query.redirect_uri, auth.code);
            if let Some(state) = query.state.as_deref() {
                redirect_url.push_str(&format!("&state={}", state));
            }
            // 是否重定向到回调URL / Whether to redirect to callback URL
            let is_redirect = query.is_redirect.unwrap_or(true);
            if is_redirect {
                // 重定向到授权码回调URL / Redirect to callback with code
                return v::response::respond_any(actix_web::http::StatusCode::FOUND, redirect_url);
            }
            // 返回授权码 / Return authorization code payload
            v::response::respond_any(actix_web::http::StatusCode::OK, auth)
        }
        Err(e) => {
            response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("OAuth2 authorization failed: {}", e),
            )
        }
    };
}
