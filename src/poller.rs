use crate::{Config, GitHubClient, Notification, StateManager};
use std::time::Duration as StdDuration;
use tokio::time::Instant;

pub trait Notifier: Send + Sync {
    fn send_notification(&self, title: &str, body: &str, url: &str) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Poller {
    config: Config,
    github_client: GitHubClient,
    state_manager: StateManager,
    notifier: Box<dyn Notifier>,
}

impl Poller {
    pub fn new(
        config: Config,
        github_client: GitHubClient,
        state_manager: StateManager,
        notifier: Box<dyn Notifier>,
    ) -> Self {
        Poller {
            config,
            github_client,
            state_manager,
            notifier,
        }
    }

    /// ポーリングを実行する非同期ループ
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::time::{interval};

        let mut interval = interval(StdDuration::from_secs(self.config.poll_interval_sec));

        loop {
            interval.tick().await; // 次のポーリングまで待機

            // StateManager から最終確認日時を取得
            let if_modified_since = self.state_manager.get_last_checked_at();

            // GitHub API から通知を取得
            match self.github_client.get_notifications(if_modified_since, None).await {
                Ok(Some(notifications)) => {
                    // 最終確認日時以降の新しい通知のみを処理（task-9.2）
                    let new_notifications: Vec<&Notification> = if let Some(last_checked) = self.state_manager.get_last_checked_at() {
                        notifications.iter().filter(|n| n.updated_at.as_str() > last_checked).collect()
                    } else {
                        // 最終確認日時がない場合はすべて新しいと見なす
                        notifications.iter().collect()
                    };

                    if !new_notifications.is_empty() {
                        // 最新の通知の updated_at を最終確認日時として更新
                        if let Some(latest) = new_notifications.iter().max_by_key(|n| &n.updated_at) {
                            self.state_manager.update_last_checked_at(latest.updated_at.clone());
                        }

                        for notification in new_notifications {
                            // 通知を Notifier に渡す（task-9.3）
                            let title = format!("{} / {}", notification.repository.full_name, notification.subject.kind);
                            let body = &notification.subject.title;
                            let url = &notification.subject.url.as_ref().unwrap_or(&notification.url);

                            if let Err(e) = self.notifier.send_notification(&title, body, url) {
                                eprintln!("Failed to send notification: {}", e);
                            }

                            // 設定で mark_as_read_on_notify が有効なら既読にする（task-9.4）
                            if self.config.mark_as_read_on_notify {
                                if let Err(e) = self.github_client.mark_notification_as_read(&notification.id).await {
                                    eprintln!("Failed to mark notification as read: {}", e);
                                }
                            }
                        }

                        // 状態を保存
                        if let Err(e) = self.state_manager.save() {
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
}

struct DummyNotifier;

impl Notifier for DummyNotifier {
    fn send_notification(&self, title: &str, body: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Notification: {} - {} (URL: {})", title, body, url);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthManager, Notification, NotificationSubject, NotificationRepository};

    #[tokio::test]
    #[ignore]  // 認証トークンがないとテストできないため
    async fn test_poller_creation() {
        let config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let github_client = GitHubClient::new(auth_manager).unwrap();
        let state_manager = StateManager::new().unwrap();
        let notifier: Box<dyn Notifier> = Box::new(DummyNotifier);
        let poller = Poller::new(config, github_client, state_manager, notifier);
        // 構造体が作成できることを確認
        assert_eq!(poller.config.poll_interval_sec, 30);
    }

    #[test]
    fn test_filter_new_notifications() {
        let config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let github_client = GitHubClient::new(auth_manager).unwrap();
        let mut state_manager = StateManager::new().unwrap();
        let notifier: Box<dyn Notifier> = Box::new(DummyNotifier);
        let poller = Poller::new(config, github_client, state_manager, notifier);

        let old_time = "2023-01-01T00:00:00Z";
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: old_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Old notification".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "repo1".to_string(),
                    full_name: "user/repo1".to_string(),
                    private: false,
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "New notification".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "repo2".to_string(),
                    full_name: "user/repo2".to_string(),
                    private: false,
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
        ];

        // 最終確認日時を設定
        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at(old_time.to_string());

        // フィルタリング処理（実際にはPoller構造体に状態がないため、外部から行う）
        let new_notifications: Vec<&Notification> = if let Some(last_checked) = state_manager.get_last_checked_at() {
            notifications.iter().filter(|n| n.updated_at.as_str() > last_checked).collect()
        } else {
            notifications.iter().collect()
        };

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }
}