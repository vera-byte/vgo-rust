//! 插件事件订阅/发布系统 / Plugin Event Subscription/Publication System
//!
//! 提供插件间的事件订阅和发布机制
//! Provides event subscription and publication mechanism between plugins

use anyhow::Result;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::runtime::PluginConnectionPool;

/// 事件订阅信息 / Event subscription info
#[derive(Clone, Debug)]
pub struct EventSubscription {
    /// 订阅者插件名称 / Subscriber plugin name
    pub subscriber: String,
    /// 事件类型模式（支持通配符）/ Event type pattern (supports wildcards)
    pub event_pattern: String,
    /// 订阅优先级 / Subscription priority
    pub priority: i32,
}

/// 插件事件总线 / Plugin Event Bus
///
/// 管理插件间的事件订阅和发布
/// Manages event subscription and publication between plugins
pub struct PluginEventBus {
    /// 事件订阅表：event_type -> subscribers / Event subscriptions: event_type -> subscribers
    subscriptions: Arc<DashMap<String, Vec<EventSubscription>>>,
    /// 插件连接池 / Plugin connection pool
    pool: Arc<PluginConnectionPool>,
    /// 事件历史记录（可选，用于调试）/ Event history (optional, for debugging)
    event_history: Arc<RwLock<Vec<EventRecord>>>,
    /// 是否启用事件历史 / Whether to enable event history
    enable_history: bool,
}

/// 事件记录 / Event record
#[derive(Clone, Debug)]
pub struct EventRecord {
    /// 事件类型 / Event type
    pub event_type: String,
    /// 发布者插件 / Publisher plugin
    pub publisher: String,
    /// 订阅者列表 / Subscriber list
    pub subscribers: Vec<String>,
    /// 时间戳 / Timestamp
    pub timestamp: i64,
}

impl PluginEventBus {
    /// 创建新的事件总线 / Create new event bus
    pub fn new(pool: Arc<PluginConnectionPool>) -> Self {
        Self {
            subscriptions: Arc::new(DashMap::new()),
            pool,
            event_history: Arc::new(RwLock::new(Vec::new())),
            enable_history: false,
        }
    }

    /// 启用事件历史记录 / Enable event history
    pub fn enable_history(&mut self, enable: bool) {
        self.enable_history = enable;
    }

    /// 订阅事件 / Subscribe to event
    ///
    /// # 参数 / Parameters
    /// - `subscriber`: 订阅者插件名称 / Subscriber plugin name
    /// - `event_pattern`: 事件类型模式，支持通配符 `*` / Event type pattern, supports wildcard `*`
    /// - `priority`: 订阅优先级，数值越大优先级越高 / Subscription priority, higher value = higher priority
    ///
    /// # 示例 / Example
    /// ```rust
    /// // 订阅特定事件 / Subscribe to specific event
    /// event_bus.subscribe("plugin_a", "user.login", 10).await?;
    ///
    /// // 订阅所有用户相关事件 / Subscribe to all user-related events
    /// event_bus.subscribe("plugin_a", "user.*", 10).await?;
    ///
    /// // 订阅所有事件 / Subscribe to all events
    /// event_bus.subscribe("plugin_a", "*", 5).await?;
    /// ```
    pub async fn subscribe(
        &self,
        subscriber: &str,
        event_pattern: &str,
        priority: i32,
    ) -> Result<()> {
        info!(
            "📝 插件订阅事件 / Plugin subscribes to event: {} -> {} (priority: {})",
            subscriber, event_pattern, priority
        );

        let subscription = EventSubscription {
            subscriber: subscriber.to_string(),
            event_pattern: event_pattern.to_string(),
            priority,
        };

        // 添加订阅 / Add subscription
        self.subscriptions
            .entry(event_pattern.to_string())
            .or_insert_with(Vec::new)
            .push(subscription.clone());

        // 按优先级排序 / Sort by priority
        if let Some(mut subs) = self.subscriptions.get_mut(event_pattern) {
            subs.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        Ok(())
    }

    /// 取消订阅 / Unsubscribe from event
    ///
    /// # 参数 / Parameters
    /// - `subscriber`: 订阅者插件名称 / Subscriber plugin name
    /// - `event_pattern`: 事件类型模式 / Event type pattern
    pub async fn unsubscribe(&self, subscriber: &str, event_pattern: &str) -> Result<()> {
        info!(
            "🗑️  插件取消订阅 / Plugin unsubscribes: {} -> {}",
            subscriber, event_pattern
        );

        if let Some(mut subs) = self.subscriptions.get_mut(event_pattern) {
            subs.retain(|s| s.subscriber != subscriber);
        }

        Ok(())
    }

    /// 发布事件 / Publish event
    ///
    /// # 参数 / Parameters
    /// - `publisher`: 发布者插件名称 / Publisher plugin name
    /// - `event_type`: 事件类型 / Event type
    /// - `payload`: 事件载荷 / Event payload
    ///
    /// # 返回值 / Returns
    /// - `Ok(responses)`: 所有订阅者的响应 / Responses from all subscribers
    ///
    /// # 示例 / Example
    /// ```rust
    /// let responses = event_bus.publish(
    ///     "plugin_a",
    ///     "user.login",
    ///     &json!({"user_id": "123", "timestamp": 1234567890})
    /// ).await?;
    /// ```
    pub async fn publish(
        &self,
        publisher: &str,
        event_type: &str,
        payload: &Value,
    ) -> Result<Vec<(String, Value)>> {
        info!(
            "📣 发布事件 / Publish event: {} -> {}",
            publisher, event_type
        );

        let mut responses = Vec::new();
        let mut matched_subscribers = Vec::new();

        // 查找匹配的订阅者 / Find matching subscribers
        for entry in self.subscriptions.iter() {
            let pattern = entry.key();
            if self.matches_pattern(event_type, pattern) {
                for sub in entry.value().iter() {
                    // 跳过发布者自己（除非明确订阅）/ Skip publisher itself (unless explicitly subscribed)
                    if sub.subscriber == publisher && pattern != event_type {
                        continue;
                    }
                    matched_subscribers.push(sub.clone());
                }
            }
        }

        // 按优先级排序 / Sort by priority
        matched_subscribers.sort_by(|a, b| b.priority.cmp(&a.priority));

        debug!(
            "🎯 找到 {} 个订阅者 / Found {} subscribers",
            matched_subscribers.len(),
            matched_subscribers.len()
        );

        // 构建事件消息 / Build event message
        let event_message = serde_json::json!({
            "publisher": publisher,
            "event_type": event_type,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "payload": payload
        });

        // 向所有订阅者发送事件 / Send event to all subscribers
        let subscriber_names: Vec<String> = matched_subscribers
            .iter()
            .map(|s| s.subscriber.clone())
            .collect();

        for sub in matched_subscribers {
            match self
                .pool
                .send_event(&sub.subscriber, "event.published", &event_message)
                .await
            {
                Ok(Some(response)) => {
                    info!("✅ 订阅者响应 / Subscriber responded: {}", sub.subscriber);
                    responses.push((sub.subscriber.clone(), response));
                }
                Ok(None) => {
                    warn!(
                        "⚠️  订阅者未连接 / Subscriber not connected: {}",
                        sub.subscriber
                    );
                }
                Err(e) => {
                    warn!(
                        "⚠️  向订阅者发送事件失败 / Failed to send event to subscriber {}: {}",
                        sub.subscriber, e
                    );
                }
            }
        }

        // 记录事件历史 / Record event history
        if self.enable_history {
            let record = EventRecord {
                event_type: event_type.to_string(),
                publisher: publisher.to_string(),
                subscribers: subscriber_names,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            self.event_history.write().await.push(record);
        }

        info!(
            "📊 事件发布完成 / Event published: {} 个订阅者响应 / {} subscribers responded",
            responses.len(),
            responses.len()
        );

        Ok(responses)
    }

    /// 匹配事件模式 / Match event pattern
    ///
    /// 支持通配符 `*`
    /// Supports wildcard `*`
    fn matches_pattern(&self, event_type: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern == event_type {
            return true;
        }

        // 支持通配符匹配 / Support wildcard matching
        // 例如: "user.*" 匹配 "user.login", "user.logout" 等
        // Example: "user.*" matches "user.login", "user.logout", etc.
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            if event_type.starts_with(prefix) && event_type.len() > prefix.len() {
                let remaining = &event_type[prefix.len()..];
                return remaining.starts_with('.') && !remaining[1..].contains('.');
            }
        }

        false
    }

    /// 获取插件的所有订阅 / Get all subscriptions of a plugin
    pub async fn get_subscriptions(&self, plugin_name: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        for entry in self.subscriptions.iter() {
            if entry.value().iter().any(|s| s.subscriber == plugin_name) {
                patterns.push(entry.key().clone());
            }
        }
        patterns
    }

    /// 获取事件的所有订阅者 / Get all subscribers of an event
    pub async fn get_subscribers(&self, event_pattern: &str) -> Vec<String> {
        self.subscriptions
            .get(event_pattern)
            .map(|subs| subs.iter().map(|s| s.subscriber.clone()).collect())
            .unwrap_or_default()
    }

    /// 清除插件的所有订阅 / Clear all subscriptions of a plugin
    pub async fn clear_plugin_subscriptions(&self, plugin_name: &str) -> Result<()> {
        info!(
            "🧹 清除插件订阅 / Clear plugin subscriptions: {}",
            plugin_name
        );

        for mut entry in self.subscriptions.iter_mut() {
            entry.value_mut().retain(|s| s.subscriber != plugin_name);
        }

        Ok(())
    }

    /// 获取事件历史 / Get event history
    pub async fn get_event_history(&self, limit: usize) -> Vec<EventRecord> {
        let history = self.event_history.read().await;
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }

    /// 清除事件历史 / Clear event history
    pub async fn clear_event_history(&self) {
        self.event_history.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let bus = PluginEventBus::new(Arc::new(PluginConnectionPool::new(Arc::new(
            crate::plugins::runtime::PluginRuntimeManager::new("./plugins", "./sockets"),
        ))));

        assert!(bus.matches_pattern("user.login", "*"));
        assert!(bus.matches_pattern("user.login", "user.login"));
        assert!(bus.matches_pattern("user.login", "user.*"));
        assert!(!bus.matches_pattern("user.login", "admin.*"));
        assert!(!bus.matches_pattern("user.login.success", "user.*"));
    }
}
