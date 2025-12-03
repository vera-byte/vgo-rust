# OpenAPI 生成示例 / OpenAPI Generation Example

## 功能特性 / Features

现在 OpenAPI 生成器会自动为每个 API 端点生成：

### 1. 路径参数 / Path Parameters
对于路径如 `/user/{id}/posts/{post_id}`，会自动提取并生成：
- `id` - 路径参数
- `post_id` - 路径参数

### 2. 查询参数 / Query Parameters
对于 GET 请求，自动添加通用分页参数：
- `page` - 页码（可选）
- `limit` - 每页数量（可选）

### 3. 请求体 / Request Body
对于 POST/PUT/PATCH 请求，自动生成：
- Content-Type: `application/json`
- Schema: Object 类型

### 4. 响应结构 / Response Structure
标准响应格式：
```json
{
  "code": 0,        // 响应码 (必需)
  "message": "ok",  // 响应消息 (必需)
  "data": {}        // 响应数据 (可选)
}
```

### 5. HTTP 状态码 / HTTP Status Codes
- `200` - 成功响应（包含详细 schema）
- `400` - 请求错误
- `401` - 未授权
- `500` - 服务器错误

## API 文件示例 / API File Example

```rust
// src/api/v1/user/detail.rs

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

// 可选：自定义路由路径 / Optional: Custom route path
pub const ROUTE_PATH: &str = "/api/v1/user/{id}";

#[derive(Deserialize)]
pub struct UserQuery {
    pub include_posts: Option<bool>,
}

#[derive(Serialize)]
pub struct UserDetail {
    pub id: String,
    pub name: String,
    pub email: String,
}

pub fn register(cfg: &mut web::ServiceConfig, route: &str) {
    cfg.service(
        web::resource(route)
            .route(web::get().to(get_user))
    );
}

async fn get_user(
    path: web::Path<String>,
    query: web::Query<UserQuery>,
) -> HttpResponse {
    // 实现逻辑 / Implementation
    HttpResponse::Ok().json(serde_json::json!({
        "code": 0,
        "message": "success",
        "data": {
            "id": path.into_inner(),
            "name": "John Doe",
            "email": "john@example.com"
        }
    }))
}
```

## 生成的 OpenAPI 文档位置 / Generated OpenAPI Location

编译后会在以下位置生成 `openapi.json`：
- `target/debug/build/<project>/out/openapi.json`
- `v/src/comm/generator/openapi.json`

## 查看 OpenAPI 文档 / View OpenAPI Documentation

可以使用以下工具查看生成的 OpenAPI 文档：
1. **Swagger UI**: https://editor.swagger.io/
2. **Redoc**: https://redocly.github.io/redoc/
3. **Postman**: 导入 OpenAPI JSON 文件

## 自定义扩展 / Custom Extensions

如果需要更详细的参数定义，可以在源代码中添加注释标记：

```rust
/// @openapi
/// parameters:
///   - name: id
///     type: integer
///     description: User ID
/// responses:
///   200:
///     schema:
///       type: object
///       properties:
///         id: integer
///         name: string
pub async fn get_user(path: web::Path<i64>) -> HttpResponse {
    // ...
}
```

未来版本会支持从这些注释中提取更详细的类型信息。
