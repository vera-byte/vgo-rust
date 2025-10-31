// Author: 金书记
//
//! StpUtil 演示代码
//! StpUtil Demo Code

use sa_token_core::StpUtil;

/// StpUtil 演示函数
/// StpUtil Demo Function
pub async fn demo_stp_util() -> anyhow::Result<()> {
    tracing::info!("===== StpUtil 功能演示 =====");
    tracing::info!("===== StpUtil Feature Demonstration =====");

    // 1. 登录演示
    // 1. Login demonstration
    let token = StpUtil::login("demo_user").await?;
    tracing::info!("1. 登录成功，token: {}", token);
    tracing::info!("1. Login successful, token: {}", token);

    // 2. 检查登录状态
    // 2. Check login status
    let is_login = StpUtil::is_login(&token).await;
    tracing::info!("2. 是否登录: {}", is_login);
    tracing::info!("2. Is logged in: {}", is_login);

    // 3. 获取登录ID
    // 3. Get login ID
    let login_id = StpUtil::get_login_id(&token).await?;
    tracing::info!("3. 登录ID: {}", login_id);
    tracing::info!("3. Login ID: {}", login_id);

    // 4. 获取 token 信息
    // 4. Get token information
    let token_info = StpUtil::get_token_info(&token).await?;
    tracing::info!(
        "4. token信息: login_id={}, device={:?}",
        token_info.login_id,
        token_info.device
    );
    tracing::info!(
        "4. token information: login_id={}, device={:?}",
        token_info.login_id,
        token_info.device
    );

    // 5. 获取 token 有效期
    // 5. Get token expiration time
    let token_timeout = StpUtil::get_token_timeout(&token).await?;
    match token_timeout {
        Some(timeout) => {
            tracing::info!("5. token有效期: {}秒", timeout);
            tracing::info!("5. token expiration: {} seconds", timeout);
        }
        None => {
            tracing::info!("5. token有效期: 永久有效");
            tracing::info!("5. token expiration: never expires");
        }
    }

    // 6. 获取会话
    // 6. Get session
    let mut session = StpUtil::get_session(&login_id).await?;
    session.set("demo_key", "demo_value")?;
    tracing::info!("6. 会话操作成功，设置 demo_key=demo_value");
    tracing::info!("6. Session operation successful, set demo_key=demo_value");

    // 7. 权限操作
    // 7. Permission operations
    StpUtil::add_permission(&login_id, "demo:read".to_string()).await?;
    StpUtil::add_permission(&login_id, "demo:write".to_string()).await?;
    let permissions = StpUtil::get_permissions(&login_id).await;
    tracing::info!("7. 设置权限成功: {:?}", permissions);
    tracing::info!("7. Permission setting successful: {:?}", permissions);

    // 8. 角色操作
    // 8. Role operations
    StpUtil::add_role(&login_id, "demo_role".to_string()).await?;
    let roles = StpUtil::get_roles(&login_id).await;
    tracing::info!("8. 设置角色成功: {:?}", roles);
    tracing::info!("8. Role setting successful: {:?}", roles);

    // 9. 权限检查
    // 9. Permission check
    let has_permission = StpUtil::has_permission(&login_id, "demo:read").await;
    tracing::info!("9. 是否有 demo:read 权限: {}", has_permission);
    tracing::info!("9. Has demo:read permission: {}", has_permission);

    // 10. 角色检查
    // 10. Role check
    let has_role = StpUtil::has_role(&login_id, "demo_role").await;
    tracing::info!("10. 是否有 demo_role 角色: {}", has_role);
    tracing::info!("10. Has demo_role role: {}", has_role);

    // 11. 登出
    // 11. Logout
    StpUtil::logout(&token).await?;
    let is_login_after_logout = StpUtil::is_login(&token).await;
    tracing::info!("11. 登出后，是否登录: {}", is_login_after_logout);
    tracing::info!("11. After logout, is logged in: {}", is_login_after_logout);

    tracing::info!("===== StpUtil 演示完成 =====");
    tracing::info!("===== StpUtil Demonstration Completed =====");
    Ok(())
}
