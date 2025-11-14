use crate::{Config, Notification};

/// Filters notifications based on reason inclusion/exclusion rules
pub fn filter_by_reason(notification: &Notification, config: &Config) -> bool {
    // 通知理由のフィルタリング
    if !config.notification_filters().include_reasons.is_empty()
        && !config
            .notification_filters()
            .include_reasons
            .contains(&notification.reason)
    {
        return false;
    }

    if config
        .notification_filters()
        .exclude_reasons
        .contains(&notification.reason)
    {
        return false;
    }

    true
}
