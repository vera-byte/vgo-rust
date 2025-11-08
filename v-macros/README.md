# 宏
## #[v_macros::base_model]
为具名字段结构体追加基础模型字段，同时保留原字段与属性
字段：
- id: 主键，自增
- created_at: 创建时间，默认当前时间
- updated_at: 更新时间，默认当前时间
- deleted_at: 删除时间，默认 None

```rust
#[v_macros::base_model]
pub struct BaseSysConf {
    pub name: String,
    pub value: String,
}
结果等于
```rust
pub struct BaseSysConf {
    pub id: i64,
    pub name: String,
    pub value: String,
    pub created_at: chrono::Utc::now(),
    pub updated_at: chrono::Utc::now(),
    pub deleted_at: Option<chrono::Utc>,
}
```