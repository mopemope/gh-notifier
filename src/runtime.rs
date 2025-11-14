use crate::{Config, GitHubClient, HistoryManager, StateManager, poller::Notifier};
use tokio::sync::broadcast;

/// Execute the main polling loop with shutdown capability
pub async fn run_polling_loop_with_shutdown(
    config: Config,
    mut github_client: GitHubClient,
    mut state_manager: StateManager,
    notifier: Box<dyn Notifier>,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create shutdown channel
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_tx_for_poller = shutdown_tx.clone(); // Clone for the poller task

    // For now, starting the API server in the same context without spawning
    // This is because Actix-web's HttpServer is not Send and can't be spawned in a task
    let api_handle = if config.api_enabled() {
        let history_manager_clone = history_manager.clone();
        let api_port = config.api_port();

        // Note: The API server is started but will block the current thread
        // In a real-world scenario, you might want to configure this differently
        tokio::task::spawn_local(async move {
            if let Err(e) = crate::api::ApiServer::new(history_manager_clone, api_port)
                .start()
                .await
            {
                tracing::error!("API server error: {}", e);
            }
        })
    } else {
        // Create a dummy handle for consistency
        tokio::spawn(async {})
    };

    // Spawn the polling loop as a separate async task
    let poller_task = tokio::spawn(async move {
        // Create a shutdown receiver for the spawned task
        let mut shutdown_rx = shutdown_tx_for_poller.subscribe();
        crate::polling::run_polling_loop_with_shutdown(
            &config,
            &mut github_client,
            &mut state_manager,
            notifier.as_ref(),
            &mut shutdown_rx,
            &history_manager,
        )
        .await
    });

    // Wait for shutdown signal
    super::shutdown::wait_for_shutdown_signal().await;

    tracing::info!("Shutdown signal received, attempting graceful shutdown...");

    // Send shutdown signal to the polling task
    let _ = shutdown_tx.send(());

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

    // Wait for the API server to complete as well
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), api_handle).await;

    Ok(())
}

/// Execute the main polling loop without shutdown capability
pub async fn run_polling_loop(
    config: Config,
    mut github_client: GitHubClient,
    mut state_manager: StateManager,
    notifier: Box<dyn Notifier>,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::polling::run_polling_loop(
        &config,
        &mut github_client,
        &mut state_manager,
        notifier.as_ref(),
        &history_manager,
    )
    .await
}
