# 插件开发模式指南 / Plugin Development Mode Guide

## 功能说明 / Feature Description

开发模式允许你直接从插件源码目录运行插件，无需编译打包。`v-connect-im` 会自动使用 `cargo run` 启动插件。

## 配置方法 / Configuration

### 1. 编辑配置文件 / Edit Config File

编辑 `v-connect-im/config/default.toml`：

```toml
[plugins]
# 开发模式插件（直接从源码运行）/ Development mode plugins (run from source)
# 格式 / Format: "plugin_name:cargo_project_path"
dev_plugins = [
    "example:/Users/mac/workspace/v-connect-im-plugin-example",
]

# 启用 debug 模式（推荐）/ Enable debug mode (recommended)
debug = true

# Unix Socket 路径
socket_path = "~/vp/sockets/runtime.sock"
```

### 2. 启动 v-connect-im

```bash
cd v-connect-im
cargo run
```

**日志输出 / Log Output:**
```
🔌 Plugin runtime manager initialized
🛠️ Registered dev plugin: example from /Users/mac/workspace/v-connect-im-plugin-example
🔌 Unix Socket server starting on: ~/vp/sockets/runtime.sock
🛠️ Starting dev plugin example with cargo run
   Compiling v-connect-im-plugin-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.34s
     Running `target/debug/example --socket ~/vp/sockets/runtime.sock --debug`
🐛 Debug mode enabled
📊 Log level: DEBUG
🚀 wk.plugin.example v0.1.0 starting... (priority: 1)
📡 Socket path: ~/vp/sockets/runtime.sock
```

## 工作原理 / How It Works

### 开发模式 vs 生产模式 / Dev Mode vs Production Mode

| 模式 | 启动方式 | 路径类型 | 编译 |
|------|---------|---------|------|
| **开发模式** | `cargo run` | 目录路径 | 自动编译 |
| **生产模式** | 直接运行二进制 | 文件路径 | 预编译 |

### 自动检测 / Auto Detection

```rust
// 如果路径是目录 -> 开发模式
if runtime.path.is_dir() {
    // 使用 cargo run
    Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(runtime.path.join("Cargo.toml"))
        .arg("--")
        .arg("--socket").arg(socket_path)
        .arg("--debug")
} else {
    // 直接运行二进制
    Command::new(&runtime.path)
        .arg("--socket").arg(socket_path)
        .arg("--debug")
}
```

## 使用场景 / Use Cases

### 场景 1：开发新插件

```toml
[plugins]
dev_plugins = [
    "my-plugin:/Users/mac/workspace/my-plugin",
]
debug = true
```

**优势：**
- ✅ 修改代码后自动重新编译
- ✅ 无需手动打包安装
- ✅ 实时查看编译错误
- ✅ 支持 debug 日志

### 场景 2：调试现有插件

```toml
[plugins]
dev_plugins = [
    "example:/Users/mac/workspace/v-connect-im-plugin-example",
]
debug = true
log_level = "trace"  # 最详细的日志
```

### 场景 3：同时开发多个插件

```toml
[plugins]
dev_plugins = [
    "plugin-a:/path/to/plugin-a",
    "plugin-b:/path/to/plugin-b",
    "plugin-c:/path/to/plugin-c",
]
debug = true
```

### 场景 4：混合模式（开发 + 生产）

```toml
[plugins]
# 开发模式插件
dev_plugins = [
    "my-new-plugin:/Users/mac/workspace/my-new-plugin",
]

# 生产模式插件（已打包安装）
install = [
    "file://../../stable-plugin/plugin.vp",
]

debug = true
```

## 开发工作流 / Development Workflow

### 1. 创建插件项目

```bash
# 复制示例项目
cp -r v-connect-im-plugin-example my-plugin
cd my-plugin

# 修改 Cargo.toml
[package]
name = "my-plugin"
```

### 2. 配置开发模式

```toml
# v-connect-im/config/default.toml
[plugins]
dev_plugins = [
    "my-plugin:/Users/mac/workspace/my-plugin",
]
debug = true
```

### 3. 启动开发

```bash
# 终端 1：启动 v-connect-im
cd v-connect-im
cargo run

# 终端 2：修改插件代码
cd my-plugin
vim src/main.rs

# 保存后，v-connect-im 会自动重启插件并重新编译
```

### 4. 实时调试

修改插件代码后：
1. 保存文件
2. v-connect-im 检测到插件退出
3. 自动重新启动（触发 `cargo run`）
4. 自动编译最新代码
5. 启动新版本插件

## 配置格式详解 / Configuration Format

### 基本格式 / Basic Format

```toml
dev_plugins = ["name:path"]
```

### 多个插件 / Multiple Plugins

```toml
dev_plugins = [
    "plugin1:/absolute/path/to/plugin1",
    "plugin2:/absolute/path/to/plugin2",
]
```

### 路径要求 / Path Requirements

- ✅ 必须是绝对路径
- ✅ 必须是 Cargo 项目目录（包含 `Cargo.toml`）
- ✅ 必须存在且可访问

### 名称要求 / Name Requirements

- ✅ 用于标识插件
- ✅ 与插件编号无关
- ✅ 建议使用简短名称

## 性能对比 / Performance Comparison

| 操作 | 开发模式 | 生产模式 |
|------|---------|---------|
| **首次启动** | 慢（需编译） | 快（直接运行） |
| **重启** | 慢（重新编译） | 快 |
| **代码修改** | 自动生效 | 需重新打包 |
| **调试** | 方便 | 需重新编译 |
| **运行性能** | debug 构建较慢 | release 构建最快 |

## 最佳实践 / Best Practices

### 1. 开发时使用 dev_plugins

```toml
[plugins]
dev_plugins = ["my-plugin:/path/to/my-plugin"]
debug = true
```

### 2. 生产时使用 install

```toml
[plugins]
install = ["file://./my-plugin.vp"]
debug = false
log_level = "warn"
```

### 3. 使用 release 构建测试性能

```bash
# 在插件目录
cargo build --release

# 临时使用 release 二进制
./target/release/my-plugin --socket ~/vp/sockets/runtime.sock
```

### 4. 版本控制

```toml
# 开发分支
[plugins]
dev_plugins = ["plugin:/path"]

# 生产分支
[plugins]
install = ["https://releases/plugin-v1.0.0.vp"]
```

## 故障排查 / Troubleshooting

### 问题 1：插件未启动

**检查路径：**
```bash
# 确保路径存在
ls /Users/mac/workspace/v-connect-im-plugin-example

# 确保有 Cargo.toml
ls /Users/mac/workspace/v-connect-im-plugin-example/Cargo.toml
```

**检查日志：**
```
Dev plugin path not found: /path/to/plugin
```

### 问题 2：编译失败

**查看完整编译输出：**
```bash
# v-connect-im 会显示 cargo 的输出
error: could not compile `my-plugin`
```

**手动测试编译：**
```bash
cd /path/to/plugin
cargo build
```

### 问题 3：插件频繁重启

**原因：** 插件代码有错误导致崩溃

**解决：**
1. 查看插件日志
2. 修复代码错误
3. 保存后自动重新编译

### 问题 4：无法连接 socket

**检查 socket 路径：**
```toml
[plugins]
socket_path = "~/vp/sockets/runtime.sock"  # 确保路径正确
```

**检查插件参数：**
```bash
# 插件应该收到正确的 socket 参数
--socket ~/vp/sockets/runtime.sock
```

## 示例项目结构 / Example Project Structure

```
workspace/
├── v-connect-im/              # 主服务
│   ├── config/
│   │   └── default.toml       # 配置 dev_plugins
│   └── src/
└── v-connect-im-plugin-example/  # 插件项目
    ├── Cargo.toml
    ├── src/
    │   └── main.rs
    └── plugin.json
```

**配置示例：**
```toml
# v-connect-im/config/default.toml
[plugins]
dev_plugins = [
    "example:/Users/mac/workspace/v-connect-im-plugin-example",
]
```

现在你可以直接修改插件代码，保存后 v-connect-im 会自动重新编译并启动插件！🚀
