use gh_notifier::HistoryManager;
use gh_notifier::models::{Notification, NotificationRepository, NotificationSubject};
use tempfile::tempdir;

#[tokio::test]
async fn test_read_status_persistence_after_restart() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Initialize the history manager for the first time
    let history_manager = HistoryManager::new(&db_path).unwrap();

    // Create a test notification
    let notification = Notification {
        id: "test-notification-id".to_string(),
        unread: true,
        reason: "review_requested".to_string(),
        updated_at: "2023-01-01T00:00:00Z".to_string(),
        last_read_at: None,
        subject: NotificationSubject {
            title: "Test notification".to_string(),
            url: Some("https://github.com/test/repo/pull/1".to_string()),
            latest_comment_url: None,
            kind: "PullRequest".to_string(),
        },
        repository: NotificationRepository {
            id: 1,
            node_id: "repo1".to_string(),
            name: "repo".to_string(),
            full_name: "test/repo".to_string(),
            private: false,
        },
        url: "https://api.github.com/notifications/threads/1".to_string(),
        subscription_url: "https://api.github.com/notifications/threads/1/subscription".to_string(),
    };

    // Save the notification
    history_manager.save_notification(&notification).unwrap();

    // Check that the notification is initially unread
    let all_notifications = history_manager.get_all_notifications().unwrap();
    assert_eq!(all_notifications.len(), 1);
    assert_eq!(all_notifications[0].id, "test-notification-id");
    assert!(!all_notifications[0].is_read);

    // Mark the notification as read
    history_manager
        .mark_as_read("test-notification-id")
        .unwrap();

    // Verify that the notification is now marked as read
    let all_notifications_after_mark = history_manager.get_all_notifications().unwrap();
    assert!(all_notifications_after_mark[0].is_read);
    assert!(all_notifications_after_mark[0].marked_read_at.is_some());

    // Simulate a restart by creating a new HistoryManager instance
    // This will re-open the same database file
    drop(history_manager); // Explicitly drop the first instance

    let new_history_manager = HistoryManager::new(&db_path).unwrap();

    // Check that the read status is preserved after "restart"
    let notifications_after_restart = new_history_manager.get_all_notifications().unwrap();
    assert_eq!(notifications_after_restart.len(), 1);
    assert_eq!(notifications_after_restart[0].id, "test-notification-id");
    assert!(notifications_after_restart[0].is_read);
    assert!(notifications_after_restart[0].marked_read_at.is_some());

    println!("Read status successfully persisted after restart!");
}
