use crate::{Config, Notification};

/// Filters notifications based on content inclusion/exclusion rules
pub fn filter_by_content(notification: &Notification, config: &Config) -> bool {
    // タイトルコンテンツベースのフィルタリング
    if !config.notification_filters().title_contains.is_empty() {
        let title_lower = notification.subject.title.to_lowercase();
        let mut contains_any = false;
        for keyword in &config.notification_filters().title_contains {
            if title_lower.contains(&keyword.to_lowercase()) {
                contains_any = true;
                break;
            }
        }
        if !contains_any {
            return false;
        }
    }

    for keyword in &config.notification_filters().title_not_contains {
        if notification
            .subject
            .title
            .to_lowercase()
            .contains(&keyword.to_lowercase())
        {
            return false;
        }
    }

    // リポジトリ名のフィルタリング
    if !config.notification_filters().repository_contains.is_empty() {
        let repo_name_lower = notification.repository.full_name.to_lowercase();
        let mut contains_any = false;
        for keyword in &config.notification_filters().repository_contains {
            if repo_name_lower.contains(&keyword.to_lowercase()) {
                contains_any = true;
                break;
            }
        }
        if !contains_any {
            return false;
        }
    }

    true
}
