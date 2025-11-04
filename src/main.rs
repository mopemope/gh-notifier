use gh_notifier::config::load_config;
use gh_notifier::{AuthError, GitHubClient, Poller, StateManager, auth_manager::AuthManager};
use tokio::signal;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    // Load config first to get log level
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    // Set up file logging first so we can log setup process
    let (file_writer, guard) = create_file_logger(&config.log_file_path);

    // Initialize tracing logger with level from config
    let log_level = &config.log_level;
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(file_writer) // Log to file
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    // Keep the guard alive to ensure log messages are flushed
    let _guard = guard;

    tracing::info!("GitHub Notifier starting...");

    let mut auth_manager = AuthManager::new()?;

    // Try to load existing token from keychain and ensure it's valid
    if let Ok(Some(token_info)) = auth_manager.load_token_from_keychain() {
        tracing::info!("Found existing token in keychain");
        auth_manager.token_info = Some(token_info);

        // Use the comprehensive token management - handles validation, refresh, and re-auth
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                tracing::info!("Token is valid and ready for use");
                println!("Token is valid and ready for use. The application is now running in the background.");
                println!("It will continuously check GitHub for new notifications.");
                println!("Press Ctrl+C to stop the application.");
            }
            Err(e) => {
                tracing::error!("Failed to get valid token: {}", e);
                eprintln!(
                    "Failed to validate existing token. This may be due to an invalid or unconfigured GitHub OAuth client ID."
                );
                eprintln!(
                    "The existing token may be invalid, or the client ID may need to be updated."
                );
                eprintln!(
                    "Please check your configuration and ensure you have a valid client_id set."
                );
                std::process::exit(1);
            }
        }
    } else {
        tracing::info!("No existing token found, starting OAuth Device Flow...");
        // Perform the OAuth device flow to get a new token
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                tracing::info!("Authentication successful!");
                println!("Authentication successful! The application is now running in the background.");
                println!("It will continuously check GitHub for new notifications.");
                println!("Press Ctrl+C to stop the application.");
            }
            Err(e) => {
                tracing::error!("Authentication failed: {}", e);
                eprintln!(
                    "Authentication failed. This may be due to an invalid or unconfigured GitHub OAuth client ID."
                );
                eprintln!(
                    "Please ensure you have a valid client_id configured in your config file."
                );
                eprintln!(
                    "To configure your own client ID, register a GitHub OAuth App and set the 'client_id' in the config file."
                );
                std::process::exit(1);
            }
        }
    }

    // GitHubクライアント、ステートマネージャー、通知マネージャーを初期化
    let github_client = GitHubClient::new(auth_manager).unwrap();
    let state_manager = StateManager::new().unwrap();
    let notifier = Box::new(gh_notifier::DesktopNotifier);

    // Create shutdown channel
    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

    // Pollerを初期化
    let mut poller = Poller::new(config, github_client, state_manager, notifier);

    tracing::info!("GitHub Notifier running with authenticated access");

    // Clone the sender to use in the main task for sending shutdown signals
    let shutdown_tx_clone = shutdown_tx.clone();

    // Spawn the polling loop as a separate async task
    let poller_task = tokio::spawn(async move {
        // Create a shutdown receiver for the spawned task
        let poller_shutdown_rx = shutdown_tx.subscribe();
        poller.run_with_shutdown(poller_shutdown_rx).await
    });

    // Wait for shutdown signal
    shutdown_signal().await;
    tracing::info!("Shutdown signal received, attempting graceful shutdown...");

    // Send shutdown signal to the polling task
    let _ = shutdown_tx_clone.send(());

    // Wait for the polling task to complete gracefully
    match tokio::time::timeout(std::time::Duration::from_secs(5), poller_task).await {
        Ok(task_result) => match task_result {
            Ok(task_result_inner) => match task_result_inner {
                Ok(_) => tracing::info!("Polling task exited normally after shutdown signal"),
                Err(e) => tracing::error!("Polling task error after shutdown: {}", e),
            },
            Err(e) => tracing::error!("Polling task join error after shutdown: {}", e),
        },
        Err(_) => {
            tracing::error!("Polling task did not complete within timeout, forcing shutdown");
        }
    }

    tracing::info!("GitHub Notifier shutdown complete");
    Ok(())
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

// Wait for a shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
