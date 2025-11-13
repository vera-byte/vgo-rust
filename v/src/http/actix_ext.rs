#[cfg(feature = "web_actix")]
use actix_web::{http::header::HeaderName, http::header::HeaderValue, http::StatusCode, HttpResponse, ResponseError};

use serde::Serialize;

use super::error::{GitHubErrorBody, HttpError};

#[cfg(feature = "web_actix")]
impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        let body: GitHubErrorBody = self.to_body(None);
        HttpResponse::build(StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)).json(body)
    }
}

#[cfg(feature = "web_actix")]
pub fn json_ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

#[cfg(feature = "web_actix")]
pub fn no_content() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

#[cfg(feature = "web_actix")]
pub fn ok_with_headers<T: Serialize>(data: T, headers: &[(&str, &str)]) -> HttpResponse {
    let mut builder = HttpResponse::Ok();
    for (k, v) in headers {
        let name = HeaderName::from_lowercase(k.as_bytes()).unwrap_or(HeaderName::from_static("x-invalid"));
        let value = HeaderValue::from_str(v).unwrap_or(HeaderValue::from_static(""));
        builder.insert_header((name, value));
    }
    builder.json(data)
}

#[cfg(feature = "web_actix")]
pub fn empty_with_headers(status: StatusCode, headers: &[(&str, &str)]) -> HttpResponse {
    let mut builder = HttpResponse::build(status);
    for (k, v) in headers {
        let name = HeaderName::from_lowercase(k.as_bytes()).unwrap_or(HeaderName::from_static("x-invalid"));
        let value = HeaderValue::from_str(v).unwrap_or(HeaderValue::from_static(""));
        builder.insert_header((name, value));
    }
    builder.finish()
}
