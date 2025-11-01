use crate::poller::Notifier;
use crate::{Config, GitHubClient, StateManager};
use std::time::Duration as StdDuration;
use tokio::time::interval;

pub async fn run_polling_loop(
    config: &Config,
    github_client: &mut GitHubClient,
    state_manager: &mut StateManager,
    notifier: &dyn Notifier,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = interval(StdDuration::from_secs(config.poll_interval_sec));

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
                let new_notifications =
                    crate::polling::filter::filter_new_notifications(&notifications, state_manager);

                if !new_notifications.is_empty() {
                    // 最新の通知の updated_at を最終確認日時として更新
                    if let Some(latest) = new_notifications.iter().max_by_key(|n| &n.updated_at) {
                        state_manager.update_last_checked_at(latest.updated_at.clone());
                    }

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

                    // 状態を保存
                    if let Err(e) = state_manager.save() {
                        eprintln!("Failed to save state: {}", e);
                    }
                }
            }
            Ok(None) => {
                // 304 Not Modified
                println!("No new notifications (304 Not Modified)");
            }
            Err(e) => {
                eprintln!("Error fetching notifications: {}", e);
            }
        }
    }
}
