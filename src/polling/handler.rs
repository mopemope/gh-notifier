use crate::poller::Notifier;
use crate::{GitHubClient, Notification};
use chrono::{DateTime, Local, Utc};

/// 通知を Notifier に渡して表示し、必要に応じて既読にする
pub async fn handle_notification(
    notification: &Notification,
    notifier: &dyn Notifier,
    github_client: &mut GitHubClient,
    mark_as_read_on_notify: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a more specific title with reason information
    let reason_text = get_reason_display_text(&notification.reason);
    let repo_name = if notification.repository.private {
        format!("🔒 {}", notification.repository.full_name)
    } else {
        notification.repository.full_name.clone()
    };
    let title = format!("{} - {}", repo_name, reason_text);

    // Create a more informative body with additional context
    let time_ago_text = format_time_ago(&notification.updated_at);
    let body = format!(
        "{}\n\n{} | {} | Updated: {}",
        notification.subject.title,
        notification.repository.name,
        format_subject_kind(&notification.subject.kind),
        time_ago_text
    );

    let url = &notification
        .subject
        .url
        .as_ref()
        .unwrap_or(&notification.url);

    notifier.send_notification(&title, &body, url)?;

    if mark_as_read_on_notify {
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
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
