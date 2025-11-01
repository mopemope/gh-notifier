use gh_notifier::config::load_config;
use gh_notifier::{AuthError, GitHubClient, Poller, StateManager, auth_manager::AuthManager};
use tokio::signal;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    // Load config first to get log level
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    // Initialize tracing logger with level from config
    let log_level = &config.log_level;
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

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
            }
            Err(e) => {
                tracing::error!("Failed to get valid token: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        tracing::info!("No existing token found, starting OAuth Device Flow...");
        // Perform the OAuth device flow to get a new token
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                tracing::info!("Authentication successful!");
            }
            Err(e) => {
                tracing::error!("Authentication failed: {}", e);
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
