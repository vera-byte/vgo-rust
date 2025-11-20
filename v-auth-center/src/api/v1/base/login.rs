use crate::service::base_service::BaseService;
use actix_web::{http, web, Responder};
use serde::Deserialize;
use validator::Validate;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(base_login_handle)));
}
/// 基础登录请求参数
/// Base login request parameters
#[derive(Deserialize, Validate)]
pub struct BaseLoginReq {
    /// 用户名
    /// Username
    #[validate(length(min = 3, max = 50, message = "username长度必须在3-50个字符之间"))]
    username: String,
    /// 密码
    /// Password
    #[validate(length(min = 3, max = 50, message = "password长度必须在3-50个字符之间"))]
    password: String,
}

pub async fn base_login_handle(req: web::Json<BaseLoginReq>) -> impl Responder {
    // 参数校验 / Validate request params
    if let Err(e) = req.validate() {
        return v::response::respond_any(
            actix_web::http::StatusCode::BAD_REQUEST,
            format!("{}", e),
        );
    }
    let req = req.into_inner();
    // 调用基础服务登录 / Call base service login
    let res = BaseService::login_by_password(&crate::service::base_service::LoginRequest {
        username: req.username,
        password: req.password,
    })
    .await;
    match res {
        Ok(_) => v::response::respond_any(http::StatusCode::OK, "base_login_handle"),
        Err(e) => v::response::respond_any(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}", e),
        ),
    }
}
