use crate::polling::utils::parse_duration;
use crate::polling::utils::parse_iso8601;
use crate::{Config, Notification};
use std::time::{SystemTime, UNIX_EPOCH};

/// Filters notifications based on time constraints
pub fn filter_by_time(notification: &Notification, config: &Config) -> bool {
    // 時間ベースのフィルタリング
    if let Some(ref min_time_str) = config.notification_filters().minimum_updated_time
        && let Ok(min_duration) = parse_duration(min_time_str)
        && let Ok(updated_time) = parse_iso8601(&notification.updated_at)
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
}
