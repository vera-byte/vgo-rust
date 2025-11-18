use crate::config::sa_token_conf::init_sa_token_oath2;
use actix_web::{web, HttpRequest, Responder};
use v::response;

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(authorize_handle)));
}

use crate::repo::oauth2_app_repo;
use sa_token_plugin_actix_web::OAuth2Client;

#[derive(serde::Deserialize)]
struct AuthorizeHandleReq {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
}

pub async fn authorize_handle(
    path: web::Query<AuthorizeHandleReq>,
    req: HttpRequest,
) -> impl Responder {
    let query = path.into_inner();
    println!(
        "请求参数{:?}",
        (
            query.response_type,
            query.client_id,
            query.redirect_uri,
            query.scope,
            query.state
        )
    );
    let app_info = match oauth2_app_repo::info(18).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            return response::respond_any(
                actix_web::http::StatusCode::NOT_FOUND,
                "oauth2_app not found",
            )
        }
        Err(e) => {
            return response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}", e),
            )
        }
    };
    tracing::info!("oauth服务启动");
    let oauth2_server = match init_sa_token_oath2().await {
        Ok(s) => s,
        Err(e) => {
            return response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sa-Token OAuth2 initialization failed: {}", e),
            )
        }
    };
    tracing::info!("oauth客户端注册");
    let client = OAuth2Client {
        client_id: app_info.client_id.clone(),
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

    println!("客户端信息:");
    println!("  Client ID: {}", client.client_id);
    println!("  Client Secret: {}", client.client_secret);
    println!("  Redirect URIs: {:?}", client.redirect_uris);
    println!("  Grant Types: {:?}", client.grant_types);
    println!("  Scopes: {:?}\n", client.scope);
    let requested_scope = vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
        "offline_access".to_string(),
    ];
    // 构建回调完整 URL（不含查询参数）
    let conn_info = req.connection_info();
    let redirect_uri = format!(
        "{}://{}{}",
        conn_info.scheme(),
        conn_info.host(),
        req.path()
    );

    if !oauth2_server.validate_redirect_uri(&client, &query.redirect_uri) {
        return response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            "Invalid redirect URI",
        );
    }
    if !oauth2_server.validate_scope(&client, &requested_scope) {
        return response::respond_any(actix_web::http::StatusCode::BAD_REQUEST, "Invalid scope");
    }

    let auth_code = oauth2_server.generate_authorization_code(
        client.client_id.clone(),
        app_info.id.to_string(),
        redirect_uri.to_string(),
        requested_scope.clone(),
    );
    println!("授权码信息:");
    println!("  Code: {}", auth_code.code);
    println!("  User ID: {}", auth_code.user_id);
    println!("  Client ID: {}", auth_code.client_id);
    println!("  Redirect URI: {}", auth_code.redirect_uri);
    println!("  Scope: {:?}", auth_code.scope);
    println!("  Expires At: {}\n", auth_code.expires_at);

    oauth2_server.store_authorization_code(&auth_code).await;

    println!("✓ 授权码已存储\n");

    println!(">>> 步骤 3: 用授权码换取访问令牌\n");
    let access_token = oauth2_server
        .exchange_code_for_token(
            &auth_code.code,
            &client.client_id,
            &client.client_secret,
            &redirect_uri,
        )
        .await;
    match access_token {
        Ok(token) => {
            println!("访问令牌:");
            println!("  Access Token: {}", token.access_token);
            println!("  Token Type: {}", token.token_type);
            println!("  Expires In: {} 秒", token.expires_in);
            println!("  Refresh Token: {}", token.refresh_token.as_ref().unwrap());
            println!("  Scope: {:?}\n", token.scope);
        }
        Err(e) => {
            return response::respond_any(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("exchange_code_for_token failed: {}", e),
            );
        }
    }
    response::respond_any(actix_web::http::StatusCode::OK, app_info)
}
