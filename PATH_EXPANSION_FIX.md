# Socket 路径展开修复 / Socket Path Expansion Fix

## 问题 / Problem

配置文件中的 `~/vp/sockets/runtime.sock` 没有被正确展开，导致在项目目录下创建了 `~/vp/sockets/` 目录。

## 修复 / Fix

已在 `v-connect-im/src/main.rs` 中添加 `~` 路径展开逻辑：

```rust
// 展开 ~ 为用户主目录 / Expand ~ to user home directory
let socket_path = if socket_path.starts_with("~/") {
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = std::path::Path::new(&home);
        home_path.join(&socket_path[2..]).to_string_lossy().to_string()
    } else {
        socket_path
    }
} else {
    socket_path
};
```

## 清理错误目录 / Clean Up Wrong Directory

如果在项目中创建了错误的目录，请手动删除：

```bash
# 删除项目中错误创建的目录
cd /Users/mac/workspace/vgo-rust/v-connect-im
rm -rf '~'

# 或者如果在其他位置
find . -name '~' -type d -exec rm -rf {} +
```

## 验证 / Verification

重新启动 v-connect-im，应该看到正确的路径：

```bash
cd /Users/mac/workspace/vgo-rust/v-connect-im
cargo run
```

**期望日志：**
```
🔌 Unix Socket server starting on: /Users/mac/vp/sockets/runtime.sock
```

**不应该是：**
```
🔌 Unix Socket server starting on: ~/vp/sockets/runtime.sock
```

## 配置示例 / Configuration Examples

### 使用 ~ 展开（推荐）

```toml
[plugins]
socket_path = "~/vp/sockets/runtime.sock"
# 展开为: /Users/mac/vp/sockets/runtime.sock
```

### 使用绝对路径

```toml
[plugins]
socket_path = "/Users/mac/vp/sockets/runtime.sock"
```

### 使用相对路径

```toml
[plugins]
socket_path = "./plugins/sockets/runtime.sock"
# 相对于 v-connect-im 项目目录
```

## 其他路径配置 / Other Path Configurations

同样的逻辑也适用于其他路径配置：

### 开发模式插件路径

```toml
[plugins]
dev_plugins = [
    "example:~/workspace/v-connect-im-plugin-example",
]
```

**注意：** 开发插件路径暂时不支持 `~` 展开，请使用绝对路径：

```toml
[plugins]
dev_plugins = [
    "example:/Users/mac/workspace/v-connect-im-plugin-example",
]
```

### 插件目录

```toml
[plugins]
plugin_dir = "./plugins"  # 相对路径
# 或
plugin_dir = "/Users/mac/vp/plugins"  # 绝对路径
```

## 最佳实践 / Best Practices

1. **Socket 路径**：使用 `~/vp/sockets/runtime.sock`（支持 `~` 展开）
2. **插件目录**：使用相对路径 `./plugins`（相对于项目）
3. **开发插件路径**：使用绝对路径（暂不支持 `~`）

## 创建必要的目录 / Create Required Directories

首次运行前，确保目录存在：

```bash
# 创建 socket 目录
mkdir -p ~/vp/sockets

# 创建插件目录（如果使用绝对路径）
mkdir -p ~/vp/plugins
```

v-connect-im 会自动创建相对路径的目录。

## 故障排查 / Troubleshooting

### 问题：Socket 文件未创建

**检查路径：**
```bash
# 查看实际创建的 socket 文件
ls -la ~/vp/sockets/

# 或查看日志中的路径
cargo run 2>&1 | grep "Socket server"
```

### 问题：权限错误

```bash
# 确保目录有写权限
chmod 755 ~/vp/sockets
```

### 问题：路径仍然不正确

**检查环境变量：**
```bash
echo $HOME
# 应该输出: /Users/mac
```

**手动测试路径展开：**
```bash
cd /Users/mac/workspace/vgo-rust/v-connect-im
cargo run 2>&1 | grep -E "Socket|socket"
```

应该看到展开后的完整路径，而不是 `~`。
