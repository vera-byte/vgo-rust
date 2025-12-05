//! 插件间通信功能演示 / Inter-plugin communication demo
//!
//! 运行方式 / Run with:
//! ```bash
//! cargo run --example plugin_communication_demo
//! ```

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志 / Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 插件间通信功能演示 / Inter-plugin Communication Demo");
    println!("=".repeat(60));

    demo_plugin_call().await?;
    demo_plugin_message().await?;
    demo_plugin_broadcast().await?;
    demo_event_bus().await?;

    println!("\n✅ 演示完成 / Demo completed");
    Ok(())
}

/// 演示插件 RPC 调用 / Demo plugin RPC call
async fn demo_plugin_call() -> Result<()> {
    println!("\n📞 演示 1: 插件 RPC 调用 / Demo 1: Plugin RPC Call");
    println!("-".repeat(60));

    println!("场景：插件 A 调用插件 B 的 process_data 方法");
    println!("Scenario: Plugin A calls Plugin B's process_data method");

    // 模拟调用 / Simulate call
    let request = json!({
        "from_plugin": "plugin_a",
        "to_plugin": "plugin_b",
        "method": "process_data",
        "params": {
            "data": "hello world",
            "options": {
                "uppercase": true
            }
        }
    });

    println!("\n📤 请求 / Request:");
    println!("{}", serde_json::to_string_pretty(&request)?);

    // 模拟响应 / Simulate response
    let response = json!({
        "status": "ok",
        "result": {
            "processed": "HELLO WORLD",
            "length": 11
        }
    });

    sleep(Duration::from_millis(100)).await;

    println!("\n📥 响应 / Response:");
    println!("{}", serde_json::to_string_pretty(&response)?);

    println!("\n✅ RPC 调用成功 / RPC call succeeded");

    Ok(())
}

/// 演示插件点对点消息 / Demo plugin point-to-point messaging
async fn demo_plugin_message() -> Result<()> {
    println!("\n💌 演示 2: 插件点对点消息 / Demo 2: Plugin P2P Messaging");
    println!("-".repeat(60));

    println!("场景：存储插件通知缓存插件刷新缓存");
    println!("Scenario: Storage plugin notifies cache plugin to refresh");

    let message = json!({
        "from_plugin": "storage-sled",
        "to_plugin": "cache-redis",
        "message": {
            "action": "invalidate",
            "key": "user:123",
            "timestamp": 1234567890
        }
    });

    println!("\n📤 消息 / Message:");
    println!("{}", serde_json::to_string_pretty(&message)?);

    sleep(Duration::from_millis(100)).await;

    println!("\n✅ 消息已送达 / Message delivered");

    Ok(())
}

/// 演示插件广播 / Demo plugin broadcast
async fn demo_plugin_broadcast() -> Result<()> {
    println!("\n📢 演示 3: 插件广播 / Demo 3: Plugin Broadcast");
    println!("-".repeat(60));

    println!("场景：数据更新插件广播给所有存储插件");
    println!("Scenario: Data update plugin broadcasts to all storage plugins");

    let broadcast = json!({
        "from_plugin": "data-sync",
        "message": {
            "event": "data_updated",
            "data_id": "123",
            "timestamp": 1234567890
        },
        "filter_capabilities": ["storage"]
    });

    println!("\n📤 广播消息 / Broadcast Message:");
    println!("{}", serde_json::to_string_pretty(&broadcast)?);

    sleep(Duration::from_millis(100)).await;

    // 模拟多个插件响应 / Simulate multiple plugin responses
    let responses = vec![
        ("storage-sled", json!({"status": "ok", "cached": true})),
        ("storage-redis", json!({"status": "ok", "synced": true})),
    ];

    println!("\n📥 插件响应 / Plugin Responses:");
    for (plugin, response) in responses {
        println!("  {} -> {}", plugin, serde_json::to_string(&response)?);
    }

    println!("\n✅ 广播完成，2 个插件响应 / Broadcast completed, 2 plugins responded");

    Ok(())
}

/// 演示事件总线 / Demo event bus
async fn demo_event_bus() -> Result<()> {
    println!("\n🎯 演示 4: 事件订阅/发布 / Demo 4: Event Subscription/Publication");
    println!("-".repeat(60));

    println!("场景：用户登录事件的订阅和发布");
    println!("Scenario: User login event subscription and publication");

    // 订阅事件 / Subscribe to events
    println!("\n📝 订阅事件 / Subscribe to Events:");
    let subscriptions = vec![
        ("logging-plugin", "user.*", 100),
        ("statistics-plugin", "user.login", 50),
        ("notification-plugin", "user.*", 30),
    ];

    for (plugin, pattern, priority) in &subscriptions {
        println!("  {} 订阅 {} (优先级: {})", plugin, pattern, priority);
        println!(
            "  {} subscribes to {} (priority: {})",
            plugin, pattern, priority
        );
    }

    sleep(Duration::from_millis(100)).await;

    // 发布事件 / Publish event
    println!("\n📣 发布事件 / Publish Event:");
    let event = json!({
        "publisher": "auth-plugin",
        "event_type": "user.login",
        "payload": {
            "user_id": "123",
            "username": "alice",
            "ip": "192.168.1.1",
            "timestamp": 1234567890
        }
    });

    println!("{}", serde_json::to_string_pretty(&event)?);

    sleep(Duration::from_millis(100)).await;

    // 模拟订阅者响应（按优先级顺序）/ Simulate subscriber responses (in priority order)
    println!("\n📥 订阅者响应（按优先级）/ Subscriber Responses (by priority):");
    let responses = vec![
        (
            "logging-plugin",
            json!({"status": "logged", "log_id": "log_001"}),
        ),
        (
            "statistics-plugin",
            json!({"status": "counted", "online_users": 42}),
        ),
        (
            "notification-plugin",
            json!({"status": "sent", "message_id": "msg_001"}),
        ),
    ];

    for (subscriber, response) in responses {
        println!("  {} -> {}", subscriber, serde_json::to_string(&response)?);
        sleep(Duration::from_millis(50)).await;
    }

    println!("\n✅ 事件发布完成，3 个订阅者响应 / Event published, 3 subscribers responded");

    Ok(())
}
