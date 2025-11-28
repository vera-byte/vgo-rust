# v-connect-im-plugin-example

v-connect-im 示例插件 / Example plugin for v-connect-im

## 功能 / Features

这是一个简单的示例插件，演示如何创建和运行 v-connect-im 插件。

This is a simple example plugin demonstrating how to create and run v-connect-im plugins.

## 构建 / Build

```bash
cargo build --release
```

## 打包 / Package

使用打包脚本生成 `.wkp` 文件：

```bash
./scripts/package.sh
```

这将生成 `example.wkp` 文件，可以在 v-connect-im 中使用。

## 运行 / Run

插件可以通过以下方式运行：

1. **作为独立进程** / As standalone process:
   ```bash
   ./target/release/example --socket ./plugins/example.sock
   ```

2. **通过 v-connect-im 自动加载** / Auto-loaded by v-connect-im:
   在 `v-connect-im/config/default.toml` 中配置：
   ```toml
   [plugins]
   install = [
       "file:///path/to/example.wkp"
   ]
   ```

## 插件结构 / Plugin Structure

```
v-connect-im-plugin-example/
├── Cargo.toml          # Rust 项目配置
├── plugin.json         # 插件元数据
├── src/
│   └── main.rs         # 插件主程序
└── scripts/
    └── package.sh      # 打包脚本
```

## 通信协议 / Communication Protocol

插件通过 Unix Socket 与 v-connect-im 服务器通信：

1. 插件连接到指定的 Unix Socket
2. 发送插件信息（JSON 格式）
3. 接收服务器响应
4. 进入消息处理循环，接收和处理事件

## 事件类型 / Event Types

插件可以处理以下事件：

- `message.incoming` - 接收消息
- `message.outgoing` - 发送消息
- `room.created` - 房间创建
- `connection.established` - 连接建立
- `user.online` - 用户上线

