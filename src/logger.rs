use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::config::Config;

/// Set up application logging based on configuration
pub fn setup_logging(config: &Config) -> tracing_appender::non_blocking::WorkerGuard {
    // Initialize tracing logger with level from config
    let log_level = &config.log_level();
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    if config.log_file_path().is_none() {
        // When no file path is specified, log only to stdout/stderr
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr) // Log to stderr by default
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global tracing subscriber");

        // Return a dummy guard - we still need to return the same type
        // Create a non-blocking writer that won't be used
        let (_dummy_writer, guard) = tracing_appender::non_blocking(
            tracing_appender::rolling::never(std::env::temp_dir(), "unused.log"),
        );

        guard
    } else {
        // Use file logging when a file path is specified
        let (file_writer, guard) = create_file_logger(config.log_file_path());

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .with_writer(file_writer) // Log to file
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global tracing subscriber");

        guard
    }
}

// Create file logger
fn create_file_logger(
    log_file_path: &Option<String>,
) -> (NonBlocking, tracing_appender::non_blocking::WorkerGuard) {
    // Determine the log file path and whether to use rolling or simple appender
    if let Some(path) = log_file_path {
        let log_path = std::path::PathBuf::from(path);
        let log_dir = log_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                dirs::data_local_dir()
                    .unwrap_or_else(|| {
                        std::env::current_dir().expect("Current directory not accessible")
                    })
                    .join("gh-notifier")
                    .join("logs")
            });

        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

        let log_file_name = log_path
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("gh-notifier.log"));

        // For custom paths, we'll use a simple file appender (no rotation) since rotation with custom paths
        // can be more complex - we'll use a non-rotating file appender
        let file_appender = tracing_appender::rolling::never(&log_dir, log_file_name);
        tracing_appender::non_blocking(file_appender)
    } else {
        // Use default location with daily rotation
        let default_log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"))
            .join("gh-notifier")
            .join("logs");

        // Create the default log directory if it doesn't exist
        std::fs::create_dir_all(&default_log_dir).expect("Failed to create log directory");

        let file_appender = RollingFileAppender::new(
            tracing_appender::rolling::Rotation::DAILY,
            default_log_dir,
            "gh-notifier.log",
        );

        tracing_appender::non_blocking(file_appender)
    }
}
