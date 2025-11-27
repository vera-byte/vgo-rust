# 代码质量改进说明 / Code Quality Improvements

## 变更前 / Before

- 消息命名耦合：类型名 `WuKongMessage` 出现在核心数据路径
- 警告较多：未使用导入/变量、冗余 `mut`、未读字段、未使用方法
- Clippy 提示：`or_insert_with` 使用、冗余 `return`、模式匹配冗余、借用冗余
- 特性模块未门控引用：`cluster::raft_async` 导入失败

## 变更后 / After

- 统一消息封装：`ImMessage`，移除所有 WuKong 命名与引用
- 警告清零（本服务）：系统性清理导入、变量、绑定与未用成员
- Clippy 优化落地：
  - `.or_default()` 替代 `.or_insert_with(DashSet::new)`
  - 删除冗余 `return`，简化错误分支与匹配
  - 修复 `needless_borrow` 与 `redundant_pattern_matching`
  - QUIC 与异步 Raft 按特性门控，避免未定义导入
- 注释与文档：为关键结构/方法补充中英文注释与文档

## 影响评估 / Impact

- 可维护性：命名更清晰、模块边界更明确
- 稳定性：编译更干净，静态分析更友好
- 性能：无负面影响；部分路径减少不必要绑定与借用

