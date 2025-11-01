use crate::poller::Notifier;
use crate::{Config, GitHubClient, Notification, StateManager};
use std::collections::VecDeque;
use std::time::Duration as StdDuration;
use tokio::sync::broadcast;
use tokio::time::{Instant, interval};

pub async fn run_polling_loop(
    config: &Config,
    github_client: &mut GitHubClient,
    state_manager: &mut StateManager,
    notifier: &dyn Notifier,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut interval = interval(StdDuration::from_secs(config.poll_interval_sec));
    // バッチ処理用のバッファとタイマー
    let batch_size = config.notification_batch_config.batch_size;
    let batch_interval =
        StdDuration::from_secs(config.notification_batch_config.batch_interval_sec);
    let error_handling = &config.polling_error_handling_config;
    let mut batch_buffer: VecDeque<Notification> = VecDeque::new();
    let mut last_batch_time = Instant::now();

    loop {
        interval.tick().await; // 次のポーリングまで待機

        // StateManager から最終確認日時を取得
        let if_modified_since = state_manager.get_last_checked_at();

        // GitHub API から通知を取得
        match github_client
            .get_notifications(if_modified_since, None)
            .await
        {
            Ok(Some(notifications)) => {
                // 最終確認日時以降の新しい通知のみを処理
                let new_notifications = crate::polling::filter::filter_new_notifications(
                    &notifications,
                    state_manager,
                    config,
                );

                if !new_notifications.is_empty() {
                    // 最新の通知の updated_at を最終確認日時として更新
                    if let Some(latest) = new_notifications.iter().max_by_key(|n| &n.updated_at) {
                        state_manager.update_last_checked_at(latest.updated_at.clone());
                    }

                    // バッチ処理が有効な場合はバッファに追加
                    if batch_size > 0 {
                        for notification in new_notifications {
                            batch_buffer.push_back(notification.clone());
                        }

                        // バッチサイズに達したか、時間経過時に処理
                        if batch_buffer.len() >= batch_size
                            || last_batch_time.elapsed() >= batch_interval
                        {
                            if let Err(e) = process_batch(
                                &batch_buffer,
                                notifier,
                                github_client,
                                config,
                                error_handling,
                            )
                            .await
                            {
                                eprintln!("Failed to process batch: {}", e);
                            }
                            batch_buffer.clear();
                            last_batch_time = Instant::now();
                        }
                    } else {
                        // バッチ処理が無効な場合は1つずつ処理
                        for notification in new_notifications {
                            // 通知を Notifier に渡す
                            if let Err(e) = crate::polling::handler::handle_notification(
                                notification,
                                notifier,
                                github_client,
                                config.mark_as_read_on_notify,
                            )
                            .await
                            {
                                eprintln!("Failed to handle notification: {}", e);
                            }
                        }
                    }

                    // 状態を保存
                    if let Err(e) = state_manager.save() {
                        eprintln!("Failed to save state: {}", e);
                    }
                }
            }
            Ok(None) => {
                // 304 Not Modified
                tracing::debug!("No new notifications (304 Not Modified)");
            }
            Err(e) => {
                tracing::error!("Error fetching notifications: {}", e);
            }
        }
    }
}

/// シャットダウンシグナル付きでポーリングを実行する非同期ループ
pub async fn run_polling_loop_with_shutdown(
    config: &Config,
    github_client: &mut GitHubClient,
    state_manager: &mut StateManager,
    notifier: &dyn Notifier,
    shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut interval = interval(StdDuration::from_secs(config.poll_interval_sec));
    // バッチ処理用のバッファとタイマー
    let batch_size = config.notification_batch_config.batch_size;
    let batch_interval =
        StdDuration::from_secs(config.notification_batch_config.batch_interval_sec);
    let error_handling = &config.polling_error_handling_config;
    let mut batch_buffer: VecDeque<Notification> = VecDeque::new();
    let mut last_batch_time = Instant::now();

    loop {
        // シャットダウンシグナルを待機しつつ、ポーリング間隔を待機
        tokio::select! {
            _ = interval.tick() => {
                // StateManager から最終確認日時を取得
                let if_modified_since = state_manager.get_last_checked_at();

                // GitHub API から通知を取得
                match github_client
                    .get_notifications(if_modified_since, None)
                    .await
                {
                    Ok(Some(notifications)) => {
                        // 最終確認日時以降の新しい通知のみを処理
                        let new_notifications = crate::polling::filter::filter_new_notifications(
                            &notifications,
                            state_manager,
                            config,
                        );

                        if !new_notifications.is_empty() {
                            // 最新の通知の updated_at を最終確認日時として更新
                            if let Some(latest) = new_notifications.iter().max_by_key(|n| &n.updated_at) {
                                state_manager.update_last_checked_at(latest.updated_at.clone());
                            }

                            // バッチ処理が有効な場合はバッファに追加
                            if batch_size > 0 {
                                for notification in new_notifications {
                                    batch_buffer.push_back(notification.clone());
                                }

                                // バッチサイズに達したか、時間経過時に処理
                                if batch_buffer.len() >= batch_size
                                    || last_batch_time.elapsed() >= batch_interval
                                {
                                    if let Err(e) = process_batch(
                                        &batch_buffer,
                                        notifier,
                                        github_client,
                                        config,
                                        error_handling,
                                    )
                                    .await
                                    {
                                        tracing::error!("Failed to process batch: {}", e);
                                    }
                                    batch_buffer.clear();
                                    last_batch_time = Instant::now();
                                }
                            } else {
                                // バッチ処理が無効な場合は1つずつ処理
                                for notification in new_notifications {
                                    // 通知を Notifier に渡す
                                    if let Err(e) = crate::polling::handler::handle_notification(
                                        notification,
                                        notifier,
                                        github_client,
                                        config.mark_as_read_on_notify,
                                    )
                                    .await
                                    {
                                        tracing::error!("Failed to handle notification: {}", e);
                                    }
                                }
                            }

                            // 状態を保存
                            if let Err(e) = state_manager.save() {
                                tracing::error!("Failed to save state: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // 304 Not Modified
                        tracing::debug!("No new notifications (304 Not Modified)");
                    }
                    Err(e) => {
                        tracing::error!("Error fetching notifications: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Shutdown signal received, saving state and exiting...");
                // 終了前に状態を保存
                if let Err(e) = state_manager.save() {
                    tracing::error!("Failed to save state on shutdown: {}", e);
                }
                tracing::info!("State saved, exiting polling loop");
                return Ok(());
            }
        }
    }
}

/// バッチ処理を実行
async fn process_batch(
    batch: &VecDeque<Notification>,
    notifier: &dyn Notifier,
    github_client: &mut GitHubClient,
    config: &Config,
    _error_handling: &crate::config::PollingErrorHandlingConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for notification in batch {
        // 通知を Notifier に渡す
        if let Err(e) = crate::polling::handler::handle_notification(
            notification,
            notifier,
            github_client,
            config.mark_as_read_on_notify,
        )
        .await
        {
            tracing::error!("Failed to handle notification: {}", e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthManager, Config, GitHubClient, StateManager};
    use tokio::sync::broadcast;

    struct MockNotifier;

    impl crate::poller::Notifier for MockNotifier {
        fn send_notification(
            &self,
            _title: &str,
            _body: &str,
            _url: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_run_polling_loop_with_shutdown_immediate() {
        let config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let mut github_client = GitHubClient::new(auth_manager).unwrap();
        let mut state_manager = StateManager::new().unwrap();
        let notifier = MockNotifier;

        // Create a shutdown sender and immediately send a shutdown signal
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        // Send shutdown signal
        let _ = shutdown_tx.send(());

        // Run polling loop with shutdown signal already sent
        let result = run_polling_loop_with_shutdown(
            &config,
            &mut github_client,
            &mut state_manager,
            &notifier,
            &mut shutdown_rx,
        )
        .await;

        // It should exit gracefully without error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_polling_loop_with_shutdown_after_creation() {
        let config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let mut github_client = GitHubClient::new(auth_manager).unwrap();
        let mut state_manager = StateManager::new().unwrap();
        let notifier = MockNotifier;

        // Create a shutdown sender
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        // Create a second sender to send shutdown signal later
        let shutdown_tx2 = shutdown_tx.clone();

        // Spawn the function and then send shutdown
        let handle = tokio::spawn(async move {
            run_polling_loop_with_shutdown(
                &config,
                &mut github_client,
                &mut state_manager,
                &notifier,
                &mut shutdown_rx,
            )
            .await
        });

        // Send shutdown after a short delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = shutdown_tx2.send(());

        // Wait for the task to complete
        let result = handle.await.unwrap();

        assert!(result.is_ok());
    }
}
