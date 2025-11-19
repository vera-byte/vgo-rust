use crate::config::sa_token_conf::init_sa_token_oath2;
use actix_web::{web, Responder};
use serde::Deserialize;
use tracing::{debug, info};
use v::response;
use validator::Validate;
/// 授权端点注册（GET）
/// Register authorization endpoint (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(oauth2_authorize_handle)));
}

use crate::repo::oauth2_app_repo;
use sa_token_plugin_actix_web::OAuth2Client;

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
    // 格式化 scope（允许为空） / Format scope (allow empty)
    let scopes: Vec<String> = query
        .scope
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(String::from)
        .collect();
    debug!(
        response_type = %query.response_type,
        client_id = %query.client_id,
        redirect_uri = %query.redirect_uri,
        scopes = ?scopes,
        state = ?query.state,
        "收到授权请求 / Received authorize request"
    );
    let app_info = match oauth2_app_repo::info_by_client_id(&query.client_id).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            // 没有查询到应用信息
            return response::respond_any(
                actix_web::http::StatusCode::BAD_REQUEST,
                "无效的client_id",
            );
        }
        Err(e) => {
            return response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}", e),
            )
        }
    };
    info!("OAuth2 服务初始化 / Initialize OAuth2 service");
    let oauth2_server = match init_sa_token_oath2().await {
        Ok(s) => s,
        Err(e) => {
            return response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sa-Token OAuth2 initialization failed: {}", e),
            )
        }
    };
    info!("注册 OAuth2 客户端 / Register OAuth2 client");
    let client = OAuth2Client {
        client_id: app_info.client_id.clone(),
        // 避免日志泄露敏感信息 / Avoid logging secrets
        client_secret: app_info.client_secret.clone().unwrap_or_default(),
        redirect_uris: app_info.redirect_uris.clone(),
        grant_types: app_info.grant_types.clone(),
        scope: app_info.scopes.clone(),
    };
    if let Err(e) = oauth2_server.register_client(&client).await {
        return response::respond_any(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("oauth2 client register failed: {}", e),
        );
    }
    debug!(
        client_id = %client.client_id,
        redirect_uris = ?client.redirect_uris,
        grant_types = ?client.grant_types,
        scopes = ?client.scope,
        "客户端已注册 / Client registered"
    );

    // 验证重定向地址 / Validate redirect URI
    if !oauth2_server.validate_redirect_uri(&client, &query.redirect_uri.as_str()) {
        return response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            "Invalid redirect URI",
        );
    }
    // 验证 scopes 是否有效 / Validate scopes
    if !oauth2_server.validate_scope(&client, &scopes) {
        return response::respond_any(actix_web::http::StatusCode::BAD_REQUEST, "Invalid scope");
    }
    // 生成授权码 / Generate authorization code
    let auth_code = oauth2_server.generate_authorization_code(
        client.client_id.clone(),
        app_info.id.to_string(),
        query.redirect_uri.clone(),
        scopes,
    );
    debug!(
        code = %auth_code.code,
        user_id = %auth_code.user_id,
        client_id = %auth_code.client_id,
        redirect_uri = %auth_code.redirect_uri,
        scope = ?auth_code.scope,
        expires_at = %auth_code.expires_at,
        "授权码生成成功 / Authorization code generated"
    );
    // 存储授权码 / Store authorization code
    if let Err(e) = oauth2_server.store_authorization_code(&auth_code).await {
        return response::respond_any(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存授权码失败: {} / Failed to store auth code: {}", e, e),
        );
    }
    // 构建重定向URL
    let mut redirect_url = format!("{}?code={}", query.redirect_uri, auth_code.code);
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
    return v::response::respond_any(actix_web::http::StatusCode::OK, auth_code);
}
