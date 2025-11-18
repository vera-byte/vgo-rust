// 用户业务模型 / User business model
use serde::{Deserialize, Serialize};
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub tenant_id: Option<i32>,
    pub username: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub password_hash: String,
    pub real_name: Option<String>,
    pub nick_name: Option<String>,
    pub avatar_url: Option<String>,
    pub gender: i16,
    pub birthday: Option<chrono::NaiveDate>,
    pub id_card: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub status: i16,
    pub is_email_verified: bool,
    pub is_mobile_verified: bool,
    pub last_login_time: Option<chrono::NaiveDateTime>,
    pub last_login_ip: Option<String>,
    pub login_count: i32,
    pub source: Option<String>,
    pub invite_code: Option<String>,
    pub referrer_id: Option<i64>,
    pub remark: Option<String>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

pub const TABLE_NAME: &str = "user";
pub const TABLE_GROUP: &str = "default";
