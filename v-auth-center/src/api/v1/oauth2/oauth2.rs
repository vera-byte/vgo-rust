use crate::config::sa_token_conf::init_sa_token_oath2;
use actix_web::web;

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(oauth2)));
}

use crate::repo::oauth2_app_repo;
use sa_token_plugin_actix_web::OAuth2Client;
use v::http::HttpError;
use v::response;

pub async fn oauth2() -> Result<web::Json<serde_json::Value>, HttpError> {
    let app_info = oauth2_app_repo::info(18)
        .await
        .map_err(|e| HttpError::Internal(e.to_string()))?
        .ok_or(HttpError::NotFound("oauth2_app not found".to_string()))?;
    tracing::info!("oauth服务启动");
    let oauth2_server = init_sa_token_oath2().await.map_err(|e| {
        HttpError::Internal(format!("Sa-Token OAuth2 initialization failed: {}", e))
    })?; // Sa-Token OAuth2 initialization failed ｜Sa-Token OAuth2 初始化失败
    tracing::info!("oauth客户端注册");
    let client = OAuth2Client {
        client_id: app_info.client_id.clone(),
        client_secret: app_info.client_secret.unwrap_or_default(),
        redirect_uris: app_info.redirect_uris.clone(),
        grant_types: app_info.grant_types.clone(),
        scope: app_info.scopes.clone(),
    };
    oauth2_server
        .register_client(&client)
        .await
        .map_err(|e| HttpError::Internal(format!("oauth2 client register failed: {}", e)))?;

    println!("客户端信息:");
    println!("  Client ID: {}", client.client_id);
    println!("  Client Secret: {}", client.client_secret);
    println!("  Redirect URIs: {:?}", client.redirect_uris);
    println!("  Grant Types: {:?}", client.grant_types);
    println!("  Scopes: {:?}\n", client.scope);

    let body = response::ok_body(&serde_json::json!(client));
    Ok(web::Json(body))
}
