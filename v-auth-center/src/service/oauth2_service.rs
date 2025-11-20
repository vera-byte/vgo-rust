pub struct OAuth2Server {}
use crate::config::sa_token_conf::init_sa_token_oath2;
use crate::repo::oauth2_app_repo;
use sa_token_plugin_actix_web::{AuthorizationCode, OAuth2Client};

use tracing::{debug, info};

impl OAuth2Server {
    /// 处理授权请求 / Handle authorize request
    pub async fn oauth2_authorize(
        scope: String,
        response_type: String,
        client_id: String,
        redirect_uri: String,
        state: String,
    ) -> Result<AuthorizationCode, String> {
        // 格式化 scope（允许为空） / Format scope (allow empty)
        let scopes: Vec<String> = scope.split_whitespace().map(String::from).collect();
        debug!(
            response_type = %response_type,
            client_id = %client_id,
            redirect_uri = %redirect_uri,
            scopes = ?scopes,
            state = ?state,
            "收到授权请求 / Received authorize request"
        );
        info!("OAuth2 服务初始化 / Initialize OAuth2 service");
        let oauth2_server: sa_token_plugin_actix_web::OAuth2Manager =
            match init_sa_token_oath2().await {
                Ok(s) => s,
                Err(e) => {
                    return Err(format!("Sa-Token OAuth2 initialization failed: {}", e));
                }
            };
        let app_info = match oauth2_app_repo::info_by_client_id(&client_id).await {
            Ok(Some(info)) => info,
            Ok(None) => {
                // 没有查询到应用信息
                return Err(format!("Client ID {} not found", client_id));
            }
            Err(e) => {
                return Err(format!("{}", e));
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
            return Err(format!("oauth2 client register failed: {}", e));
        }
        debug!(
            client_id = %client.client_id,
            redirect_uris = ?client.redirect_uris,
            grant_types = ?client.grant_types,
            scopes = ?client.scope,
            "客户端已注册 / Client registered"
        );
        if !oauth2_server.validate_redirect_uri(&client, &redirect_uri.as_str()) {
            return Err("Invalid redirect URI".to_string());
        }
        // 验证 scopes 是否有效 / Validate scopes
        if !oauth2_server.validate_scope(&client, &scopes) {
            return Err("Invalid scope".to_string());
        }
        // 生成授权码 / Generate authorization code
        let auth_code: AuthorizationCode = oauth2_server.generate_authorization_code(
            client.client_id.clone(),
            app_info.id.to_string(),
            redirect_uri.clone(),
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
            return Err(format!("Failed to store auth code: {}", e));
        }
        Ok(auth_code)
    }
}
