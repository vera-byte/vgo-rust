# VSCode launch.json 配置说明 / VSCode launch.json Configuration Guide

## 问题原因 / Root Cause

之前的 `launch.json` 配置不完整，缺少关键字段，导致：
1. VSCode 无法正确启动调试器
2. 参数没有传递给程序
3. 插件使用了默认的 socket 路径

## 完整配置 / Complete Configuration

### 必需字段 / Required Fields

```json
{
    "version": "0.2.0",  // ← 必需：配置文件版本
    "configurations": [
        {
            "name": "Debug Plugin (example)",  // 配置名称
            "type": "lldb",                    // ← 必需：调试器类型
            "request": "launch",               // ← 必需：启动类型
            "cargo": {                         // ← 必需：Cargo 构建配置
                "args": [
                    "build",
                    "--bin=example",
                    "--package=v-connect-im-plugin-example"
                ],
                "filter": {
                    "name": "example",
                    "kind": "bin"
                }
            },
            "args": [                          // 传递给程序的参数
                "--socket",
                "${env:HOME}/vp/sockets/runtime.sock",
                "--debug"
            ],
            "cwd": "${workspaceFolder}"        // 工作目录
        }
    ]
}
```

## 字段说明 / Field Descriptions

### 1. version
```json
"version": "0.2.0"
```
- **必需**
- VSCode launch 配置文件的版本号
- 固定值：`"0.2.0"`

### 2. type
```json
"type": "lldb"
```
- **必需**
- 调试器类型
- macOS/Linux Rust 项目使用 `"lldb"`
- Windows 使用 `"cppvsdbg"` 或 `"lldb"`

### 3. request
```json
"request": "launch"
```
- **必需**
- 启动模式
- `"launch"` - 启动新进程
- `"attach"` - 附加到已运行的进程

### 4. cargo
```json
"cargo": {
    "args": [
        "build",
        "--bin=example",
        "--package=v-connect-im-plugin-example"
    ],
    "filter": {
        "name": "example",
        "kind": "bin"
    }
}
```
- **必需**（对于 Rust 项目）
- 告诉 VSCode 如何构建 Rust 项目
- `args` - 传递给 `cargo` 的参数
- `filter` - 指定要调试的二进制文件

### 5. args
```json
"args": [
    "--socket",
    "${env:HOME}/vp/sockets/runtime.sock",
    "--debug"
]
```
- **可选**
- 传递给程序的命令行参数
- 支持变量替换：`${env:HOME}`, `${workspaceFolder}` 等

### 6. cwd
```json
"cwd": "${workspaceFolder}"
```
- **可选**
- 程序运行的工作目录
- `${workspaceFolder}` - 当前工作区根目录

## 常见错误 / Common Mistakes

### ❌ 错误 1：缺少 type 和 request

```json
{
    "configurations": [
        {
            "name": "Debug Plugin",
            // ❌ 缺少 "type": "lldb"
            // ❌ 缺少 "request": "launch"
            "args": ["--socket", "..."]
        }
    ]
}
```

**结果：** VSCode 无法启动调试器

### ❌ 错误 2：缺少 cargo 配置

```json
{
    "configurations": [
        {
            "name": "Debug Plugin",
            "type": "lldb",
            "request": "launch",
            // ❌ 缺少 "cargo" 配置
            "args": ["--socket", "..."]
        }
    ]
}
```

**结果：** VSCode 不知道如何构建 Rust 项目

### ❌ 错误 3：缺少 version

```json
{
    // ❌ 缺少 "version": "0.2.0"
    "configurations": [...]
}
```

**结果：** VSCode 可能无法正确解析配置

## 变量替换 / Variable Substitution

VSCode 支持在配置中使用变量：

### 环境变量
```json
"args": [
    "--socket",
    "${env:HOME}/vp/sockets/runtime.sock"  // $HOME 环境变量
]
```

### 工作区变量
```json
"cwd": "${workspaceFolder}",              // 工作区根目录
"args": [
    "--socket",
    "${workspaceFolder}/plugins/sockets/runtime.sock"
]
```

### 其他常用变量
```json
"${file}"                  // 当前打开的文件
"${fileBasename}"          // 当前文件名
"${fileDirname}"           // 当前文件所在目录
"${workspaceFolderBasename}" // 工作区名称
```

## 调试流程 / Debug Flow

### 1. 按 F5 启动调试

VSCode 会执行以下步骤：

```bash
# 1. 构建项目
cargo build --bin=example --package=v-connect-im-plugin-example

# 2. 启动调试器
lldb target/debug/example

# 3. 传递参数
--socket /Users/mac/vp/sockets/runtime.sock --debug

# 4. 运行程序
run
```

### 2. 查看实际执行的命令

在 VSCode 的 "Debug Console" 中可以看到：

```
Running: cargo build --bin=example --package=v-connect-im-plugin-example
   Compiling v-connect-im-plugin-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.23s

Running: lldb target/debug/example -- --socket /Users/mac/vp/sockets/runtime.sock --debug
```

## 多配置示例 / Multiple Configurations

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "Debug (default socket)",
            "type": "lldb",
            "request": "launch",
            "cargo": {
                "args": ["build", "--bin=example"]
            },
            "args": [
                "--socket",
                "${env:HOME}/vp/sockets/runtime.sock",
                "--debug"
            ]
        },
        {
            "name": "Debug (trace level)",
            "type": "lldb",
            "request": "launch",
            "cargo": {
                "args": ["build", "--bin=example"]
            },
            "args": [
                "--socket",
                "${env:HOME}/vp/sockets/runtime.sock",
                "--log-level",
                "trace"
            ]
        },
        {
            "name": "Debug (custom socket)",
            "type": "lldb",
            "request": "launch",
            "cargo": {
                "args": ["build", "--bin=example"]
            },
            "args": [
                "--socket",
                "./plugins/sockets/runtime.sock",
                "--debug"
            ]
        }
    ]
}
```

## 验证配置 / Verify Configuration

### 1. 检查配置是否有效

在 VSCode 中：
1. 打开 "Run and Debug" 面板（⇧⌘D）
2. 查看配置下拉列表
3. 应该能看到所有配置名称

### 2. 测试参数传递

在插件代码中添加日志：

```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // 打印所有命令行参数
    let args: Vec<String> = std::env::args().collect();
    println!("Args: {:?}", args);
    
    // ... 其他代码
}
```

启动调试后应该看到：

```
Args: ["target/debug/example", "--socket", "/Users/mac/vp/sockets/runtime.sock", "--debug"]
```

### 3. 验证 socket 路径

在插件启动日志中应该看到：

```
📡 Socket path: /Users/mac/vp/sockets/runtime.sock
```

**不应该是：**
```
📡 Socket path: ./plugins/v.plugin.example.sock
```

## 故障排查 / Troubleshooting

### 问题：参数没有传递

**症状：**
```
socket not found: ./plugins/v.plugin.example.sock
```

**原因：** `launch.json` 配置不完整

**解决：** 确保包含所有必需字段（见上文）

### 问题：调试器无法启动

**症状：**
```
Could not find lldb-mi
```

**解决：** 安装 CodeLLDB 扩展
1. 打开 Extensions (⇧⌘X)
2. 搜索 "CodeLLDB"
3. 安装

### 问题：找不到二进制文件

**症状：**
```
Error: No such file or directory
```

**解决：** 检查 `cargo.filter.name` 是否正确

```json
"cargo": {
    "filter": {
        "name": "example",  // ← 必须与 Cargo.toml 中的 [[bin]] name 匹配
        "kind": "bin"
    }
}
```

### 问题：环境变量未展开

**症状：**
```
socket not found: ${env:HOME}/vp/sockets/runtime.sock
```

**原因：** VSCode 版本太旧或配置错误

**解决：**
1. 更新 VSCode 到最新版本
2. 或使用绝对路径：
```json
"args": [
    "--socket",
    "/Users/mac/vp/sockets/runtime.sock"
]
```

## 完整示例 / Complete Example

**文件：** `/Users/mac/workspace/v-connect-im-plugin-example/.vscode/launch.json`

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "Debug Plugin (example)",
            "type": "lldb",
            "request": "launch",
            "cargo": {
                "args": [
                    "build",
                    "--bin=example",
                    "--package=v-connect-im-plugin-example"
                ],
                "filter": {
                    "name": "example",
                    "kind": "bin"
                }
            },
            "args": [
                "--socket",
                "${env:HOME}/vp/sockets/runtime.sock",
                "--debug"
            ],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

现在配置应该正确工作了！🎯
