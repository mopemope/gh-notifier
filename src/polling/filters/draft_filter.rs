use crate::{Config, Notification};

/// Filters notifications based on draft PR status
pub fn filter_by_draft_status(notification: &Notification, config: &Config) -> bool {
    // ドラフトPRのフィルタリング (タイトルに特定のパターンがある場合)
    if config.notification_filters.exclude_draft_prs && notification.subject.kind == "PullRequest" {
        // Draft PR かどうかをタイトルから判定 (一般的なパターンをチェック)
        let title_lower = notification.subject.title.to_lowercase();
        let is_draft_by_title = title_lower.contains("draft")
            || title_lower.contains("[draft]")
            || title_lower.starts_with("draft:")
            || title_lower.starts_with("[draft")
            || title_lower.contains("(draft");

        if is_draft_by_title {
            tracing::debug!(
                "Excluding Draft PR notification: '{}' (ID: {})",
                notification.subject.title,
                notification.id
            );
            return false;
        }
    } else if config.notification_filters.exclude_draft_prs && notification.subject.kind != "PullRequest" {
        // Log that this filter doesn't apply to non-PR notifications
        tracing::trace!(
            "Draft filter skipped for non-PR notification (type: {}, ID: {})",
            notification.subject.kind,
            notification.id
        );
    }

    true
}
