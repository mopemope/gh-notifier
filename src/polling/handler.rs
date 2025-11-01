use crate::poller::Notifier;
use crate::{GitHubClient, Notification};

/// 通知を Notifier に渡して表示し、必要に応じて既読にする
pub async fn handle_notification(
    notification: &Notification,
    notifier: &dyn Notifier,
    github_client: &mut GitHubClient,
    mark_as_read_on_notify: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let title = format!(
        "{} / {}",
        notification.repository.full_name, notification.subject.kind
    );
    let body = &notification.subject.title;
    let url = &notification
        .subject
        .url
        .as_ref()
        .unwrap_or(&notification.url);

    notifier.send_notification(&title, body, url)?;

    if mark_as_read_on_notify {
        github_client
            .mark_notification_as_read(&notification.id)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthManager, Config, Notification, NotificationRepository, NotificationSubject};

    struct DummyNotifier;

    impl crate::poller::Notifier for DummyNotifier {
        fn send_notification(
            &self,
            _title: &str,
            _body: &str,
            _url: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    #[tokio::test]
    #[ignore] // 認証トークンがないとテストできないため
    async fn test_handle_notification() {
        let _config = Config::default();
        let auth_manager = AuthManager::new().unwrap();
        let mut github_client = GitHubClient::new(auth_manager).unwrap();
        let notification = Notification {
            id: "1".to_string(),
            unread: true,
            reason: "mention".to_string(),
            updated_at: "2023-01-02T00:00:00Z".to_string(),
            last_read_at: None,
            subject: NotificationSubject {
                title: "Test notification".to_string(),
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
        };
        let notifier: &dyn crate::poller::Notifier = &DummyNotifier;

        let result = handle_notification(&notification, notifier, &mut github_client, false).await;
        assert!(result.is_ok());
    }
}
