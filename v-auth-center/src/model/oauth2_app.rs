// OAuth2 应用模型 / OAuth2 Application model
// 说明: 业务模型字段与数据库结构统一为蛇形命名（snake_case）
// Description: Business model fields aligned with database schema using snake_case

use serde::{Deserialize, Serialize};
use v::db::model::{ColType, ColumnDef, ModelSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2App {
    pub id: i64,
    pub tenant_id: Option<i64>,
    pub name: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub confidentiality: i16,
    pub first_party: bool,
    pub status: i16,
    pub revoked_at: Option<chrono::NaiveDateTime>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub scopes: Vec<String>,
    pub token_auth_method: String,
    pub logo_url: Option<String>,
    pub website_url: Option<String>,
    pub contacts: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

pub const TABLE_NAME: &str = "oauth2_app";
pub const TABLE_GROUP: &str = "default";

// DbModel 由构建脚本自动实现 / DbModel is auto-implemented by build script

impl ModelSpec for OAuth2App {
    fn columns() -> &'static [ColumnDef] {
        static COLS: &[ColumnDef] = &[
            ColumnDef {
                name: "id",
                ty: ColType::Int64,
            },
            ColumnDef {
                name: "tenant_id",
                ty: ColType::Int64,
            },
            ColumnDef {
                name: "name",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "client_id",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "client_secret",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "confidentiality",
                ty: ColType::Int16,
            },
            ColumnDef {
                name: "first_party",
                ty: ColType::Bool,
            },
            ColumnDef {
                name: "status",
                ty: ColType::Int16,
            },
            ColumnDef {
                name: "revoked_at",
                ty: ColType::Timestamp,
            },
            ColumnDef {
                name: "redirect_uris",
                ty: ColType::ArrayText,
            },
            ColumnDef {
                name: "grant_types",
                ty: ColType::ArrayText,
            },
            ColumnDef {
                name: "response_types",
                ty: ColType::ArrayText,
            },
            ColumnDef {
                name: "scopes",
                ty: ColType::ArrayText,
            },
            ColumnDef {
                name: "token_auth_method",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "logo_url",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "website_url",
                ty: ColType::Text,
            },
            ColumnDef {
                name: "contacts",
                ty: ColType::Json,
            },
            ColumnDef {
                name: "metadata",
                ty: ColType::Json,
            },
            ColumnDef {
                name: "created_at",
                ty: ColType::Timestamp,
            },
            ColumnDef {
                name: "updated_at",
                ty: ColType::Timestamp,
            },
            ColumnDef {
                name: "deleted_at",
                ty: ColType::Timestamp,
            },
        ];
        COLS
    }
}
