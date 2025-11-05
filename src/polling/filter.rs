use crate::{Config, Notification, StateManager};
use chrono::DateTime;
use std::time::{SystemTime, UNIX_EPOCH};

/// 指定された最終確認日時以降の通知のみを抽出
pub fn filter_new_notifications<'a>(
    notifications: &'a [Notification],
    state_manager: &StateManager,
    config: &Config,
) -> Vec<&'a Notification> {
    let mut filtered_notifications: Vec<&'a Notification> =
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
    filtered_notifications.retain(|n| {
        // リポジトリベースのフィルタリング
        // include_repositoriesが指定されている場合、リストに含まれないリポジトリは除外
        if !config.notification_filters.include_repositories.is_empty()
            && !config
                .notification_filters
                .include_repositories
                .contains(&n.repository.full_name)
        {
            return false;
        }

        // exclude_repositoriesのチェック（既存のロジック）
        for exclude_repo in &config.notification_filters.exclude_repositories {
            if n.repository.full_name == *exclude_repo {
                return false;
            }
        }

        // 組織ベースのフィルタリング
        let org_name = extract_org_name(&n.repository.full_name);
        if !config.notification_filters.include_organizations.is_empty()
            && !config
                .notification_filters
                .include_organizations
                .contains(&org_name)
        {
            return false;
        }

        if config
            .notification_filters
            .exclude_organizations
            .contains(&org_name)
        {
            return false;
        }

        // リポジトリプロパティのフィルタリング
        if config.notification_filters.exclude_private_repos && n.repository.private {
            return false;
        }

        // NOTE: fork判定はGitHub APIのレスポンスには含まれないため、実装は一旦スキップ
        // if config.notification_filters.exclude_fork_repos && n.repository.is_fork {  // is_forkは存在しない
        //     return false;
        // }

        // 通知タイプのフィルタリング
        if !config.notification_filters.include_subject_types.is_empty()
            && !config
                .notification_filters
                .include_subject_types
                .contains(&n.subject.kind)
        {
            return false;
        }

        if config
            .notification_filters
            .exclude_subject_types
            .contains(&n.subject.kind)
        {
            return false;
        }

        // 通知理由のフィルタリング
        if !config.notification_filters.include_reasons.is_empty()
            && !config
                .notification_filters
                .include_reasons
                .contains(&n.reason)
        {
            return false;
        }

        if config
            .notification_filters
            .exclude_reasons
            .contains(&n.reason)
        {
            return false;
        }

        // タイトルコンテンツベースのフィルタリング
        if !config.notification_filters.title_contains.is_empty() {
            let title_lower = n.subject.title.to_lowercase();
            let mut contains_any = false;
            for keyword in &config.notification_filters.title_contains {
                if title_lower.contains(&keyword.to_lowercase()) {
                    contains_any = true;
                    break;
                }
            }
            if !contains_any {
                return false;
            }
        }

        for keyword in &config.notification_filters.title_not_contains {
            if n.subject
                .title
                .to_lowercase()
                .contains(&keyword.to_lowercase())
            {
                return false;
            }
        }

        // リポジトリ名のフィルタリング
        if !config.notification_filters.repository_contains.is_empty() {
            let repo_name_lower = n.repository.full_name.to_lowercase();
            let mut contains_any = false;
            for keyword in &config.notification_filters.repository_contains {
                if repo_name_lower.contains(&keyword.to_lowercase()) {
                    contains_any = true;
                    break;
                }
            }
            if !contains_any {
                return false;
            }
        }

        // 参加スレッドのフィルタリング
        // NOTE: GitHub APIの通知レスポンスにはparticipatingフィールドがないため、
        // 代わりにlast_read_atがNoneかどうかで未読のみを対象とするようなフィルタとして実装
        // 本来のexclude_participatingはAPIから得られる情報で判定することは難しい
        if config.notification_filters.exclude_participating {
            // NOTE: 本来のexclude_participatingの実装はAPIからその情報を取得できないためスキップ
            // 実際のGitHub API通知レスポンスではparticipatingフィールドが含まれない
        }

        // 時間ベースのフィルタリング
        if let Some(ref min_time_str) = config.notification_filters.minimum_updated_time
            && let Ok(min_duration) = parse_duration(min_time_str)
            && let Ok(updated_time) = parse_iso8601(&n.updated_at)
        {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // 更新時刻が基準時刻より古い場合は除外
            if updated_time < current_time - min_duration.as_secs() {
                return false;
            }
        }

        true
    });

    filtered_notifications
}

/// リポジトリ名から組織名を抽出
fn extract_org_name(full_repo_name: &str) -> String {
    if let Some(pos) = full_repo_name.find('/') {
        full_repo_name[..pos].to_string()
    } else {
        full_repo_name.to_string() // ユーザ名/repo_name形式でない場合はそのまま返す
    }
}

/// ISO 8601形式の日時文字列をUNIXタイムスタンプに変換
fn parse_iso8601(date_str: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    // 簡易的なISO 8601パーサー（実際にはより完全なパーサーが必要）
    // 例: "2023-01-01T00:00:00Z"
    let dt = DateTime::parse_from_rfc3339(date_str)?;
    Ok(dt.timestamp() as u64)
}

/// 時間表記（例: "1h", "30m", "2d"）をDurationに変換
fn parse_duration(
    duration_str: &str,
) -> Result<std::time::Duration, Box<dyn std::error::Error + Send + Sync>> {
    let duration_str = duration_str.trim();
    if duration_str.is_empty() {
        return Ok(std::time::Duration::from_secs(0));
    }

    if duration_str.len() < 2 {
        return Err("Duration string too short".into());
    }

    // Check for two-character units first
    if duration_str.len() >= 2 {
        let last_two = &duration_str[duration_str.len() - 2..];
        let first_part = &duration_str[..duration_str.len() - 2];

        match last_two {
            "ms" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_millis(num));
                }
            }
            "hr" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60));
                }
            }
            "mo" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24 * 30)); // 月を30日として計算
                }
            }
            "yr" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24 * 365)); // 年を365日として計算
                }
            }
            _ => {
                // Not a two-character unit, continue to check one-character units
            }
        }
    }

    // Check for one-character units
    if !duration_str.is_empty() {
        let last_char = &duration_str[duration_str.len() - 1..];
        let first_part = &duration_str[..duration_str.len() - 1];

        match last_char {
            "s" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num));
                }
            }
            "m" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60));
                }
            }
            "h" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60));
                }
            }
            "d" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24));
                }
            }
            _ => {
                // Not a recognized unit
            }
        }
    }

    Err("Invalid duration format".into())
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let config = Config::default();
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

        let mut config = Config::default();
        config
            .notification_filters
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

        let mut config = Config::default();
        config
            .notification_filters
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

        let mut config = Config::default();
        config.notification_filters.exclude_private_repos = true;

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

        let mut config = Config::default();
        config
            .notification_filters
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
            parse_duration("1s").unwrap(),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            parse_duration("5m").unwrap(),
            std::time::Duration::from_secs(5 * 60)
        );
        assert_eq!(
            parse_duration("2h").unwrap(),
            std::time::Duration::from_secs(2 * 60 * 60)
        );
        assert_eq!(
            parse_duration("3d").unwrap(),
            std::time::Duration::from_secs(3 * 60 * 60 * 24)
        );

        // Test case insensitivity for multi-character units
        assert_eq!(
            parse_duration("1ms").unwrap(),
            std::time::Duration::from_millis(1)
        );
        assert!(parse_duration("invalid").is_err());
    }

    #[test]
    fn test_extract_org_name() {
        assert_eq!(extract_org_name("org/repo"), "org");
        assert_eq!(extract_org_name("user/project"), "user");
        assert_eq!(extract_org_name("single"), "single");
        assert_eq!(extract_org_name(""), "");
    }
}
