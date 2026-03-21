use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with specified log level
pub fn init(log_level: &str) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Try to add file logging if log directory exists
    if let Some(data_dir) = dirs::data_local_dir() {
        let log_dir = data_dir.join("turbo-search").join("logs");
        if std::fs::create_dir_all(&log_dir).is_ok() {
            let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "app.log");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(console_layer)
                .with(file_layer)
                .init();

            Box::leak(Box::new(_guard));
            return Ok(());
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .init();

    Ok(())
}

/// Initialize logging with custom log directory (legacy function)
pub fn init_logging(log_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if let Some(log_dir) = log_dir {
        std::fs::create_dir_all(&log_dir)?;
        let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "app.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();

        Box::leak(Box::new(_guard));
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    Ok(())
}
