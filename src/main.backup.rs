use actix_web::{middleware::Logger, web, App, HttpServer, Responder};
use sa_token_core::StpUtil;
use sa_token_plugin_actix_web::{
    sa_check_login, sa_check_permission, sa_check_permissions_and, sa_check_role, sa_ignore,
    LoginIdExtractor, SaTokenMiddleware, SaTokenState,
};

mod auth;
mod comm;
mod conf;
mod modules;
mod stp_util_demo;

// 导入端口管理函数和配置管理器
use comm::path::get_keys;
use comm::port::{available_port, is_port_available_sync};

// 导入base模块路由配置
use modules::base::configure_base_routes;

// 导入服务器管理器
use comm::server::{get_server_info, init_server_manager, ServerConfig};

// 导入必要的类型
use auth::{
    login, AddPermissionRequest, ApiError, ApiResponse, DeleteUserRequest, ManageUserRequest,
    RegisterRequest, RemovePermissionRequest, UserInfo,
};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置管理器（使用全局单例）
    let config = comm::config::get_global_config_manager()?;

    // 打印配置源信息
    println!("当前配置数据源信息:");
    config.print_sources_info();

    // 获取各种类型的配置值（使用默认值避免配置文件不存在的问题）
    let db_host: String = config.get_or("database.host", "localhost".to_string());
    let db_port: u16 = config.get_or("database.port", 5432);
    let debug_mode: bool = config.get_or("debug", false);
    let timeout: f64 = config.get_or("timeout", 30.0);
    println!("数据库: {}:{}", db_host, db_port);
    println!("调试模式: {}", debug_mode);
    println!("超时时间: {}", timeout);

    // 使用便捷方法
    println!(
        "日志级别: {}",
        config
            .get_string("logging.level")
            .unwrap_or("info".to_string())
    );

    let _keys = get_keys().unwrap_or_else(|_| "default".to_string());
    // 初始化日志
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    tracing::info!("🚀 启动 sa-token-rust Actix-web 完整示例");
    tracing::info!("🚀 Starting sa-token-rust Actix-web complete example");

    // 1. 初始化 Sa-Token (StpUtil会自动初始化)
    // 1. Initialize Sa-Token (StpUtil will be automatically initialized)
    let sa_token_manager = conf::init_sa_token(None)
        .await
        .expect("Sa-Token initialization failed"); // Sa-Token initialization failed ｜Sa-Token 初始化失败

    // 创建 Sa-Token 状态
    // Create Sa-Token state
    let sa_token_state = SaTokenState {
        manager: sa_token_manager.clone(),
    };

    // 2. 初始化测试权限
    // 2. Initialize test permissions
    init_test_permissions().await;

    // 3. 创建服务器配置
    // 3. Create server configuration
    let server_config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        workers: Some(8),
    };

    // 4. 初始化全局服务器管理器
    // 4. Initialize global server manager
    init_server_manager(server_config, sa_token_state)?;

    // 5. 获取服务器信息并启动
    // 5. Get server info and start server
    let (config, sa_token_state, server_port) = get_server_info()?;

    tracing::info!("服务器将在端口 {} 上启动", server_port);
    tracing::info!("Server will start on port {}", server_port);

    // 创建 Sa-Token 数据用于注入
    let sa_token_data = web::Data::new(sa_token_state.clone());

    // 服务器运行在 http://localhost:{port}
    // Server runs on http://localhost:{port}
    // 测试账号 / Test accounts:
    // admin / admin123 (拥有所有权限 / has all permissions)
    // user / user123 (普通用户权限 / normal user permissions)
    // guest / guest123 (访客权限 / guest permissions)

    let mut server = HttpServer::new(move || {
        App::new()
            // 注册 sa-token 中间件
            // Register sa-token middleware
            .wrap(Logger::default())
            .app_data(sa_token_data.clone()) // 注入 Sa-Token 到应用状态 / Inject Sa-Token into application state
            // 用来创建并注册Sa-Token的Actix-web中间件
            // Create and register Sa-Token's Actix-web middleware
            .wrap(SaTokenMiddleware::new(sa_token_state.clone()))
            // 公开接口（不需要认证）
            // Public endpoints (no authentication required)
            .route("/api/login", web::post().to(login))
            .route("/api/register", web::post().to(register))
            .route("/", web::get().to(index))
            .route("/api/health", web::get().to(health_check))
            // 需要登录的接口
            // Endpoints requiring login
            .route("/api/user/info", web::get().to(user_info))
            .route("/api/user/profile", web::get().to(user_profile))
            // 需要特定权限的接口
            // Endpoints requiring specific permissions
            .route("/api/user/list", web::get().to(list_users))
            .route("/api/user/delete", web::post().to(delete_user))
            // 需要管理员角色的接口
            // Endpoints requiring admin role
            .route("/api/admin/panel", web::get().to(admin_panel))
            .route("/api/admin/stats", web::get().to(admin_stats))
            // 需要多个权限的接口
            // Endpoints requiring multiple permissions
            .route("/api/user/manage", web::post().to(manage_user))
            // 权限管理接口（需要 admin 角色）
            // Permission management endpoints (requires admin role)
            .route("/api/permission/list", web::get().to(list_permissions))
            .route("/api/permission/add", web::post().to(add_permission))
            .route("/api/permission/remove", web::post().to(remove_permission))
            .route("/api/role/list", web::get().to(list_roles))
            // StpUtil demo endpoint
            .route("/api/demo/stp-util", web::get().to(demo_stp_util_api))
            // 配置base模块路由
            // Configure base module routes
            .configure(configure_base_routes)
    });

    if let Some(workers) = config.workers {
        server = server.workers(workers);
    }

    server
        .bind(format!("{}:{}", config.host, server_port))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

/// 初始化测试用户的权限和角色
/// Initialize test user permissions and roles
///
/// 使用 StpUtil 来管理权限和角色，简单高效！
/// Using StpUtil to manage permissions and roles, simple and efficient!
async fn init_test_permissions() {
    tracing::info!("🔐 初始化测试用户权限（使用 StpUtil）...");
    tracing::info!("🔐 Initializing test user permissions (using StpUtil)...");

    // ========== 管理员用户 (admin) ==========
    // ========== Admin user (admin) ==========
    StpUtil::set_permissions(
        "admin",
        vec![
            "user:list".to_string(),
            "user:create".to_string(),
            "user:update".to_string(),
            "user:delete".to_string(),
            "system:config".to_string(),
            "system:log".to_string(),
            "admin:*".to_string(),
        ],
    )
    .await
    .unwrap();

    StpUtil::set_roles("admin", vec!["admin".to_string(), "user".to_string()])
        .await
        .unwrap();

    tracing::info!("  ✓ admin: 权限=[user:*, system:*, admin:*], 角色=[admin, user]");
    tracing::info!("  ✓ admin: permissions=[user:*, system:*, admin:*], roles=[admin, user]");

    // ========== 普通用户 (user) ==========
    // ========== Normal user (user) ==========
    StpUtil::set_permissions(
        "user",
        vec![
            "user:list".to_string(),
            "user:view".to_string(),
            "profile:edit".to_string(),
        ],
    )
    .await
    .unwrap();

    StpUtil::set_roles("user", vec!["user".to_string()])
        .await
        .unwrap();

    tracing::info!("  ✓ user: 权限=[user:list, user:view, profile:edit], 角色=[user]");
    tracing::info!("  ✓ user: permissions=[user:list, user:view, profile:edit], roles=[user]");

    // ========== 访客用户 (guest) ==========
    // ========== Guest user (guest) ==========
    StpUtil::set_permissions("guest", vec!["user:view".to_string()])
        .await
        .unwrap();

    StpUtil::set_roles("guest", vec!["guest".to_string()])
        .await
        .unwrap();

    tracing::info!("  ✓ guest: 权限=[user:view], 角色=[guest]");
    tracing::info!("  ✓ guest: permissions=[user:view], roles=[guest]");
    tracing::info!("✅ 权限初始化完成！");
    tracing::info!("✅ Permissions initialization completed!\n");
}

// ==================== 公开接口（使用 #[sa_ignore] 宏）====================
// ==================== Public endpoints (using #[sa_ignore] macro) ====================

#[sa_ignore]
async fn index() -> impl Responder {
    "Welcome to sa-token-rust! Visit /api/health to check health."
}

#[sa_ignore]
async fn health_check() -> web::Json<serde_json::Value> {
    web::Json(serde_json::json!({
        "status": "ok",
        "service": "sa-token-rust",
        "version": "0.1.0"
    }))
}

#[sa_ignore]
async fn register(
    _state: web::Data<SaTokenState>,
    req: web::Json<RegisterRequest>,
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    // 实际应用中应该存储到数据库
    tracing::info!("用户注册: {}", req.username);

    Ok(web::Json(ApiResponse::success(
        "注册成功，请登录".to_string(),
    )))
}

// ==================== 需要登录的接口 ====================
// ==================== Endpoints requiring login ====================

#[sa_check_login]
async fn user_info(
    login_id: LoginIdExtractor,
) -> Result<web::Json<ApiResponse<UserInfo>>, ApiError> {
    let user_id = login_id.0;

    let info = UserInfo {
        id: user_id.clone(),
        username: match user_id.as_str() {
            "admin" => "admin",
            "user" => "user",
            "guest" => "guest",
            _ => "unknown",
        }
        .to_string(),
        nickname: match user_id.as_str() {
            "admin" => "管理员",
            "user" => "普通用户",
            "guest" => "访客",
            _ => "未知用户",
        }
        .to_string(),
        email: Some(format!("{}@example.com", user_id)),
    };

    Ok(web::Json(ApiResponse::success(info)))
}

#[sa_check_login]
async fn user_profile() -> Result<web::Json<ApiResponse<String>>, ApiError> {
    Ok(web::Json(ApiResponse::success("用户资料".to_string())))
}

// ==================== 需要权限的接口 ====================
// ==================== Endpoints requiring permissions ====================

#[sa_check_permission("user:list")]
async fn list_users() -> Result<web::Json<ApiResponse<Vec<UserInfo>>>, ApiError> {
    let users = vec![
        UserInfo {
            id: "1".to_string(),
            username: "admin".to_string(),
            nickname: "管理员".to_string(),
            email: Some("admin@example.com".to_string()),
        },
        UserInfo {
            id: "2".to_string(),
            username: "user".to_string(),
            nickname: "普通用户".to_string(),
            email: Some("user@example.com".to_string()),
        },
    ];

    Ok(web::Json(ApiResponse::success(users)))
}

#[sa_check_permission("user:delete")]
async fn delete_user(
    req: web::Json<DeleteUserRequest>,
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    tracing::info!("删除用户: {}", req.user_id);
    Ok(web::Json(ApiResponse::success(format!(
        "用户 {} 已删除",
        req.user_id
    ))))
}

// ==================== 权限管理接口 ====================
// ==================== Permission management endpoints ====================

/// 查询用户权限列表
/// Query user permission list
#[sa_check_role("admin")]
async fn list_permissions() -> Result<web::Json<ApiResponse<serde_json::Value>>, ApiError> {
    // 使用 StpUtil 获取权限
    // Use StpUtil to get permissions
    let admin_perms = StpUtil::get_permissions("admin").await;
    let user_perms = StpUtil::get_permissions("user").await;
    let guest_perms = StpUtil::get_permissions("guest").await;

    let data = serde_json::json!({
        "admin": admin_perms,
        "user": user_perms,
        "guest": guest_perms,
    });

    Ok(web::Json(ApiResponse::success(data)))
}

/// 为用户添加权限
/// Add permission for user
#[sa_check_role("admin")]
async fn add_permission(
    req: web::Json<AddPermissionRequest>,
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    // 使用 StpUtil 添加权限
    // Use StpUtil to add permission
    StpUtil::add_permission(&req.user_id, req.permission.clone())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("✅ 为用户 {} 添加权限: {}", req.user_id, req.permission);
    Ok(web::Json(ApiResponse::success(format!(
        "成功为用户 {} 添加权限: {}",
        req.user_id, req.permission
    ))))
}

/// 移除用户权限
/// Remove permission from user
#[sa_check_role("admin")]
async fn remove_permission(
    req: web::Json<RemovePermissionRequest>,
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    // 使用 StpUtil 移除权限
    // Use StpUtil to remove permission
    StpUtil::remove_permission(&req.user_id, &req.permission)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("✅ 移除用户 {} 的权限: {}", req.user_id, req.permission);
    Ok(web::Json(ApiResponse::success(format!(
        "成功移除用户 {} 的权限: {}",
        req.user_id, req.permission
    ))))
}

/// 查询用户角色列表
/// Query user role list
#[sa_check_role("admin")]
async fn list_roles() -> Result<web::Json<ApiResponse<serde_json::Value>>, ApiError> {
    // 使用 StpUtil 获取角色
    // Use StpUtil to get roles
    let admin_roles = StpUtil::get_roles("admin").await;
    let user_roles = StpUtil::get_roles("user").await;
    let guest_roles = StpUtil::get_roles("guest").await;

    let data = serde_json::json!({
        "admin": admin_roles,
        "user": user_roles,
        "guest": guest_roles,
    });

    Ok(web::Json(ApiResponse::success(data)))
}

// ==================== 需要角色的接口 ====================
// ==================== Endpoints requiring roles ====================

#[sa_check_role("admin")]
async fn admin_panel() -> Result<web::Json<ApiResponse<String>>, ApiError> {
    Ok(web::Json(ApiResponse::success("管理员面板".to_string())))
}

#[sa_check_role("admin")]
async fn admin_stats() -> Result<web::Json<ApiResponse<serde_json::Value>>, ApiError> {
    let stats = serde_json::json!({
        "total_users": 100,
        "active_users": 80,
        "new_users_today": 5,
    });

    Ok(web::Json(ApiResponse::success(stats)))
}

// ==================== 需要多个权限的接口 ====================
// ==================== Endpoints requiring multiple permissions ====================

#[sa_check_permissions_and("user:read", "user:write")]
async fn manage_user(
    req: web::Json<ManageUserRequest>,
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    tracing::info!("管理用户: {}", req.user_id);
    Ok(web::Json(ApiResponse::success(format!(
        "用户 {} 管理成功",
        req.user_id
    ))))
}

// ==================== StpUtil 演示接口 ====================
// ==================== StpUtil demo endpoint ====================

/// StpUtil 功能演示接口
/// StpUtil feature demonstration endpoint
#[sa_ignore]
async fn demo_stp_util_api(
    _state: web::Data<SaTokenState>, // 使用注入的 Sa-Token 状态 / Using injected Sa-Token state
) -> Result<web::Json<ApiResponse<String>>, ApiError> {
    tracing::info!("触发 StpUtil 演示...");
    tracing::info!("Triggering StpUtil demo...");

    match stp_util_demo::demo_stp_util().await {
        Ok(_) => Ok(web::Json(ApiResponse::success(
            "StpUtil 演示完成，请查看服务器日志 / StpUtil demo completed, please check server logs"
                .to_string(),
        ))),
        Err(e) => Err(ApiError::InternalError(format!(
            "演示失败 / Demo failed: {}",
            e
        ))),
    }
}
