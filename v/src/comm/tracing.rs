use anyhow::Result;
use chrono::{Datelike, Timelike};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, EnvFilter};

struct LogTimer;

impl fmt::time::FormatTime for LogTimer {
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = chrono::Local::now();
        let cs = now.timestamp_subsec_millis() / 10;
        let s = format!(
            "{:04}-{:02}-{:02}:{:02}:{:02}:{:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            cs
        );
        w.write_str(&s)
    }
}

pub fn init_tracing() -> Result<()> {
    let level: String = crate::comm::config::get_global_config_manager()
        .ok()
        .and_then(|mgr| mgr.get("logging.level").ok())
        .unwrap_or_else(|| "info".to_string());

    let filter = EnvFilter::try_new(format!("{},sqlx=trace", level))
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=trace"));
    LogTracer::init().ok();
    fmt::SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_timer(LogTimer)
        .compact()
        .with_target(false)
        .try_init()
        .ok();
    Ok(())
}
