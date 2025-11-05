use crate::{Config, GitHubClient, Poller, StateManager, poller::Notifier};
use tokio::sync::broadcast;

/// Execute the main polling loop with shutdown capability
pub async fn run_polling_loop_with_shutdown(
    config: Config,
    github_client: GitHubClient,
    state_manager: StateManager,
    notifier: Box<dyn Notifier>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create shutdown channel
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_tx_for_poller = shutdown_tx.clone(); // Clone for the poller task

    // Create poller with initialized components
    let mut poller = Poller::new(config, github_client, state_manager, notifier);

    // Spawn the polling loop as a separate async task
    let poller_task = tokio::spawn(async move {
        // Create a shutdown receiver for the spawned task
        let poller_shutdown_rx = shutdown_tx_for_poller.subscribe();
        poller.run_with_shutdown(poller_shutdown_rx).await
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

    Ok(())
}
