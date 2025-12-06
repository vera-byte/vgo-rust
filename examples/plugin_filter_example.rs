//! # 消息过滤插件示例 / Message Filter Plugin Example
//!
//! 演示如何创建一个消息内容过滤插件
//! Demonstrates how to create a message content filter plugin
//!
//! ## 功能 / Features
//! - ✅ 敏感词过滤
//! - ✅ 垃圾消息检测
//! - ✅ 消息内容审核
//! - ✅ 自定义过滤规则
//!
//! ## 运行方式 / How to Run
//! ```bash
//! cargo run --example plugin_filter_example -- --socket ./plugins/filter.sock
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use v::plugin::pdk::{json, Context, Plugin};
use v::{debug, info, warn};

// ============================================================================
// 插件配置 / Plugin Configuration
// ============================================================================

/// 过滤插件配置 / Filter plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterConfig {
    /// 敏感词列表 / Sensitive words list
    #[serde(default)]
    sensitive_words: Vec<String>,

    /// 是否启用垃圾消息检测 / Enable spam detection
    #[serde(default = "default_enable_spam_detection")]
    enable_spam_detection: bool,

    /// 是否启用 URL 过滤 / Enable URL filtering
    #[serde(default)]
    enable_url_filter: bool,

    /// 替换字符 / Replacement character
    #[serde(default = "default_replacement")]
    replacement: String,
}

fn default_enable_spam_detection() -> bool {
    true
}

fn default_replacement() -> String {
    "*".to_string()
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            sensitive_words: vec!["垃圾".to_string(), "广告".to_string(), "spam".to_string()],
            enable_spam_detection: true,
            enable_url_filter: false,
            replacement: "*".to_string(),
        }
    }
}

// ============================================================================
// 插件主结构 / Plugin Main Structure
// ============================================================================

/// 消息过滤插件 / Message filter plugin
struct FilterPlugin {
    /// 配置 / Configuration
    config: FilterConfig,

    /// 敏感词集合（用于快速查找）/ Sensitive words set (for fast lookup)
    sensitive_words_set: HashSet<String>,

    /// 统计信息 / Statistics
    stats: FilterStats,
}

/// 过滤统计信息 / Filter statistics
#[derive(Debug, Default)]
struct FilterStats {
    /// 处理的消息总数 / Total messages processed
    total_processed: u64,

    /// 被过滤的消息数 / Messages filtered
    filtered_count: u64,

    /// 检测到的敏感词数 / Sensitive words detected
    sensitive_words_detected: u64,
}

impl Plugin for FilterPlugin {
    type Config = FilterConfig;

    /// 创建插件实例 / Create plugin instance
    fn new() -> Self {
        info!("🛡️  初始化消息过滤插件 / Initializing Filter Plugin");

        let config = FilterConfig::default();
        let sensitive_words_set: HashSet<String> = config.sensitive_words.iter().cloned().collect();

        info!(
            "📋 加载了 {} 个敏感词 / Loaded {} sensitive words",
            sensitive_words_set.len(),
            sensitive_words_set.len()
        );

        Self {
            config,
            sensitive_words_set,
            stats: FilterStats::default(),
        }
    }

    /// 获取配置 / Get configuration
    fn config(&self) -> Option<&Self::Config> {
        Some(&self.config)
    }

    /// 获取可变配置 / Get mutable configuration
    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        Some(&mut self.config)
    }

    /// 配置更新回调 / Configuration update callback
    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        info!("📝 过滤配置已更新 / Filter config updated");

        // 更新敏感词集合 / Update sensitive words set
        self.sensitive_words_set = config.sensitive_words.iter().cloned().collect();
        self.config = config;

        Ok(())
    }

    /// 声明插件能力 / Declare plugin capabilities
    fn capabilities(&self) -> Vec<String> {
        vec!["filter".into()]
    }

    /// 接收并处理事件 / Receive and handle events
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        let event_type = ctx.event_type();
        debug!("📨 收到过滤事件 / Received filter event: {}", event_type);

        match event_type {
            "filter.message" => self.handle_message_filter(ctx),
            "filter.check" => self.handle_check(ctx),
            "filter.stats" => self.handle_stats(ctx),
            _ => {
                warn!(
                    "⚠️  未知的过滤事件类型 / Unknown filter event type: {}",
                    event_type
                );
                ctx.reply(json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
                Ok(())
            }
        }
    }
}

// ============================================================================
// 事件处理方法 / Event Handler Methods
// ============================================================================

impl FilterPlugin {
    /// 处理消息过滤事件 / Handle message filter event
    fn handle_message_filter(&mut self, ctx: &mut Context) -> Result<()> {
        let content = ctx.get_payload_str("content").unwrap_or("");
        let user_id = ctx.get_payload_str("user_id").unwrap_or("unknown");

        debug!("🔍 过滤消息 / Filtering message from user: {}", user_id);

        self.stats.total_processed += 1;

        // 执行过滤 / Perform filtering
        let (filtered_content, is_filtered, violations) = self.filter_content(content);

        if is_filtered {
            self.stats.filtered_count += 1;
            warn!("⚠️  检测到违规内容 / Violations detected: {:?}", violations);
        }

        // 返回过滤结果 / Return filter result
        ctx.reply(json!({
            "status": "ok",
            "original": content,
            "filtered": filtered_content,
            "is_filtered": is_filtered,
            "violations": violations,
            "user_id": user_id
        }))?;

        info!(
            "✅ 消息过滤完成 / Message filtered, violations: {}",
            violations.len()
        );
        Ok(())
    }

    /// 处理内容检查事件 / Handle content check event
    fn handle_check(&mut self, ctx: &mut Context) -> Result<()> {
        let content = ctx.get_payload_str("content").unwrap_or("");

        // 只检查不修改 / Check only, don't modify
        let violations = self.detect_violations(content);
        let is_safe = violations.is_empty();

        ctx.reply(json!({
            "status": "ok",
            "is_safe": is_safe,
            "violations": violations
        }))?;

        Ok(())
    }

    /// 处理统计信息查询事件 / Handle stats query event
    fn handle_stats(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.reply(json!({
            "status": "ok",
            "stats": {
                "total_processed": self.stats.total_processed,
                "filtered_count": self.stats.filtered_count,
                "sensitive_words_detected": self.stats.sensitive_words_detected,
                "filter_rate": if self.stats.total_processed > 0 {
                    (self.stats.filtered_count as f64 / self.stats.total_processed as f64) * 100.0
                } else {
                    0.0
                }
            }
        }))?;

        Ok(())
    }

    /// 过滤内容 / Filter content
    ///
    /// 返回：(过滤后的内容, 是否被过滤, 违规项列表)
    /// Returns: (filtered content, is filtered, violations list)
    fn filter_content(&mut self, content: &str) -> (String, bool, Vec<String>) {
        let mut filtered = content.to_string();
        let mut violations = Vec::new();

        // 1. 敏感词过滤 / Sensitive words filtering
        for word in &self.sensitive_words_set {
            if filtered.contains(word) {
                violations.push(format!("sensitive_word: {}", word));
                self.stats.sensitive_words_detected += 1;

                // 替换敏感词 / Replace sensitive word
                let replacement = self.config.replacement.repeat(word.chars().count());
                filtered = filtered.replace(word, &replacement);
            }
        }

        // 2. 垃圾消息检测 / Spam detection
        if self.config.enable_spam_detection && self.is_spam(&filtered) {
            violations.push("spam_detected".to_string());
        }

        // 3. URL 过滤 / URL filtering
        if self.config.enable_url_filter && self.contains_url(&filtered) {
            violations.push("url_detected".to_string());
            filtered = self.remove_urls(&filtered);
        }

        let is_filtered = !violations.is_empty();
        (filtered, is_filtered, violations)
    }

    /// 检测违规项 / Detect violations
    fn detect_violations(&self, content: &str) -> Vec<String> {
        let mut violations = Vec::new();

        // 检查敏感词 / Check sensitive words
        for word in &self.sensitive_words_set {
            if content.contains(word) {
                violations.push(format!("sensitive_word: {}", word));
            }
        }

        // 检查垃圾消息 / Check spam
        if self.config.enable_spam_detection && self.is_spam(content) {
            violations.push("spam_detected".to_string());
        }

        // 检查 URL / Check URL
        if self.config.enable_url_filter && self.contains_url(content) {
            violations.push("url_detected".to_string());
        }

        violations
    }

    /// 判断是否为垃圾消息 / Check if message is spam
    fn is_spam(&self, content: &str) -> bool {
        // 简化的垃圾消息检测逻辑 / Simplified spam detection logic

        // 1. 重复字符检测 / Repeated characters detection
        let has_repeated_chars = content
            .chars()
            .collect::<Vec<_>>()
            .windows(5)
            .any(|w| w.iter().all(|&c| c == w[0]));

        // 2. 全大写检测 / All caps detection
        let is_all_caps = content.len() > 10
            && content
                .chars()
                .filter(|c| c.is_alphabetic())
                .all(|c| c.is_uppercase());

        // 3. 过多感叹号 / Too many exclamation marks
        let exclamation_count = content.chars().filter(|&c| c == '!').count();
        let has_too_many_exclamations = exclamation_count > 3;

        has_repeated_chars || is_all_caps || has_too_many_exclamations
    }

    /// 检查是否包含 URL / Check if contains URL
    fn contains_url(&self, content: &str) -> bool {
        content.contains("http://") || content.contains("https://") || content.contains("www.")
    }

    /// 移除 URL / Remove URLs
    fn remove_urls(&self, content: &str) -> String {
        // 简化的 URL 移除逻辑 / Simplified URL removal logic
        let mut result = content.to_string();

        // 移除 http/https URL
        for protocol in &["http://", "https://", "www."] {
            while let Some(start) = result.find(protocol) {
                let end = result[start..]
                    .find(|c: char| c.is_whitespace())
                    .map(|i| start + i)
                    .unwrap_or(result.len());
                result.replace_range(start..end, "[链接已过滤]");
            }
        }

        result
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 过滤插件程序入口点 / Filter plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 插件元信息 / Plugin metadata
    const PLUGIN_NO: &str = "v.plugin.filter-example";
    const VERSION: &str = "0.1.0";
    const PRIORITY: i32 = 800; // 较高优先级，在存储前过滤 / High priority, filter before storage

    info!("🚀 启动消息过滤插件示例 / Starting Filter Plugin Example");

    // 启动插件服务器 / Start plugin server
    v::plugin::pdk::run_server::<FilterPlugin>(PLUGIN_NO, VERSION, PRIORITY).await
}
