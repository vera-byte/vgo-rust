use async_trait::async_trait;
use sa_token_core::SaTokenListener;

pub struct MyListener;

#[async_trait]
impl SaTokenListener for MyListener {
    async fn on_login(&self, login_id: &str, _token: &str, _login_type: &str) {
        println!("用户 {} 登录了，token: {}", login_id, _token);

        // 在这里添加您的业务逻辑
        // 例如：
        // - 记录登录日志到数据库
        // - 更新用户最后登录时间
        // - 发送登录通知
        // - 统计登录次数
    }

    async fn on_logout(&self, login_id: &str, _token: &str, _login_type: &str) {
        println!("用户 {} 登出了", login_id);

        // 在这里添加您的业务逻辑
    }

    async fn on_kick_out(&self, login_id: &str, _token: &str, _login_type: &str) {
        println!("用户 {} 被踢下线了", login_id);

        // 在这里添加您的业务逻辑
    }

    // 其他事件方法是可选的
    // async fn on_renew_timeout(...) {}
    // async fn on_replaced(...) {}
    // async fn on_banned(...) {}
}
