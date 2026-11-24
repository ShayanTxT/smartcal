use crate::types::{LogEntry, LogLevel};
use chrono::Utc;
use directories::ProjectDirs;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

const MAX_MEMORY_LOGS: usize = 600;

static MEMORY_LOGS: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();
static FILE_GUARD: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();

fn log_buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    MEMORY_LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_MEMORY_LOGS)))
}

/// Returns the standard log directory (`~/.local/share/smartcal/logs` on Linux).
pub fn log_dir() -> PathBuf {
    if let Some(proj) = ProjectDirs::from("org", "smartcal", "smartcal") {
        let dir = proj.data_local_dir().join("logs");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    let fallback = PathBuf::from("logs");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// Scrubs potentially sensitive environment/user strings before logging.
pub fn sanitize_log_text(input: &str) -> String {
    let mut out = input.to_string();
    for key in ["PASSWORD=", "TOKEN=", "SECRET=", "SUDO_PASSWORD=", "KEY="] {
        if let Some(idx) = out.to_uppercase().find(key) {
            out.truncate(idx + key.len());
            out.push_str("[REDACTED]");
        }
    }
    out
}

pub fn record_log(level: LogLevel, target: &str, message: impl AsRef<str>) {
    let clean = sanitize_log_text(message.as_ref());
    match level {
        LogLevel::Debug => tracing::debug!(target: "smartcal", "{}: {}", target, clean),
        LogLevel::Info => tracing::info!(target: "smartcal", "{}: {}", target, clean),
        LogLevel::Warning => tracing::warn!(target: "smartcal", "{}: {}", target, clean),
        LogLevel::Error => tracing::error!(target: "smartcal", "{}: {}", target, clean),
    }
    if let Ok(mut buf) = log_buffer().lock() {
        if buf.len() >= MAX_MEMORY_LOGS {
            buf.pop_front();
        }
        buf.push_back(LogEntry {
            timestamp: Utc::now(),
            level,
            target: target.to_string(),
            message: clean,
        });
    }
}

pub fn recent_logs(min_level: Option<LogLevel>) -> Vec<LogEntry> {
    let Ok(buf) = log_buffer().lock() else {
        return Vec::new();
    };
    buf.iter()
        .filter(|e| min_level.is_none_or(|m| e.level >= m))
        .cloned()
        .collect()
}

pub fn clear_memory_logs() {
    if let Ok(mut buf) = log_buffer().lock() {
        buf.clear();
    }
}

pub fn init_logging(level: LogLevel) {
    let dir = log_dir();
    let filter_str = match level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warning => "warn",
        LogLevel::Error => "error",
    };

    let file_appender = tracing_appender::rolling::daily(&dir, "smartcal.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let guard_cell = FILE_GUARD.get_or_init(|| Mutex::new(None));
    if let Ok(mut slot) = guard_cell.lock() {
        *slot = Some(guard);
    }

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("smartcal={filter_str},{filter_str}")));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();

    record_log(
        LogLevel::Info,
        "logging",
        format!(
            "Initialized SmartCal logger at level {level} in {}",
            dir.display()
        ),
    );
}
