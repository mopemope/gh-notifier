use crate::poller::Notifier;
use crate::{Config, GitHubClient, HistoryManager, Notification};
use chrono::{DateTime, Local, Utc};

/// 通知を Notifier に渡して表示し、必要に応じて既読にする
pub async fn handle_notification(
    notification: &Notification,
    notifier: &dyn Notifier,
    github_client: &mut GitHubClient,
    config: &Config,
    history_manager: &HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!(
        "Handling notification ID: {}, Title: '{}', Reason: '{}', Type: '{}', Repo: '{}', URL: '{}'",
        notification.id,
        notification.subject.title,
        notification.reason,
        notification.subject.kind,
        notification.repository.full_name,
        notification.url
    );

    // Additional logging for Pull Request notifications to help debug why they might not appear
    if notification.subject.kind == "PullRequest" {
        tracing::info!(
            "Processing PR notification - ID: {}, Title: '{}', Reason: '{}', Repo: '{}', URL: {}",
            notification.id,
            notification.subject.title,
            notification.reason,
            notification.repository.full_name,
            notification.url
        );
    }

    // 通知が既に保存されていて既読状態になっているか確認
    let is_already_read = history_manager
        .is_notification_read(&notification.id)
        .unwrap_or(false);

    // 通知が既読なら、表示しない設定があれば通知をスキップする
    // (ただし、通知を表示するかどうかは設定で制御可能)
    if is_already_read && !config.show_read_notifications {
        tracing::debug!(
            "Notification {} is already read, skipping notification",
            notification.id
        );
        // それでも履歴には保存しておく
        if let Err(e) = history_manager.save_notification(notification) {
            tracing::error!("Failed to save notification to history: {}", e);
        }
        return Ok(());
    }

    // Create a more specific title with reason information
    let reason_text = get_reason_display_text(&notification.reason);
    let repo_name = if notification.repository.private {
        format!("🔒 {}", notification.repository.full_name)
    } else {
        notification.repository.full_name.clone()
    };

    // 表示用タイトルに既読状態を反映
    let title = if is_already_read {
        format!("[READ] {} - {}", repo_name, reason_text)
    } else {
        format!("{} - {}", repo_name, reason_text)
    };

    // Create a more informative body with additional context
    let time_ago_text = format_time_ago(&notification.updated_at);
    let url = &notification
        .subject
        .url
        .as_ref()
        .unwrap_or(&notification.url);
    let body = format!(
        "{}\n\n{} | {} | Updated: {}\nURL: {}",
        notification.subject.title,
        notification.repository.name,
        format_subject_kind(&notification.subject.kind),
        time_ago_text,
        url
    );

    tracing::debug!("Sending notification: Title='{}', Body='{}'", title, body);
    notifier.send_notification(&title, &body, url, &notification.reason, config)?;

    // 通知を履歴に保存（重複チェック付き）
    if let Err(e) = history_manager.save_notification(notification) {
        tracing::error!("Failed to save notification to history: {}", e);
    }

    if config.mark_as_read_on_notify {
        github_client
            .mark_notification_as_read(&notification.id)
            .await?;
    }

    Ok(())
}

/// Get a user-friendly display text for notification reasons
fn get_reason_display_text(reason: &str) -> String {
    match reason {
        "assign" => "_assigned to you_".to_string(),
        "author" => "authored by you".to_string(),
        "comment" => "commented on".to_string(),
        "invitation" => "invited you".to_string(),
        "manual" => "mentioned you".to_string(),
        "mention" => "mentioned you".to_string(),
        "review_requested" => "_Review Requested_".to_string(),
        "security_alert" => "_Security Alert_".to_string(),
        "state_change" => "state changed".to_string(),
        "subscribed" => "subscribed".to_string(),
        "team_mention" => "team mentioned".to_string(),
        _ => reason.to_string(),
    }
}

/// Format the subject kind for better readability
fn format_subject_kind(kind: &str) -> String {
    match kind {
        "Issue" => "Issue".to_string(),
        "PullRequest" => "Pull Request".to_string(),
        "Commit" => "Commit".to_string(),
        "Release" => "Release".to_string(),
        _ => kind.to_string(),
    }
}

/// Format time to show how long ago the notification was updated
fn format_time_ago(updated_at: &str) -> String {
    // Parse the ISO 8601 timestamp from GitHub API
    match DateTime::parse_from_rfc3339(updated_at) {
        Ok(updated_time) => {
            let utc_time: DateTime<Utc> =
                DateTime::from_naive_utc_and_offset(updated_time.naive_utc(), Utc);

            let local_time: DateTime<Local> = DateTime::<Local>::from(utc_time);

            let now = Local::now();
            let duration = now.signed_duration_since(local_time);

            // Format based on duration
            if duration.num_seconds() < 60 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else if duration.num_days() < 7 {
                format!("{}d ago", duration.num_days())
            } else {
                // Show date if older than a week
                local_time.format("%b %d").to_string()
            }
        }
        Err(_) => updated_at.to_string(), // Fallback to original string if parsing fails
    }
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
            _notification_reason: &str,
            _config: &Config,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    #[ignore] // 認証トークンがないとテストできないため
    async fn test_handle_notification() {
        use tempfile::tempdir;
        let config = Config::default();
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

        // テスト用に一時的なデータベースを作成
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let history_manager = crate::HistoryManager::new(&db_path).unwrap();

        let result = handle_notification(
            &notification,
            notifier,
            &mut github_client,
            &config,
            &history_manager,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_notification_with_importance_logic() {
        use std::sync::{Arc, Mutex};
        use tempfile::tempdir;

        // Create a mock notifier that captures the notification reason passed to it
        struct MockNotifier {
            captured_reasons: Arc<Mutex<Vec<String>>>,
        }

        impl crate::poller::Notifier for MockNotifier {
            fn send_notification(
                &self,
                _title: &str,
                _body: &str,
                _url: &str,
                notification_reason: &str,
                _config: &Config,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                self.captured_reasons
                    .lock()
                    .unwrap()
                    .push(notification_reason.to_string());
                Ok(())
            }
        }

        let captured_reasons = Arc::new(Mutex::new(Vec::new()));
        let mock_notifier = MockNotifier {
            captured_reasons: captured_reasons.clone(),
        };

        let config = Config {
            important_notification_reasons: vec!["review_requested".to_string()],
            persistent_important_notifications: true,
            persistent_notifications: false,
            ..Config::default()
        };

        let notification = Notification {
            id: "1".to_string(),
            unread: true,
            reason: "review_requested".to_string(), // This is an important reason
            updated_at: "2023-01-02T00:00:00Z".to_string(),
            last_read_at: None,
            subject: NotificationSubject {
                title: "Test notification".to_string(),
                url: Some("https://example.com/1".to_string()),
                latest_comment_url: None,
                kind: "PullRequest".to_string(),
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

        // Mock GitHub client that doesn't make real API calls
        let auth_manager = AuthManager::new().unwrap();
        let mut github_client = GitHubClient::new(auth_manager).unwrap();

        // Test with an important notification
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let history_manager = crate::HistoryManager::new(&db_path).unwrap();

        let result = handle_notification(
            &notification,
            &mock_notifier,
            &mut github_client,
            &config,
            &history_manager,
        )
        .await;

        assert!(result.is_ok());

        // Verify that the notification reason was passed correctly to the notifier
        let captured = captured_reasons.lock().unwrap();
        assert!(!captured.is_empty());
        assert_eq!(captured[0], "review_requested");
    }
}
