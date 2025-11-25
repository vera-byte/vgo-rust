use actix_web::{Responder, http::StatusCode, web};
use captcha_rs::CaptchaBuilder;
use serde::{Deserialize, Serialize};
use tracing::info;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(base_captcha_handle)));
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct BaseCaptchaHandleReq {
    height: u32,
    width: u32,
    color: String,
}

impl Default for BaseCaptchaHandleReq {
    fn default() -> Self {
        Self {
            height: 45,
            width: 150,
            color: "#2c3142".to_string(),
        }
    }
}
pub async fn base_captcha_handle(req: web::Query<BaseCaptchaHandleReq>) -> impl Responder {
    let params = req.into_inner();
    let captcha = CaptchaBuilder::new()
        .length(4)
        .width(params.width)
        .height(params.height)
        .dark_mode(false)
        .complexity(5) // min: 1, max: 10
        .compression(99) // min: 1, max: 99
        .build();
    info!("captcha: {}", captcha.text);
    v::response::respond_any(
        StatusCode::OK,
        serde_json::json!({
            "code": 1000,
            "message": "success",
            "data": {
                "captchaId": captcha.text,
                "data": captcha.to_base64(),
            }
        }),
    )
}
