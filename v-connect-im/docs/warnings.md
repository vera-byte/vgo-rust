# 警告消除报告 / Warning Elimination Report

## 基线（变更前）/ Baseline (Before)

- `v-connect-im` 主要警告：
  - 未使用导入：`async_trait::async_trait`
  - 未使用导入：`std::collections::HashMap`（在 `quic` 特性关闭时）
  - 未使用导入：`IVec`（sled）
  - 未使用变量：`client_id`、`e`、`v` 等
  - 不必要的 `mut` 绑定：若干 `DashSet`/`DashMap` 的 `get_mut`
  - 未读字段/未使用方法：`Connection.client_id`、若干 `impl` 方法
  - 多处 `WuKongMessage` 类型及引用

- `v` 公共库警告（保留）：
  - `ambiguous_glob_reexports`、`unused_mut`、`dead_code` 等（需在公共库统一治理）

## 处理措施 / Actions Taken

- 删除未使用导入：`async_trait`、`IVec`；`HashMap` 在 `quic` 特性下按需引入
- 变量预处理：未使用变量统一加下划线前缀或移除
- 绑定优化：移除不必要的 `mut` 模式绑定
- 结构体字段：对暂未读取的字段增加 `#[allow(dead_code)]`（限本服务）
- 方法标注：对暂未使用的方法增加 `#[allow(dead_code)]`
- 模块门控：`cluster::raft_async` 与 `net::quic` 按特性门控
- 消息类型重命名：`WuKongMessage` → `ImMessage`，全局替换所有引用
- Clippy 优化：
  - `or_insert_with(DashSet::new)` → `.or_default()`
  - `needless_return`、`redundant_pattern_matching`、`let_underscore_future` 修复
  - `needless_borrow` 修复 QUIC 收包与字符串借用

## 结果（变更后）/ Result (After)

- `cargo check`：`v-connect-im` 零警告；`v` 库仍有 4 个警告（需在公共库修复）
- `cargo clippy`：`v-connect-im` 仅保留少量设计建议（参数过多等），不影响编译与运行

## 后续建议 / Next Steps

- 在 `v` 公共库统一修复 re-export 与 `dead_code` 问题
- 对参数较多的方法进行结构化重构（如封装参数对象）

