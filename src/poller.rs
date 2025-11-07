use crate::{Config, GitHubClient, StateManager};
use notify_rust::Notification;

pub trait Notifier: Send + Sync {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str,
        notification_reason: &str,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct Poller {
    config: Config,
    github_client: GitHubClient,
    state_manager: StateManager,
    notifier: Box<dyn Notifier>,
    history_manager: crate::HistoryManager,
}

impl Poller {
    pub fn new(
        config: Config,
        github_client: GitHubClient,
        state_manager: StateManager,
        notifier: Box<dyn Notifier>,
        history_manager: crate::HistoryManager,
    ) -> Self {
        Poller {
            config,
            github_client,
            state_manager,
            notifier,
            history_manager,
        }
    }

    /// ポーリングを実行する非同期ループ
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crate::polling::run_polling_loop(
            &self.config,
            &mut self.github_client,
            &mut self.state_manager,
            self.notifier.as_ref(),
            &self.history_manager,
        )
        .await
    }

    /// シャットダウンシグナル付きでポーリングを実行する非同期ループ
    pub async fn run_with_shutdown(
        &mut self,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crate::polling::run_polling_loop_with_shutdown(
            &self.config,
            &mut self.github_client,
            &mut self.state_manager,
            self.notifier.as_ref(),
            &mut shutdown_rx,
            &self.history_manager,
        )
        .await
    }
}

pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str, // url を使用する
        notification_reason: &str,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut notification = Notification::new();
        notification
            .summary(title)
            .body(body)
            .icon("dialog-information");

        // 通知の重要度に応じた永続性を制御
        let is_persistent = config.get_persistent_setting_for_notification(notification_reason);
        if is_persistent {
            notification.hint(notify_rust::Hint::Transient(false)); // 永続的（自動消去しない）
        } else {
            notification.hint(notify_rust::Hint::Transient(true)); // 一時的（自動消去する）
        }

        notification.hint(notify_rust::Hint::Custom(
            "default-action".to_string(),
            url.to_string(),
        ));

        notification
            .show()
            .map_err(|e| Box::new(std::io::Error::other(e)))?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct MacNotifier;

#[cfg(target_os = "macos")]
impl Notifier for MacNotifier {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        _url: &str, // url は使用していないので、アンダースコア接頭辞を付ける
        _notification_reason: &str,
        _config: &Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        mac_notification_sys::set_application(&"gh-notifier")?;
        mac_notification_sys::send_notification(&title, &Some(&body), &"", &None)?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsNotifier;

#[cfg(target_os = "windows")]
impl Notifier for WindowsNotifier {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str, // url を使用する
        notification_reason: &str,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use winrt_notification::Toast;

        let mut toast = Toast::new(Toast::POWERSHELL_APP_ID)
            .title(&title)
            .text1(&body)
            .activation_type(winrt_notification::ActivationType::Protocol)
            .launch(&url);

        // 通知の重要度に応じた永続性を制御
        let is_persistent = config.get_persistent_setting_for_notification(notification_reason);
        if is_persistent {
            // 永続的通知（スリープ状態でも表示）
            toast = toast.duration(winrt_notification::Duration::Long);
        } else {
            // 通常通知（短め）
            toast = toast.duration(winrt_notification::Duration::Default);
        }

        toast
            .show()
            .map_err(|e| Box::new(std::io::Error::other(e)))?;

        Ok(())
    }
}

#[allow(dead_code)]
struct DummyNotifier;

impl Notifier for DummyNotifier {
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        url: &str,
        notification_reason: &str,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_persistent = config.get_persistent_setting_for_notification(notification_reason);
        let persistence_status = if is_persistent {
            "persistent"
        } else {
            "transient"
        };
        println!(
            "Notification: {} - {} (URL: {}, Reason: {}, Status: {})",
            title, body, url, notification_reason, persistence_status
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthManager, Notification, NotificationRepository, NotificationSubject};
    use tempfile::tempdir;

    struct DummyNotifier;

    impl Notifier for DummyNotifier {
        fn send_notification(
            &self,
            title: &str,
            body: &str,
            url: &str,
            _notification_reason: &str,
            _config: &Config,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            println!("Notification: {} - {} (URL: {})", title, body, url);
            Ok(())
        }
    }

    #[tokio::test]
    #[ignore] // 認証トークンがないとテストできないため
    async fn test_poller_creation() {
        use tempfile::tempdir;
        let config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let github_client = GitHubClient::new(auth_manager).unwrap();
        let state_manager = StateManager::new().unwrap();
        let notifier: Box<dyn Notifier> = Box::new(DummyNotifier);

        // テスト用に一時的なデータベースを作成
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let history_manager = crate::HistoryManager::new(&db_path).unwrap();

        let poller = Poller::new(
            config,
            github_client,
            state_manager,
            notifier,
            history_manager,
        );
        // 構造体が作成できることを確認
        assert_eq!(poller.config.poll_interval_sec, 30);
    }

    #[test]
    fn test_filter_new_notifications() {
        use crate::config::NotificationFilter;
        let mut config = Config::default();
        // Reset notification filters to allow the test to work as expected
        config.notification_filters = NotificationFilter::default();
        let auth_manager = AuthManager::new().unwrap();
        let github_client = GitHubClient::new(auth_manager).unwrap();
        let state_manager = StateManager::new().unwrap();
        let notifier: Box<dyn Notifier> = Box::new(DummyNotifier);
        // テスト用に一時的なデータベースを作成
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let history_manager = crate::HistoryManager::new(&db_path).unwrap();

        let _poller = Poller::new(
            config.clone(),
            github_client,
            state_manager,
            notifier,
            history_manager,
        );

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
            crate::polling::filter::filter_new_notifications(
                &notifications,
                &state_manager,
                &config,
            );

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }

    #[test]
    fn test_desktop_notifier_send_notification() {
        let notifier = DesktopNotifier;
        let config = crate::Config::default();
        // テストでは通知を表示しないが、エラーが発生しないことを確認
        let result = notifier.send_notification(
            "Test Title",
            "Test Body",
            "https://example.com",
            "mention",
            &config,
        );
        assert!(result.is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_notifier_send_notification() {
        let notifier = WindowsNotifier;
        let config = crate::Config::default();
        // テストでは通知を表示しないが、エラーが発生しないことを確認
        let result = notifier.send_notification(
            "Test Title",
            "Test Body",
            "https://example.com",
            "mention",
            &config,
        );
        assert!(result.is_ok());
    }
}
