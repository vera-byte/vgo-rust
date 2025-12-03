# 插件运行流程详解 / Plugin Runtime Flow

## 完整运行流程 / Complete Runtime Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    v-connect-im 启动                         │
│                    v-connect-im Startup                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  1. 初始化插件运行时管理器                                    │
│     Initialize PluginRuntimeManager                          │
│     - 创建 socket 目录                                       │
│     - 设置 debug 模式                                        │
│     - 设置日志级别                                           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  2. 注册开发模式插件 (可选)                                   │
│     Register Dev Plugins (Optional)                          │
│     - 读取 dev_plugins 配置                                  │
│     - 注册插件（路径为目录）                                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  3. 安装插件 (可选)                                          │
│     Install Plugins (Optional)                               │
│     - 从 URL 下载插件包                                      │
│     - 解压到 plugin_dir                                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  4. 启动 Unix Socket 服务器                                  │
│     Start Unix Socket Server                                 │
│     - 监听 socket_path                                       │
│     - 等待插件连接                                           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  5. 发现并启动所有插件                                        │
│     Discover and Start All Plugins                           │
│     - discover_plugins()                                     │
│     - start_all()                                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
        ┌───────────────────┴───────────────────┐
        ↓                                       ↓
┌──────────────────┐                  ┌──────────────────┐
│  开发模式插件     │                  │  生产模式插件     │
│  Dev Plugin      │                  │  Prod Plugin     │
└──────────────────┘                  └──────────────────┘
        ↓                                       ↓
┌──────────────────┐                  ┌──────────────────┐
│ cargo run        │                  │ ./plugin_binary  │
│ --manifest-path  │                  │ --socket <path>  │
│ Cargo.toml       │                  │ --debug          │
│ --               │                  └──────────────────┘
│ --socket <path>  │
│ --debug          │
└──────────────────┘
        ↓                                       ↓
        └───────────────────┬───────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  6. 插件连接 Socket                                          │
│     Plugin Connects to Socket                                │
│     - 连接到 Unix Socket                                     │
│     - 发送握手消息                                           │
│     - 接收配置                                               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  7. 插件运行中                                               │
│     Plugin Running                                           │
│     - 监听事件                                               │
│     - 处理消息                                               │
│     - 发送响应                                               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  8. 进程监控                                                 │
│     Process Monitoring                                       │
│     - 每秒检查进程状态                                       │
│     - 更新心跳时间                                           │
│     - 处理退出                                               │
└─────────────────────────────────────────────────────────────┘
```

## 详细步骤说明 / Detailed Steps

### 步骤 1：初始化运行时管理器

**代码位置：** `v-connect-im/src/main.rs`

```rust
// 创建运行时管理器
let mut runtime_manager = PluginRuntimeManager::new(&plugin_dir, &socket_dir);

// 设置 debug 模式
if plugin_debug {
    runtime_manager.set_debug_mode(true);
}

// 设置日志级别
if let Some(ref level) = plugin_log_level {
    runtime_manager.set_log_level(level.clone());
}

// 初始化（创建目录）
runtime_manager.init()?;
```

**日志输出：**
```
🔌 Plugin runtime manager initialized
🐛 Plugin debug mode enabled
📊 Plugin log level: debug
```

### 步骤 2：注册开发模式插件

**代码位置：** `v-connect-im/src/main.rs`

```rust
// 读取配置
let dev_plugins: Vec<String> = cm
    .get::<Vec<String>>("plugins.dev_plugins")
    .unwrap_or_default();

// 注册每个开发插件
for dev_plugin in dev_plugins {
    if let Some((name, path)) = dev_plugin.split_once(':') {
        runtime_manager.register_dev_plugin(
            name.to_string(), 
            PathBuf::from(path)
        )?;
    }
}
```

**日志输出：**
```
🛠️ Registering dev plugin: example from /Users/mac/workspace/v-connect-im-plugin-example
🛠️ Registered dev plugin: example from /Users/mac/workspace/v-connect-im-plugin-example
```

### 步骤 3：安装插件（可选）

**代码位置：** `v-connect-im/src/main.rs`

```rust
if !plugin_install_urls.is_empty() {
    let installer = PluginInstaller::new(&plugin_dir);
    installer.init()?;
    
    for url in &plugin_install_urls {
        installer.install_from_url(url)?;
    }
}
```

**支持的 URL 格式：**
- `file://../../plugin.vp` - 本地文件
- `https://example.com/plugin.vp` - HTTP 下载

### 步骤 4：启动 Unix Socket 服务器

**代码位置：** `v-connect-im/src/main.rs`

```rust
// 设置 socket 路径
runtime_manager.set_global_socket_path(&socket_path);

// 启动 socket 服务器
let socket_server = UnixSocketServer::new(
    &socket_path,
    runtime_manager_arc.clone(),
    shutdown_rx.clone(),
).await?;
```

**日志输出：**
```
🔌 Unix Socket server starting on: ~/vp/sockets/runtime.sock
```

### 步骤 5：发现并启动所有插件

**代码位置：** `v-connect-im/src/plugins/runtime.rs`

```rust
// 启动所有插件
pub async fn start_all(&self) -> Result<()> {
    // 1. 发现已安装的插件
    let installed = self.discover_plugins().await?;
    
    // 2. 启动每个插件
    for name in installed {
        self.start_plugin(&name).await?;
    }
    
    Ok(())
}
```

**discover_plugins() 流程：**
```rust
// 扫描 plugin_dir 目录
// 查找可执行文件
// 返回插件名称列表
```

**start_plugin() 流程：**
```rust
pub async fn start_plugin(&self, name: &str) -> Result<()> {
    // 1. 检查是否已运行
    if let Some(runtime) = self.plugins.get(name) {
        if runtime.status() == Running {
            return Ok(());
        }
    }
    
    // 2. 查找插件二进制文件
    let plugin_path = self.find_plugin_binary(name)?;
    
    // 3. 创建运行时信息
    let runtime = PluginRuntime::new(name, plugin_path, ...);
    
    // 4. 构建启动命令
    let mut cmd = if runtime.path.is_dir() {
        // 开发模式：cargo run
        Command::new("cargo")
            .arg("run")
            .arg("--manifest-path")
            .arg(runtime.path.join("Cargo.toml"))
            .arg("--")
    } else {
        // 生产模式：直接运行
        Command::new(&runtime.path)
    };
    
    // 5. 添加参数
    cmd.arg("--socket").arg(socket_path);
    if self.debug_mode {
        cmd.arg("--debug");
    }
    if let Some(ref level) = self.log_level {
        cmd.arg("--log-level").arg(level);
    }
    
    // 6. 启动进程
    let child = cmd.spawn()?;
    
    // 7. 监控进程
    tokio::spawn(monitor_plugin_process(...));
    
    Ok(())
}
```

**日志输出：**
```
Discovered 1 installed plugin(s)
Found plugin: example
🛠️ Starting dev plugin example with cargo run
   Compiling v-connect-im-plugin-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.34s
     Running `target/debug/example --socket ~/vp/sockets/runtime.sock --debug`
```

### 步骤 6：插件连接 Socket

**代码位置：** `v/src/plugin/client.rs`

```rust
pub async fn run(&self, shutdown: watch::Receiver<bool>) -> Result<()> {
    // 1. 连接 socket
    let stream = self.connect_with_retry(shutdown.clone()).await?;
    
    // 2. 发送握手消息
    self.handshake(&mut stream).await?;
    
    // 3. 进入事件循环
    self.event_loop(stream, shutdown).await?;
    
    Ok(())
}
```

**握手消息格式：**
```json
{
  "type": "handshake",
  "plugin_no": "wk.plugin.example",
  "version": "0.1.0",
  "priority": 1,
  "capabilities": ["message.incoming", "user.online"]
}
```

**日志输出：**
```
🚀 wk.plugin.example v0.1.0 starting... (priority: 1)
📡 Socket path: ~/vp/sockets/runtime.sock
[plugin:wk.plugin.example-0.1.0] connecting socket
[plugin:wk.plugin.example-0.1.0] handshake ack: {"status":"ok","config":{...}}
```

### 步骤 7：插件运行中

**事件循环：**
```rust
async fn event_loop(&self, stream: UnixStream, shutdown: watch::Receiver<bool>) -> Result<()> {
    loop {
        tokio::select! {
            // 接收事件
            result = async {
                // 读取消息长度
                let len = stream.read_u32().await?;
                
                // 读取消息内容
                let mut buf = vec![0u8; len as usize];
                stream.read_exact(&mut buf).await?;
                
                // 解析 JSON
                let msg: Value = serde_json::from_slice(&buf)?;
                
                // 处理事件
                let event_type = msg.get("event").and_then(|v| v.as_str())?;
                let payload = msg.get("payload")?;
                
                // 调用处理器
                let resp = self.handler.on_event(event_type, payload)?;
                
                // 发送响应
                let resp_bytes = serde_json::to_vec(&resp)?;
                stream.write_u32(resp_bytes.len() as u32).await?;
                stream.write_all(&resp_bytes).await?;
                stream.flush().await?;
                
                Ok::<(), anyhow::Error>(())
            } => {
                if let Err(e) = result {
                    error!("Event handling error: {}", e);
                    break;
                }
            }
            
            // 监听关闭信号
            _ = shutdown.changed() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }
    
    Ok(())
}
```

**日志输出：**
```
DEBUG [plugin:wk.plugin.example-0.1.0] event: message.incoming payload={"content":"hello"}
DEBUG [plugin:wk.plugin.example-0.1.0] response sent: {"type":1,"content":"..."}
```

### 步骤 8：进程监控

**代码位置：** `v-connect-im/src/plugins/runtime.rs`

```rust
async fn monitor_plugin_process(
    name: String,
    process: Arc<RwLock<Option<Child>>>,
    status: Arc<RwLock<PluginStatus>>,
    last_heartbeat: Arc<RwLock<Option<Instant>>>,
) {
    loop {
        sleep(Duration::from_secs(1)).await;
        
        let mut process_guard = process.write();
        if let Some(child) = process_guard.as_mut() {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // 进程已退出
                    if exit_status.success() {
                        info!("Plugin {} exited successfully", name);
                    } else {
                        error!("Plugin {} exited with error: {:?}", name, exit_status);
                        *status.write() = PluginStatus::Error(...);
                    }
                    break;
                }
                Ok(None) => {
                    // 进程仍在运行
                    *last_heartbeat.write() = Some(Instant::now());
                }
                Err(e) => {
                    // 检查状态出错
                    error!("Error checking plugin {} status: {}", name, e);
                    *status.write() = PluginStatus::Error(e.to_string());
                    break;
                }
            }
        } else {
            break;
        }
    }
}
```

## 插件状态转换 / Plugin State Transitions

```
Installed → Starting → Running → Stopped
    ↓           ↓          ↓
  Error ←───────┴──────────┘
```

**状态说明：**
- `Installed` - 已安装但未启动
- `Starting` - 启动中（进程已创建）
- `Running` - 运行中（已连接 socket）
- `Stopping` - 停止中
- `Stopped` - 已停止
- `Error` - 错误状态

## 关键配置 / Key Configuration

```toml
[plugins]
# 开发模式插件
dev_plugins = [
    "example:/Users/mac/workspace/v-connect-im-plugin-example",
]

# 生产模式插件
install = [
    "file://../../plugin.vp",
]

# 插件目录
plugin_dir = "./plugins"

# Socket 路径
socket_path = "~/vp/sockets/runtime.sock"

# Debug 模式
debug = true

# 日志级别
log_level = "debug"
```

## 命令行参数 / CLI Arguments

**v-connect-im 启动插件时传递：**
```bash
# 开发模式
cargo run --manifest-path /path/to/plugin/Cargo.toml -- \
  --socket ~/vp/sockets/runtime.sock \
  --debug \
  --log-level debug

# 生产模式
./plugins/example/example \
  --socket ~/vp/sockets/runtime.sock \
  --debug \
  --log-level debug
```

## 通信协议 / Communication Protocol

### 消息格式 / Message Format

**长度前缀协议：**
```
[4 bytes: length] [N bytes: JSON payload]
```

**握手消息：**
```json
{
  "type": "handshake",
  "plugin_no": "wk.plugin.example",
  "version": "0.1.0",
  "priority": 1,
  "capabilities": ["message.incoming"]
}
```

**事件消息：**
```json
{
  "event": "message.incoming",
  "payload": {
    "content": "hello",
    "from_uid": "user123"
  }
}
```

**响应消息：**
```json
{
  "type": 1,
  "content": "处理结果"
}
```

## 故障恢复 / Fault Recovery

### 插件崩溃

**检测：**
- 进程监控检测到退出
- 状态更新为 `Error`

**日志：**
```
ERROR Plugin example exited with error: ExitStatus(unix_wait_status(512))
```

**恢复：**
- 当前版本：需要手动重启
- 未来版本：可实现自动重启

### Socket 连接失败

**重试机制：**
```rust
// 插件端重试连接
let mut retries = 10;
while retries > 0 {
    match UnixStream::connect(&socket_path).await {
        Ok(stream) => return Ok(stream),
        Err(_) => {
            debug!("waiting for socket (retries: {})", retries);
            sleep(Duration::from_millis(500)).await;
            retries -= 1;
        }
    }
}
```

## 性能监控 / Performance Monitoring

**可监控指标：**
- 插件状态（Running/Error）
- 最后心跳时间
- 进程 PID
- 版本信息

**查询接口（未来）：**
```bash
# 列出所有插件
GET /api/plugins

# 查看插件状态
GET /api/plugins/example/status

# 重启插件
POST /api/plugins/example/restart
```

现在你了解了插件从启动到运行的完整流程！🚀
