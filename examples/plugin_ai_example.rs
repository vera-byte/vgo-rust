//! # AI 插件示例 / AI Plugin Example
//!
//! 演示如何创建一个简单的 AI 对话插件
//! Demonstrates how to create a simple AI conversation plugin
//!
//! ## 功能 / Features
//! - ✅ 接收用户消息并生成 AI 回复
//! - ✅ 支持配置自定义 AI 名称和提示词
//! - ✅ 完整的错误处理和日志记录
//!
//! ## 运行方式 / How to Run
//! ```bash
//! cargo run --example plugin_ai_example -- --socket ./plugins/ai.sock
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use v::plugin::pdk::{json, Context, Plugin};
use v::{debug, info};

// ============================================================================
// 插件配置 / Plugin Configuration
// ============================================================================

/// AI 插件配置 / AI plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AIConfig {
    /// AI 名称 / AI name
    #[serde(default = "default_ai_name")]
    ai_name: String,

    /// 系统提示词 / System prompt
    #[serde(default = "default_system_prompt")]
    system_prompt: String,

    /// 最大回复长度 / Max reply length
    #[serde(default = "default_max_length")]
    max_reply_length: usize,
}

fn default_ai_name() -> String {
    "小智助手".to_string()
}

fn default_system_prompt() -> String {
    "你是一个友好、专业的AI助手，善于解答用户的问题。".to_string()
}

fn default_max_length() -> usize {
    500
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            ai_name: default_ai_name(),
            system_prompt: default_system_prompt(),
            max_reply_length: default_max_length(),
        }
    }
}

// ============================================================================
// 插件主结构 / Plugin Main Structure
// ============================================================================

/// AI 插件 / AI Plugin
struct AIPlugin {
    /// 配置 / Configuration
    config: AIConfig,
}

impl Plugin for AIPlugin {
    type Config = AIConfig;

    /// 创建插件实例 / Create plugin instance
    fn new() -> Self {
        info!("🤖 初始化 AI 插件 / Initializing AI Plugin");

        Self {
            config: AIConfig::default(),
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
        info!("📝 AI 配置已更新 / AI config updated: {:?}", config);
        self.config = config;
        Ok(())
    }

    /// 声明插件能力 / Declare plugin capabilities
    ///
    /// AI 插件声明 "ai" 能力，服务器会将 ai.* 事件路由到此插件
    /// AI plugin declares "ai" capability, server routes ai.* events to this plugin
    fn capabilities(&self) -> Vec<String> {
        vec!["ai".into()]
    }

    /// 接收并处理事件 / Receive and handle events
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        let event_type = ctx.event_type();
        debug!("📨 收到 AI 事件 / Received AI event: {}", event_type);

        match event_type {
            "ai.chat" => self.handle_chat(ctx),
            "ai.completion" => self.handle_completion(ctx),
            "ai.summarize" => self.handle_summarize(ctx),
            _ => {
                v::warn!(
                    "⚠️  未知的 AI 事件类型 / Unknown AI event type: {}",
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

impl AIPlugin {
    /// 处理聊天事件 / Handle chat event
    fn handle_chat(&self, ctx: &mut Context) -> Result<()> {
        let user_message = ctx.get_payload_str("message").unwrap_or("");
        let user_id = ctx.get_payload_str("user_id").unwrap_or("anonymous");

        info!(
            "💬 用户 {} 发送消息 / User {} sent message: {}",
            user_id, user_id, user_message
        );

        // 生成 AI 回复 / Generate AI reply
        let ai_reply = self.generate_reply(user_message);

        // 返回响应 / Return response
        ctx.reply(json!({
            "status": "ok",
            "ai_name": self.config.ai_name,
            "reply": ai_reply,
            "timestamp": chrono::Utc::now().timestamp()
        }))?;

        info!("✅ AI 回复已生成 / AI reply generated");
        Ok(())
    }

    /// 处理文本补全事件 / Handle completion event
    fn handle_completion(&self, ctx: &mut Context) -> Result<()> {
        let prompt = ctx.get_payload_str("prompt").unwrap_or("");

        debug!("🔮 生成文本补全 / Generating completion for: {}", prompt);

        let completion = format!("{}...", prompt);

        ctx.reply(json!({
            "status": "ok",
            "completion": completion
        }))?;

        Ok(())
    }

    /// 处理摘要生成事件 / Handle summarize event
    fn handle_summarize(&self, ctx: &mut Context) -> Result<()> {
        let text = ctx.get_payload_str("text").unwrap_or("");

        debug!(
            "📝 生成摘要 / Generating summary for text length: {}",
            text.len()
        );

        let summary = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            text.to_string()
        };

        ctx.reply(json!({
            "status": "ok",
            "summary": summary,
            "original_length": text.len(),
            "summary_length": summary.len()
        }))?;

        Ok(())
    }

    /// 生成 AI 回复 / Generate AI reply
    fn generate_reply(&self, user_message: &str) -> String {
        // 这里是简化的实现，实际应该调用真实的 AI 模型
        // This is a simplified implementation, should call real AI model in production

        if user_message.contains("你好") || user_message.contains("hello") {
            format!(
                "你好！我是{}，很高兴为您服务！有什么我可以帮助您的吗？",
                self.config.ai_name
            )
        } else if user_message.contains("天气") {
            "今天天气不错，适合外出活动。不过具体天气信息建议查看天气预报哦！".to_string()
        } else if user_message.contains("时间") {
            format!(
                "现在的时间是 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            )
        } else if user_message.contains("帮助") || user_message.contains("help") {
            format!(
                "我是{}，我可以帮您：\n1. 回答问题\n2. 提供建议\n3. 进行对话\n请随时告诉我您需要什么帮助！",
                self.config.ai_name
            )
        } else {
            // 默认回复 / Default reply
            let reply = format!(
                "感谢您的消息「{}」。作为{}，我正在学习如何更好地理解和回应。您还有其他问题吗？",
                user_message, self.config.ai_name
            );

            // 限制回复长度 / Limit reply length
            if reply.len() > self.config.max_reply_length {
                format!("{}...", &reply[..self.config.max_reply_length])
            } else {
                reply
            }
        }
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// AI 插件程序入口点 / AI plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 插件元信息 / Plugin metadata
    const PLUGIN_NO: &str = "v.plugin.ai-example";
    const VERSION: &str = "0.1.0";
    const PRIORITY: i32 = 500; // 中等优先级 / Medium priority

    info!("🚀 启动 AI 插件示例 / Starting AI Plugin Example");

    // 启动插件服务器 / Start plugin server
    v::plugin::pdk::run_server::<AIPlugin>(PLUGIN_NO, VERSION, PRIORITY).await
}
