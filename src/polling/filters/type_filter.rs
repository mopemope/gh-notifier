use crate::{Config, Notification};

/// Filters notifications based on subject type inclusion/exclusion rules
pub fn filter_by_subject_type(notification: &Notification, config: &Config) -> bool {
    // 通知タイプのフィルタリング
    if !config.notification_filters.include_subject_types.is_empty()
        && !config
            .notification_filters
            .include_subject_types
            .contains(&notification.subject.kind)
    {
        tracing::debug!(
            "Excluding notification type (not in include list): '{}' for notification '{}' (ID: {})",
            notification.subject.kind,
            notification.subject.title,
            notification.id
        );
        return false;
    }

    if config
        .notification_filters
        .exclude_subject_types
        .contains(&notification.subject.kind)
    {
        tracing::debug!(
            "Excluding notification type (in exclude list): '{}' for notification '{}' (ID: {})",
            notification.subject.kind,
            notification.subject.title,
            notification.id
        );
        return false;
    }

    true
}
