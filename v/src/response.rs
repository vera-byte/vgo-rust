use actix_web::{
    http::{header, StatusCode},
    HttpResponse,
};

// 通用 HTTP 响应封装（支持 JSON、文本、二进制）
// Generic HTTP response helpers (supports JSON, text, binary)

pub enum AutoBody {
    Json(serde_json::Value),
    Text(String),
    Bytes(Vec<u8>),
}

// 自动将 serde_json::Value 封装为 JSON 响应体
// Wrap serde_json::Value as JSON body
impl From<serde_json::Value> for AutoBody {
    fn from(v: serde_json::Value) -> Self {
        AutoBody::Json(v)
    }
}
// 自动将 String 封装为文本响应体
// Wrap String as text body
impl From<String> for AutoBody {
    fn from(s: String) -> Self {
        AutoBody::Text(s)
    }
}
// 自动将 &str 封装为文本响应体
// Wrap &str as text body
impl From<&str> for AutoBody {
    fn from(s: &str) -> Self {
        AutoBody::Text(s.to_string())
    }
}
// 自动将 Vec<u8> 封装为二进制响应体
// Wrap Vec<u8> as binary body
impl From<Vec<u8>> for AutoBody {
    fn from(b: Vec<u8>) -> Self {
        AutoBody::Bytes(b)
    }
}

// 通用响应（结构体自动转 JSON，失败则原样文本）
// Generic response: auto JSON from struct, fallback to text
pub fn respond_any<T: serde::Serialize + std::fmt::Debug>(
    code: StatusCode,
    data: T,
) -> HttpResponse {
    let code_u16 = code.as_u16();
    if (300..=399).contains(&code_u16) {
        let loc = match serde_json::to_value(&data) {
            Ok(serde_json::Value::String(s)) => s,
            Ok(v) => v.to_string(),
            Err(_) => format!("{:?}", data),
        };
        return HttpResponse::build(code)
            .insert_header((header::LOCATION, loc))
            .finish();
    }
    match serde_json::to_value(&data) {
        Ok(v) => HttpResponse::build(code).json(v),
        Err(_) => HttpResponse::build(code)
            .content_type("text/plain; charset=utf-8")
            .body(format!("{:?}", data)),
    }
}

// 指定体裁响应（JSON/Text/Binary）
// Response with explicit body kind (JSON/Text/Binary)
pub fn respond_body<B: Into<AutoBody>>(code: StatusCode, body: B) -> HttpResponse {
    match body.into() {
        AutoBody::Json(v) => HttpResponse::build(code).json(v),
        AutoBody::Text(s) => HttpResponse::build(code)
            .content_type("text/plain; charset=utf-8")
            .body(s),
        AutoBody::Bytes(b) => HttpResponse::build(code)
            .content_type("application/octet-stream")
            .body(b),
    }
}
