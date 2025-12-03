# 插件 Debug 配置指南 / Plugin Debug Configuration Guide

## 配置方式 / Configuration Methods

### 方式 1：通过配置文件（推荐）/ Method 1: Via Config File (Recommended)

编辑 `v-connect-im/config/default.toml`：

```toml
[plugins]
# 插件 Debug 模式 / Plugin debug mode
# 启用后，所有插件将以 debug 模式启动，显示详细日志
debug = true

# 插件日志级别 / Plugin log level
# 可选值: trace, debug, info, warn, error
log_level = "debug"

# Unix Socket 通信地址
socket_path = "~/vp/sockets/runtime.sock"

# 插件安装列表
install = [
    "file://../../v-connect-im-plugin-example/wk.plugin.example-darwin-arm64.vp",
]
```

### 方式 2：手动启动插件 / Method 2: Manual Plugin Start

如果需要单独调试某个插件：

```bash
# 进入插件目录
cd v-connect-im-plugin-example

# 编译插件
cargo build --release

# 手动启动（debug 模式）
./target/release/example --debug --socket ~/vp/sockets/runtime.sock

# 或指定日志级别
./target/release/example --log-level trace --socket ~/vp/sockets/runtime.sock
```

## 配置说明 / Configuration Details

### debug 参数 / debug Parameter

```toml
[plugins]
debug = true  # 启用 debug 模式 / Enable debug mode
```

**效果 / Effects:**
- 自动设置日志级别为 `DEBUG`
- 显示模块路径（如 `v::plugin::client:119`）
- 显示线程 ID（如 `ThreadId(2)`）
- 显示代码行号

### log_level 参数 / log_level Parameter

```toml
[plugins]
log_level = "debug"  # 设置日志级别 / Set log level
```

**可选值 / Available Values:**

| 级别 | 说明 | 使用场景 |
|------|------|---------|
| `trace` | 最详细的日志 | 深度调试 |
| `debug` | 调试信息 | 开发和问题排查 |
| `info` | 一般信息（默认） | 正常运行 |
| `warn` | 警告信息 | 生产环境 |
| `error` | 仅错误信息 | 生产环境（最少日志） |

**优先级 / Priority:**
- 如果同时设置 `debug = true` 和 `log_level`，`debug` 优先
- `debug = true` 等同于 `log_level = "debug"` + 额外的调试信息

## 日志输出对比 / Log Output Comparison

### 普通模式（info）/ Normal Mode (info)

```bash
2024-12-03T14:00:00.123Z INFO  🚀 wk.plugin.example v0.1.0 starting...
2024-12-03T14:00:00.124Z INFO  📡 Socket path: ~/vp/sockets/runtime.sock
2024-12-03T14:00:00.125Z INFO  [plugin:wk.plugin.example-0.1.0] connected
```

### Debug 模式 / Debug Mode

```bash
2024-12-03T14:00:00.123Z INFO  v::plugin::pdk:257 ThreadId(1) 🐛 Debug mode enabled
2024-12-03T14:00:00.123Z INFO  v::plugin::pdk:259 ThreadId(1) 📊 Log level: DEBUG
2024-12-03T14:00:00.124Z INFO  v::plugin::pdk:268 ThreadId(1) 🚀 wk.plugin.example v0.1.0 starting...
2024-12-03T14:00:00.125Z DEBUG v::plugin::client:119 ThreadId(2) [plugin:wk.plugin.example-0.1.0] waiting for socket: ~/vp/sockets/runtime.sock (retries: 10)
2024-12-03T14:00:00.626Z DEBUG v::plugin::client:187 ThreadId(2) [plugin:wk.plugin.example-0.1.0] config applied from handshake
2024-12-03T14:00:00.627Z INFO  v::plugin::client:184 ThreadId(2) [plugin:wk.plugin.example-0.1.0] handshake ack: {"status":"ok"}
2024-12-03T14:00:01.128Z DEBUG v::plugin::client:212 ThreadId(2) [plugin:wk.plugin.example-0.1.0] event: message.incoming payload={"content":"hello"}
2024-12-03T14:00:01.129Z DEBUG v::plugin::client:218 ThreadId(2) [plugin:wk.plugin.example-0.1.0] response sent: {"type":1,"content":"..."}
```

## 启动流程 / Startup Process

### 1. 启动 v-connect-im

```bash
cd v-connect-im
cargo run
```

**日志输出 / Log Output:**
```
🐛 Plugin debug mode enabled
📊 Plugin log level: debug
🔌 Plugin runtime manager initialized
🔌 Unix Socket server starting on: ~/vp/sockets/runtime.sock
Starting plugin example in debug mode
Starting plugin example with log level: debug
🚀 All plugins started
```

### 2. 插件自动启动

v-connect-im 会自动启动所有已安装的插件，并传递配置的 debug 参数：

```bash
# 实际执行的命令 / Actual command executed:
./plugins/example --socket ~/vp/sockets/runtime.sock --debug --log-level debug
```

## 使用场景 / Use Cases

### 场景 1：开发新插件

```toml
[plugins]
debug = true
log_level = "trace"  # 最详细的日志
```

### 场景 2：排查问题

```toml
[plugins]
debug = true
log_level = "debug"
```

### 场景 3：生产环境

```toml
[plugins]
debug = false
log_level = "warn"  # 仅警告和错误
```

### 场景 4：性能测试

```toml
[plugins]
debug = false
log_level = "error"  # 最少日志，最佳性能
```

## 故障排查 / Troubleshooting

### 问题：插件没有 debug 日志

**检查配置：**
```toml
[plugins]
debug = true  # 确保设置为 true
```

**检查日志：**
```bash
# 查看 v-connect-im 启动日志
cargo run 2>&1 | grep -E "debug|Debug|DEBUG"
```

应该看到：
```
🐛 Plugin debug mode enabled
Starting plugin example in debug mode
```

### 问题：日志级别不生效

**优先级顺序：**
1. `debug = true` 会覆盖 `log_level`
2. 如果要使用 `log_level`，设置 `debug = false`

```toml
[plugins]
debug = false       # 禁用 debug 模式
log_level = "info"  # 使用自定义级别
```

### 问题：日志太多影响性能

**降低日志级别：**
```toml
[plugins]
debug = false
log_level = "warn"  # 或 "error"
```

## 最佳实践 / Best Practices

1. **开发环境**：使用 `debug = true`
2. **测试环境**：使用 `log_level = "info"`
3. **生产环境**：使用 `log_level = "warn"` 或 `"error"`
4. **问题排查**：临时启用 `debug = true` 或 `log_level = "trace"`
5. **性能测试**：使用 `log_level = "error"` 减少日志开销

## 动态调整（未来支持）/ Dynamic Adjustment (Future)

未来版本可能支持运行时动态调整日志级别，无需重启服务：

```bash
# 通过 API 调整（计划中）
curl -X POST http://localhost:8080/admin/plugins/example/log-level \
  -d '{"level": "debug"}'
```
