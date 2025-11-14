use crate::{Config, Notification, StateManager};

/// 指定された最終確認日時以降の通知のみを抽出
pub fn filter_new_notifications<'a>(
    notifications: &'a Vec<Notification>,
    state_manager: &StateManager,
    config: &Config,
) -> Vec<&'a Notification> {
    let filtered_notifications: Vec<&'a Notification> =
        if let Some(last_checked) = state_manager.get_last_checked_at() {
            notifications
                .iter()
                .filter(|n| n.updated_at.as_str() > last_checked)
                .collect()
        } else {
            // 最終確認日時がない場合はすべて新しいと見なす
            notifications.iter().collect()
        };

    // 設定に基づいて通知をフィルタリング
    filtered_notifications
        .into_iter()
        .filter(|n| {
            // Early exit if quick checks fail
            // リポジトリプロパティのフィルタリング - これらのチェックは軽量なので先に行う
            if config.notification_filters().exclude_private_repos && n.repository.private {
                return false;
            }

            // 各フィルタを順に適用 (短絡評価により、いずれかがfalseなら以降は評価されない)
            crate::polling::filters::repository_filter::filter_by_repository(n, config)
                && crate::polling::filters::organization_filter::filter_by_organization(n, config)
                && crate::polling::filters::type_filter::filter_by_subject_type(n, config)
                && crate::polling::filters::reason_filter::filter_by_reason(n, config)
                && crate::polling::filters::content_filter::filter_by_content(n, config)
                && crate::polling::filters::time_filter::filter_by_time(n, config)
                && crate::polling::filters::draft_filter::filter_by_draft_status(n, config)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NotificationFilter;
    use crate::{Config, Notification, NotificationRepository, NotificationSubject, StateManager};

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

        // Use a config with no filters to allow all notifications
        let config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }

    #[test]
    fn test_filter_new_notifications_with_config() {
        let old_time = "2023-01-01T00:00:00Z";
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Mention notification".to_string(),
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
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Comment notification".to_string(),
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

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Reset notification filters to allow the test to work as expected
        config
            .notification
            .filters
            .exclude_reasons
            .push("comment".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1");
    }

    #[test]
    fn test_include_repositories_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Notification 1".to_string(),
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
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Notification 2".to_string(),
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
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Reset notification filters to allow the test to work as expected
        config
            .notification
            .filters
            .include_repositories
            .push("user/repo1".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1");
    }

    #[test]
    fn test_exclude_private_repos_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Public notification".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "repo1".to_string(),
                    full_name: "user/repo1".to_string(),
                    private: false, // Public repo
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Private notification".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "repo2".to_string(),
                    full_name: "user/repo2".to_string(),
                    private: true, // Private repo
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Reset notification filters to allow the test to work as expected
        config.notification.filters.exclude_private_repos = true;

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1"); // Only the public repo notification
    }

    #[test]
    fn test_title_contains_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Urgent bug fix needed".to_string(),
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
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Regular update".to_string(),
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
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Reset notification filters to allow the test to work as expected
        config
            .notification
            .filters
            .title_contains
            .push("urgent".to_string()); // Case insensitive

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1"); // Only the notification with "urgent" in title
    }

    #[test]
    fn test_parse_duration() {
        // Test basic units
        assert_eq!(
            crate::polling::utils::parse_duration("1s").unwrap(),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            crate::polling::utils::parse_duration("5m").unwrap(),
            std::time::Duration::from_secs(5 * 60)
        );
        assert_eq!(
            crate::polling::utils::parse_duration("2h").unwrap(),
            std::time::Duration::from_secs(2 * 60 * 60)
        );
        assert_eq!(
            crate::polling::utils::parse_duration("3d").unwrap(),
            std::time::Duration::from_secs(3 * 60 * 60 * 24)
        );

        // Test case insensitivity for multi-character units
        assert_eq!(
            crate::polling::utils::parse_duration("1ms").unwrap(),
            std::time::Duration::from_millis(1)
        );
        assert!(crate::polling::utils::parse_duration("invalid").is_err());
    }

    #[test]
    fn test_extract_org_name() {
        assert_eq!(crate::polling::utils::extract_org_name("org/repo"), "org");
        assert_eq!(
            crate::polling::utils::extract_org_name("user/project"),
            "user"
        );
        assert_eq!(crate::polling::utils::extract_org_name(""), "");
    }

    #[test]
    fn test_include_organizations_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "PR notification 1".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "repo1".to_string(),
                    full_name: "myorg/repo1".to_string(), // myorg
                    private: false,
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "PR notification 2".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "repo2".to_string(),
                    full_name: "otherorg/repo2".to_string(), // otherorg
                    private: false,
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notifications are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        config
            .notification
            .filters
            .include_organizations
            .push("myorg".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1");
    }

    #[test]
    fn test_exclude_organizations_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "PR notification 1".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "repo1".to_string(),
                    full_name: "spamorg/repo1".to_string(), // spamorg
                    private: false,
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "PR notification 2".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "repo2".to_string(),
                    full_name: "goodorg/repo2".to_string(), // goodorg
                    private: false,
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notifications are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        config
            .notification
            .filters
            .exclude_organizations
            .push("spamorg".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }

    #[test]
    fn test_exclude_subject_types_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "PR notification".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(), // This should be excluded
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
                    title: "Issue notification".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(), // This should pass through
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
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notifications are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        config
            .notification
            .filters
            .exclude_subject_types
            .push("PullRequest".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }

    #[test]
    fn test_title_not_contains_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(), // Use review_requested to match default config
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "This title has spam in it".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(), // Use PullRequest to match default config
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
                reason: "review_requested".to_string(), // Use review_requested to match default config
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Clean title without the bad word".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(), // Use PullRequest to match default config
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
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notifications are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        config
            .notification
            .filters
            .title_not_contains
            .push("spam".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "2");
    }

    #[test]
    fn test_repository_contains_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "mention".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Notification 1".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "main-project".to_string(),
                    full_name: "user/main-project".to_string(), // contains "main"
                    private: false,
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Notification 2".to_string(),
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(),
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "other-repo".to_string(),
                    full_name: "user/other-repo".to_string(), // does not contain "main"
                    private: false,
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notifications are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        config
            .notification
            .filters
            .repository_contains
            .push("main".to_string());

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1");
    }

    #[test]
    fn test_combined_filters() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Urgent PR Review".to_string(),
                    url: Some("https://example.com/1".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 1,
                    node_id: "node1".to_string(),
                    name: "important-project".to_string(),
                    full_name: "user/important-project".to_string(),
                    private: false,
                },
                url: "https://example.com/1".to_string(),
                subscription_url: "https://example.com/subscription/1".to_string(),
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "mention".to_string(), // Not review_requested
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Urgent notification".to_string(), // Contains "urgent"
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(), // Not PullRequest
                },
                repository: NotificationRepository {
                    id: 2,
                    node_id: "node2".to_string(),
                    name: "other-project".to_string(),
                    full_name: "user/other-project".to_string(),
                    private: false,
                },
                url: "https://example.com/2".to_string(),
                subscription_url: "https://example.com/subscription/2".to_string(),
            },
            Notification {
                id: "3".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Normal PR Review".to_string(),
                    url: Some("https://example.com/3".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
                },
                repository: NotificationRepository {
                    id: 3,
                    node_id: "node3".to_string(),
                    name: "normal-project".to_string(),
                    full_name: "user/normal-project".to_string(),
                    private: false,
                },
                url: "https://example.com/3".to_string(),
                subscription_url: "https://example.com/subscription/3".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // For this test, we want to set specific include filters to test combination
        config.notification.filters.include_reasons = vec!["review_requested".to_string()];
        config.notification.filters.include_subject_types = vec!["PullRequest".to_string()];
        config.notification.filters.title_contains = vec!["urgent".to_string()];

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        assert_eq!(new_notifications.len(), 1);
        assert_eq!(new_notifications[0].id, "1"); // Only notification 1 matches all criteria
    }

    #[test]
    fn test_exclude_draft_prs_filter() {
        let new_time = "2023-01-02T00:00:00Z";

        let notifications = vec![
            Notification {
                id: "1".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "[Draft] New feature implementation".to_string(), // Contains "Draft"
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
            },
            Notification {
                id: "2".to_string(),
                unread: true,
                reason: "review_requested".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Ready for review - New feature".to_string(), // Regular PR
                    url: Some("https://example.com/2".to_string()),
                    latest_comment_url: None,
                    kind: "PullRequest".to_string(),
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
            Notification {
                id: "3".to_string(),
                unread: true,
                reason: "comment".to_string(),
                updated_at: new_time.to_string(),
                last_read_at: None,
                subject: NotificationSubject {
                    title: "Issue comment".to_string(),
                    url: Some("https://example.com/3".to_string()),
                    latest_comment_url: None,
                    kind: "Issue".to_string(), // Not a PR
                },
                repository: NotificationRepository {
                    id: 3,
                    node_id: "node3".to_string(),
                    name: "repo3".to_string(),
                    full_name: "user/repo3".to_string(),
                    private: false,
                },
                url: "https://example.com/3".to_string(),
                subscription_url: "https://example.com/subscription/3".to_string(),
            },
        ];

        let mut state_manager = StateManager::new().unwrap();
        state_manager.update_last_checked_at("2023-01-01T00:00:00Z".to_string());

        let mut config = Config {
            notification: crate::config::NotificationConfig {
                filters: NotificationFilter::default(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Clear include filters so all notification types are considered
        config.notification.filters.include_reasons = vec![];
        config.notification.filters.include_subject_types = vec![];
        // Enable the draft PR exclusion filter
        config.notification.filters.exclude_draft_prs = true;

        let new_notifications = filter_new_notifications(&notifications, &state_manager, &config);

        // Should have 2 notifications: the non-draft PR and the issue comment
        assert_eq!(new_notifications.len(), 2);
        // Check that the draft PR (id="1") is not in the results
        for notification in new_notifications.iter() {
            assert_ne!(notification.id, "1"); // Draft PR should be excluded
        }
    }
}
