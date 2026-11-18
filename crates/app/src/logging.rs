use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn log_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join("Library/Logs/PimSiang"))
}

pub fn init(dir: &std::path::Path) -> Result<WorkerGuard> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let appender = tracing_appender::rolling::daily(dir, "pimsiang.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = Targets::new()
        .with_default(LevelFilter::WARN)
        .with_targets([
            ("pimsiang", LevelFilter::INFO),
            ("app", LevelFilter::INFO),
            ("audio", LevelFilter::INFO),
            ("platform_macos", LevelFilter::INFO),
        ]);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(writer).with_ansi(false))
        .with(filter)
        .init();
    Ok(guard)
}
