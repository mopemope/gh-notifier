use crate::{Notification, StateManager};

/// 指定された最終確認日時以降の通知のみを抽出
pub fn filter_new_notifications<'a>(
    notifications: &'a [Notification],
    state_manager: &StateManager,
) -> Vec<&'a Notification> {
    if let Some(last_checked) = state_manager.get_last_checked_at() {
        notifications
            .iter()
            .filter(|n| n.updated_at.as_str() > last_checked)
            .collect()
    } else {
        // 最終確認日時がない場合はすべて新しいと見なす
        notifications.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Notification, NotificationRepository, NotificationSubject, StateManager};

    #[test]
    fn test_filter_new_notifications() {
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

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at(old_time.to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }
}
