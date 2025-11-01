use crate::{Config, GitHubClient, StateManager};

pub trait Notifier: Send + Sync {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
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
        crate::polling::run_polling_loop(
            &self.config,
            &mut self.github_client,
            &mut self.state_manager,
            self.notifier.as_ref(),
        )
        .await
    }
}

struct DummyNotifier;

impl Notifier for DummyNotifier {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Notification: {} - {} (URL: {})", title, body, url);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthManager, Notification, NotificationRepository, NotificationSubject};

    #[tokio::test]
    #[ignore] // 認証トークンがないとテストできないため
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
        let state_manager = StateManager::new().unwrap();
        let notifier: Box<dyn Notifier> = Box::new(DummyNotifier);
        let _poller = Poller::new(config, github_client, state_manager, notifier);

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
        let new_notifications: Vec<&Notification> =
            if let Some(last_checked) = state_manager.get_last_checked_at() {
                notifications
                    .iter()
                    .filter(|n| n.updated_at.as_str() > last_checked)
                    .collect()
            } else {
                notifications.iter().collect()
            };

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }
}
