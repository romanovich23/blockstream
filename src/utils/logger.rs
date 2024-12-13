use log::LevelFilter;
use std::env;
use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    #[error("Invalid log level: {0}")]
    InvalidLogLevel(String),
}

pub fn initialize_logger() -> Result<(), LoggerError> {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let log_level = match log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => return Err(LoggerError::InvalidLogLevel(log_level)),
    };

    let log_format = |buf: &mut env_logger::fmt::Formatter, record: &log::Record| {
        writeln!(
            buf,
            "{} [{}] - {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args(),
        )
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format(log_format)
        .init();

    Ok(())
}
