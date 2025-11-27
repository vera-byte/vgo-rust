# v-connect-im 项目结构与模块关系 / Project Structure & Module Relations

## 结构概览 / Overview

- `src/main.rs`：入口、WebSocket/HTTP 服务与核心业务逻辑
  - 路由注册：`include!(OUT_DIR/api_registry.rs)` 自动挂载 `src/api/v1/**`
  - 模块索引：`include!(OUT_DIR/auto_mod.rs)`
- `src/api/v1/**`：接口分组（遵循 actix-web 与 `v::response::respond_any`）
- `src/cluster/`：目录、路由与 Raft 模块（含异步实验性 `raft_async` 特性）
- `src/net/quic.rs`：QUIC 服务（按 `quic` 特性启用）
- `src/service/`：健康检查与 Webhook
- `src/storage/mod.rs`：消息存储、离线存储、房间成员持久化
- `build.rs`：使用公共库 `v::comm::generator` 生成路由/模块代码

## 模块关系图 / Module Relation Diagram

```
               +-------------------+
               |   build.rs (v)    |
               | code generation   |
               +---------+---------+
                         |
                   includes OUT_DIR
                         |
+------------------------v------------------------+
|                  src/main.rs                    |
|  Actix App, WS server, routing, core logic     |
|  - api_registry::configure()                    |
|  - VConnectIMServer (state)                     |
|        |            |              |            |
|        |            |              |            |
|     storage      cluster         service       net |
| (sled trees)  (directory/raft) (health/webhook) (quic)
|        |            |              |            |
|   offline WAL   node map      HealthCheck     QUIC I/O
+-------------------------------------------------------+
         ^                            |
         |                            |
         +------ api/v1/* (actix) ----+
```

## 关键约定 / Key Conventions

- 使用 `v` 公共库：配置、响应、健康检查、生成器
- 接口统一使用 `actix-web` 与 `v::response::respond_any`
- 异步运行时使用 `tokio`，并在 I/O 与并发上遵循结构化并发
- 消息封装类型统一为 `ImMessage`（移除所有 WuKong 命名）

