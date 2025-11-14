use crate::{Config, Notification};

/// Filters notifications based on repository inclusion/exclusion rules
pub fn filter_by_repository(notification: &Notification, config: &Config) -> bool {
    // include_repositoriesが指定されている場合、リストに含まれないリポジトリは除外
    if !config
        .notification_filters()
        .include_repositories
        .is_empty()
        && !config
            .notification_filters()
            .include_repositories
            .contains(&notification.repository.full_name)
    {
        return false;
    }

    // exclude_repositoriesのチェック（既存のロジック）
    for exclude_repo in &config.notification_filters().exclude_repositories {
        if notification.repository.full_name == *exclude_repo {
            return false;
        }
    }

    true
}
