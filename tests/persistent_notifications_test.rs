use gh_notifier::{
    Config,
    poller::{DesktopNotifier, Notifier},
};

#[test]
fn test_persistent_notification_config() {
    // Test that the config contains the new option
    let mut config = Config::default();

    // Check default value
    assert!(!config.persistent_notifications);

    // Test with persistent notifications enabled
    config.persistent_notifications = true;
    assert!(config.persistent_notifications);
}

#[test]
fn test_desktop_notifier_uses_persistent_config() {
    let config = Config::default();
    let notifier = DesktopNotifier;

    // Test notification with default (non-persistent) config
    let result =
        notifier.send_notification("Test Title", "Test Body", "https://example.com", &config);
    assert!(result.is_ok());

    // Test notification with persistent config
    let mut persistent_config = config;
    persistent_config.persistent_notifications = true;

    let result = notifier.send_notification(
        "Test Title",
        "Test Body",
        "https://example.com",
        &persistent_config,
    );
    assert!(result.is_ok());
}

#[test]
fn test_storage_functionality() {
    use gh_notifier::models::PersistedNotification;
    use gh_notifier::storage::NotificationStorage;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_storage.db");

    let storage = NotificationStorage::new(&db_path).unwrap();

    // Test saving a notification
    let notification = PersistedNotification {
        id: "test-notification-id".to_string(),
        title: "Test Notification".to_string(),
        body: "This is a test notification body".to_string(),
        url: "https://github.com/test/repo/issues/1".to_string(),
        repository: "test/repo".to_string(),
        reason: "mention".to_string(),
        subject_type: "Issue".to_string(),
        is_read: false,
        received_at: "2023-01-01T00:00:00Z".to_string(),
        marked_read_at: None,
    };

    storage.save_notification(&notification).unwrap();

    // Test retrieving notifications
    let notifications = storage.get_all_notifications().unwrap();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].id, "test-notification-id");

    // Test marking as read
    storage.mark_as_read("test-notification-id").unwrap();

    let all_notifications = storage.get_all_notifications().unwrap();
    assert!(all_notifications[0].is_read);
    assert!(all_notifications[0].marked_read_at.is_some());

    // Test unread notifications
    let unread_notifications = storage.get_unread_notifications().unwrap();
    assert_eq!(unread_notifications.len(), 0);
}
