# 插件使用指南 / Plugin Usage Guide

## 快速开始 / Quick Start

### 1. 构建插件 / Build Plugin

```bash
cd v-connect-im-plugin-example
cargo build --release
```

### 2. 打包插件 / Package Plugin

```bash
./scripts/package.sh
```

这将生成 `wk.plugin.example-{os}-{arch}.wkp` 文件。

### 3. 配置 v-connect-im / Configure v-connect-im

编辑 `v-connect-im/config/default.toml`：

```toml
[plugins]
# 插件安装 URL 列表 / Plugin installation URLs
install = [
    "file://./plugins/wk.plugin.example-darwin-arm64.wkp"
]

# 插件存储目录 / Plugin storage directory
plugin_dir = "./plugins"

# Unix Socket 通信地址 / Unix Socket communication address
socket_path = "./plugins/wukongim.sock"
```

**注意**：请根据你的操作系统和架构修改文件名（darwin/linux/windows 和 amd64/arm64）。

### 4. 启动 v-connect-im / Start v-connect-im

```bash
cd v-connect-im
cargo run --release
```

服务器启动时会：
1. 自动从 `file://` URL 安装插件
2. 解压插件到 `plugin_dir`
3. 自动发现并启动插件

## 使用构建脚本 / Using Build Script

你也可以使用 `v-connect-im/scripts/build-with-plugin.sh` 来自动完成所有步骤：

```bash
cd v-connect-im
./scripts/build-with-plugin.sh
```

这个脚本会：
1. 构建插件
2. 打包插件
3. 复制插件到 v-connect-im 的插件目录
4. 构建 v-connect-im

## 插件通信 / Plugin Communication

插件通过 Unix Socket 与 v-connect-im 服务器通信：

1. **连接阶段** / Connection Phase:
   - 插件连接到 `socket_path` 指定的 Unix Socket
   - 发送插件信息（JSON 格式）
   - 接收服务器响应

2. **消息处理阶段** / Message Processing Phase:
   - 插件进入消息循环
   - 接收来自服务器的事件（消息、房间、连接等）
   - 处理事件并发送响应

## 支持的事件类型 / Supported Event Types

- `message.incoming` - 接收消息
- `message.outgoing` - 发送消息
- `room.created` - 房间创建
- `room.joined` - 加入房间
- `room.left` - 离开房间
- `connection.established` - 连接建立
- `connection.closed` - 连接关闭
- `connection.authenticated` - 认证完成
- `user.online` - 用户上线
- `user.offline` - 用户离线

## 调试 / Debugging

### 查看插件日志 / View Plugin Logs

插件使用 `tracing` 进行日志记录，日志级别可以通过环境变量设置：

```bash
RUST_LOG=debug ./target/release/example --socket ./plugins/example.sock
```

### 查看服务器日志 / View Server Logs

服务器日志会显示插件安装和启动信息：

```
📦 Installing plugins from 1 URL(s)
✅ Plugin installed: example
🔌 Plugin runtime manager initialized
🚀 Plugin example started
```

### 测试插件 / Test Plugin

使用测试 API 验证插件功能：

```bash
# 获取插件统计信息
curl http://localhost:8080/v1/plugins/test/stats

# 列出运行时插件
curl http://localhost:8080/v1/plugins/runtime/list
```

## 故障排除 / Troubleshooting

### 插件无法启动 / Plugin Won't Start

1. 检查 socket 文件是否存在：
   ```bash
   ls -la ./plugins/*.sock
   ```

2. 检查插件二进制文件权限：
   ```bash
   chmod +x ./plugins/example/example
   ```

3. 查看服务器日志中的错误信息

### 插件无法连接 / Plugin Can't Connect

1. 确保 `socket_path` 配置正确
2. 确保插件和服务器使用相同的 socket 路径
3. 检查文件系统权限

### 插件未自动加载 / Plugin Not Auto-loaded

1. 检查 `install` 配置中的文件路径是否正确
2. 确保 `.wkp` 文件存在
3. 检查 `plugin_dir` 配置
4. 查看服务器启动日志

## 开发自定义插件 / Developing Custom Plugins

参考 `v-connect-im-plugin-example` 的结构：

1. 创建新的 Rust 项目
2. 实现插件主程序（参考 `src/main.rs`）
3. 创建 `plugin.json` 配置文件
4. 使用 `package.sh` 打包

## 更多信息 / More Information

- [插件系统文档](../v-connect-im/docs/plugin_test.md)
- [v-connect-im README](../v-connect-im/README.md)

