# 插件 Debug 模式使用指南 / Plugin Debug Mode Guide

## 功能特性 / Features

插件系统现在支持灵活的日志配置，包括 debug 模式。

## 命令行参数 / Command Line Arguments

### 1. Debug 模式 / Debug Mode
```bash
# 启用 debug 模式（自动设置日志级别为 DEBUG）
./example --debug
# 或使用短选项
./example -d
```

Debug 模式会额外显示：
- 目标模块名称
- 线程 ID
- 代码行号

### 2. 自定义日志级别 / Custom Log Level
```bash
# 设置为 trace 级别（最详细）
./example --log-level trace

# 设置为 debug 级别
./example --log-level debug

# 设置为 info 级别（默认）
./example --log-level info

# 设置为 warn 级别
./example --log-level warn

# 设置为 error 级别（最少）
./example --log-level error
```

### 3. 自定义 Socket 路径 / Custom Socket Path
```bash
./example --socket /custom/path/runtime.sock
```

### 4. 组合使用 / Combined Usage
```bash
# Debug 模式 + 自定义 socket
./example --debug --socket ~/vp/sockets/runtime.sock

# 自定义日志级别 + socket
./example --log-level trace --socket ~/vp/sockets/runtime.sock
```

## 日志级别说明 / Log Level Description

| 级别 / Level | 说明 / Description | 适用场景 / Use Case |
|-------------|-------------------|-------------------|
| `trace` | 最详细的日志 | 深度调试，追踪每个函数调用 |
| `debug` | 调试信息 | 开发和问题排查 |
| `info` | 一般信息（默认） | 正常运行 |
| `warn` | 警告信息 | 生产环境 |
| `error` | 错误信息 | 生产环境（最少日志） |

## 日志输出示例 / Log Output Examples

### 普通模式 / Normal Mode
```
2024-12-03T14:00:00.123Z INFO  🚀 wk.plugin.example v0.1.0 starting... (priority: 1)
2024-12-03T14:00:00.124Z INFO  📡 Socket path: ./plugins/sockets/runtime.sock
2024-12-03T14:00:00.125Z INFO  [plugin:wk.plugin.example-0.1.0] connecting socket
```

### Debug 模式 / Debug Mode
```
2024-12-03T14:00:00.123Z INFO  v::plugin::pdk:257 ThreadId(1) 🐛 Debug mode enabled
2024-12-03T14:00:00.123Z INFO  v::plugin::pdk:259 ThreadId(1) 📊 Log level: DEBUG
2024-12-03T14:00:00.124Z INFO  v::plugin::pdk:268 ThreadId(1) 🚀 wk.plugin.example v0.1.0 starting...
2024-12-03T14:00:00.125Z DEBUG v::plugin::client:119 ThreadId(2) [plugin:wk.plugin.example-0.1.0] waiting for socket
2024-12-03T14:00:00.126Z DEBUG v::plugin::client:187 ThreadId(2) [plugin:wk.plugin.example-0.1.0] config applied
2024-12-03T14:00:00.127Z DEBUG v::plugin::client:212 ThreadId(2) [plugin:wk.plugin.example-0.1.0] event: message.incoming
```

## 在配置文件中使用 / Usage in Configuration

可以在 `v-connect-im/config/default.toml` 中配置插件启动参数：

```toml
[plugins]
# 插件安装列表 / Plugin installation list
install = [
    "file://../../v-connect-im-plugin-example/wk.plugin.example-darwin-arm64.vp",
]

# 插件存储目录 / Plugin storage directory
plugin_dir = "./plugins"

# Unix Socket 通信地址 / Unix Socket communication address
socket_path = "~/vp/sockets/runtime.sock"

# 插件启动参数（未来支持）/ Plugin startup args (future support)
# [plugins.args]
# debug = true
# log_level = "debug"
```

## 代码中的日志使用 / Logging in Code

在插件代码中使用 tracing 宏：

```rust
use tracing::{trace, debug, info, warn, error};

impl Plugin for MyPlugin {
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // trace 级别 - 最详细
        trace!("Entering receive method");
        
        // debug 级别 - 调试信息
        debug!("Received message: {:?}", ctx.get_payload());
        
        // info 级别 - 一般信息
        info!("Processing message from user: {}", ctx.get_payload_str("uid")?);
        
        // warn 级别 - 警告
        warn!("Message size exceeds recommended limit");
        
        // error 级别 - 错误
        error!("Failed to process message: {}", e);
        
        Ok(())
    }
}
```

## 性能建议 / Performance Tips

1. **开发环境**：使用 `--debug` 或 `--log-level debug`
2. **测试环境**：使用 `--log-level info`（默认）
3. **生产环境**：使用 `--log-level warn` 或 `--log-level error`

Debug 模式会增加日志输出和性能开销，不建议在生产环境使用。

## 故障排查 / Troubleshooting

### 问题：看不到 debug 日志
```bash
# 确保启用了 debug 模式
./example --debug

# 或设置日志级别
./example --log-level debug
```

### 问题：日志太多
```bash
# 降低日志级别
./example --log-level warn
```

### 问题：需要追踪特定问题
```bash
# 使用 trace 级别获取最详细的日志
./example --log-level trace
```
