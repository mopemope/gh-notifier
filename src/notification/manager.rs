use crate::errors::{AppError, NotificationError};
use crate::github::types::Notification as GitHubNotification;
use crate::notification::types::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// 通知ストレージトレイト
pub trait NotificationStorage: Send + Sync {
    /// 通知を保存
    fn save(&self, notification: &UserNotification) -> Result<(), NotificationError>;

    /// 通知を取得
    fn get(&self, id: &str) -> Result<Option<UserNotification>, NotificationError>;

    /// すべての通知を取得
    fn get_all(&self) -> Result<Vec<UserNotification>, NotificationError>;

    /// 通知を削除
    fn delete(&self, id: &str) -> Result<(), NotificationError>;

    /// 既読状態を更新
    fn mark_as_read(&self, id: &str) -> Result<(), NotificationError>;
}

/// メモリストレージ実装
#[derive(Default)]
pub struct InMemoryNotificationStorage {
    notifications: Arc<Mutex<HashMap<String, UserNotification>>>,
}

impl NotificationStorage for InMemoryNotificationStorage {
    fn save(&self, notification: &UserNotification) -> Result<(), NotificationError> {
        let mut storage = self
            .notifications
            .lock()
            .map_err(|_| NotificationError::Generic {
                message: "Failed to acquire storage lock".to_string(),
            })?;

        storage.insert(notification.id.clone(), notification.clone());
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<UserNotification>, NotificationError> {
        let storage = self
            .notifications
            .lock()
            .map_err(|_| NotificationError::Generic {
                message: "Failed to acquire storage lock".to_string(),
            })?;

        Ok(storage.get(id).cloned())
    }

    fn get_all(&self) -> Result<Vec<UserNotification>, NotificationError> {
        let storage = self
            .notifications
            .lock()
            .map_err(|_| NotificationError::Generic {
                message: "Failed to acquire storage lock".to_string(),
            })?;

        Ok(storage.values().cloned().collect())
    }

    fn delete(&self, id: &str) -> Result<(), NotificationError> {
        let mut storage = self
            .notifications
            .lock()
            .map_err(|_| NotificationError::Generic {
                message: "Failed to acquire storage lock".to_string(),
            })?;

        storage.remove(id);
        Ok(())
    }

    fn mark_as_read(&self, id: &str) -> Result<(), NotificationError> {
        let mut storage = self
            .notifications
            .lock()
            .map_err(|_| NotificationError::Generic {
                message: "Failed to acquire storage lock".to_string(),
            })?;

        if let Some(notification) = storage.get_mut(id) {
            notification.is_read = true;
            notification.read_at = Some(chrono::Utc::now());
        } else {
            return Err(NotificationError::Generic {
                message: format!("Notification not found: {}", id),
            });
        }

        Ok(())
    }
}

/// 通知マネージャー
pub struct NotificationManager {
    storage: Box<dyn NotificationStorage>,
    config: NotificationConfig,
}

impl NotificationManager {
    /// 新しい通知マネージャーを作成
    pub fn new(storage: Box<dyn NotificationStorage>, config: NotificationConfig) -> Self {
        Self { storage, config }
    }

    /// GitHub通知を処理
    pub fn process_github_notification(
        &self,
        github_notification: &GitHubNotification,
    ) -> Result<Option<UserNotification>, AppError> {
        info!("Processing GitHub notification: {}", github_notification.id);

        // 変換を試みる
        match NotificationConverter::convert_github_notification(github_notification, &self.config)
        {
            Ok(user_notification) => {
                // ストレージに保存
                if let Err(e) = self.storage.save(&user_notification) {
                    error!("Failed to save notification: {}", e);
                    return Err(e.into());
                }

                info!(
                    "Successfully processed notification: {}",
                    user_notification.id
                );

                Ok(Some(user_notification))
            }
            Err(NotificationError::FilterError { reason }) => {
                // フィルタによって除外された場合は無視
                info!(
                    "Notification filtered out: {} - {}",
                    github_notification.id, reason
                );
                Ok(None)
            }
            Err(e) => {
                error!("Failed to convert notification: {}", e);
                Err(e.into())
            }
        }
    }

    /// 通知を取得
    pub fn get_notification(&self, id: &str) -> Result<Option<UserNotification>, AppError> {
        self.storage.get(id).map_err(|e| e.into())
    }

    /// すべての通知を取得
    pub fn get_all_notifications(&self) -> Result<Vec<UserNotification>, AppError> {
        let mut notifications = self.storage.get_all()?;

        // 重要度でソート
        notifications.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(notifications)
    }

    /// 未読通知を取得
    pub fn get_unread_notifications(&self) -> Result<Vec<UserNotification>, AppError> {
        let notifications = self.get_all_notifications()?;
        Ok(notifications.into_iter().filter(|n| !n.is_read).collect())
    }

    /// 通知を既読にする
    pub fn mark_as_read(&self, id: &str) -> Result<(), AppError> {
        self.storage.mark_as_read(id)?;

        if self.config.mark_as_read_on_notify() {
            // ここでGitHub APIを呼び出して既読にするロジックを追加
            // 簡略化のため、実装は割愛
            info!("Marked notification as read: {}", id);
        }

        Ok(())
    }

    /// 通知を削除
    pub fn delete_notification(&self, id: &str) -> Result<(), AppError> {
        Ok(self.storage.delete(id)?)
    }

    /// 既読期限が過ぎた通知をクリーンアップ
    pub fn cleanup_old_notifications(&self, max_age_days: i64) -> Result<Vec<String>, AppError> {
        let notifications = self.get_all_notifications()?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days);

        let mut deleted_ids = Vec::new();

        for notification in notifications {
            if notification.created_at < cutoff {
                if let Err(e) = self.delete_notification(&notification.id) {
                    warn!(
                        "Failed to delete old notification {}: {}",
                        notification.id, e
                    );
                } else {
                    deleted_ids.push(notification.id);
                }
            }
        }

        Ok(deleted_ids)
    }

    /// バッチ処理用の未読通知を取得
    pub fn get_batch_notifications(&self) -> Result<Vec<UserNotification>, AppError> {
        let mut notifications = self.get_unread_notifications()?;

        if self.config.batch.batch_size > 0 {
            notifications.truncate(self.config.batch.batch_size);
        }

        Ok(notifications)
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config;
    }

    /// 統計情報を取得
    pub fn get_statistics(&self) -> Result<NotificationStatistics, AppError> {
        let all_notifications = self.get_all_notifications()?;
        let unread_count = all_notifications.iter().filter(|n| !n.is_read).count();
        let read_count = all_notifications.len() - unread_count;

        let mut priority_counts = std::collections::HashMap::new();
        for notification in &all_notifications {
            *priority_counts.entry(notification.priority).or_insert(0) += 1;
        }

        Ok(NotificationStatistics {
            total_count: all_notifications.len(),
            unread_count,
            read_count,
            priority_counts,
        })
    }
}

/// 通知統計情報
#[derive(Debug)]
pub struct NotificationStatistics {
    /// 総通知数
    pub total_count: usize,
    /// 未読通知数
    pub unread_count: usize,
    /// 既読通知数
    pub read_count: usize,
    /// 重要度別カウント
    pub priority_counts: std::collections::HashMap<NotificationPriority, usize>,
}

/// 通知ディスパッチャー
pub trait NotificationDispatcher: Send + Sync {
    /// 通知を配信
    fn dispatch(&self, notification: &UserNotification) -> Result<(), NotificationError>;
}

/// デスクトップ通知ディスパッチャー
pub struct DesktopNotificationDispatcher;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_in_memory_storage_new() {
        let storage = InMemoryNotificationStorage::default();
        assert!(storage.get("test").unwrap().is_none());
    }

    #[test]
    fn test_in_memory_storage_save_and_get() {
        let storage = InMemoryNotificationStorage::default();
        let notification = create_test_notification("1");

        // 保存
        assert!(storage.save(&notification).is_ok());

        // 取得
        let retrieved = storage.get("1").unwrap().unwrap();
        assert_eq!(retrieved.id, "1");
        assert_eq!(retrieved.title, "Test Notification 1");
    }

    #[test]
    fn test_in_memory_storage_delete() {
        let storage = InMemoryNotificationStorage::default();
        let notification = create_test_notification("1");

        // 保存
        storage.save(&notification).unwrap();
        assert!(storage.get("1").unwrap().is_some());

        // 削除
        storage.delete("1").unwrap();
        assert!(storage.get("1").unwrap().is_none());
    }

    #[test]
    fn test_in_memory_storage_mark_as_read() {
        let storage = InMemoryNotificationStorage::default();
        let mut notification = create_test_notification("1");
        notification.is_read = false;

        // 保存
        storage.save(&notification).unwrap();

        // 既読にする
        storage.mark_as_read("1").unwrap();

        // 確認
        let updated = storage.get("1").unwrap().unwrap();
        assert!(updated.is_read);
        assert!(updated.read_at.is_some());
    }

    #[test]
    fn test_notification_manager_new() {
        let storage = Box::new(InMemoryNotificationStorage::default());
        let config = crate::notification::types::NotificationConfig::default();
        let manager = NotificationManager::new(storage, config);

        assert!(manager.get_all_notifications().unwrap().is_empty());
    }

    #[test]
    fn test_notification_manager_save_and_get() {
        let storage = Box::new(InMemoryNotificationStorage::default());
        let config = crate::notification::types::NotificationConfig::default();
        let manager = NotificationManager::new(storage, config);

        let notification = create_test_notification("1");
        // Use the appropriate method name that exists in NotificationManager
        // If save_notification doesn't exist, use storage directly for the test
        manager.storage.save(&notification).unwrap();

        let retrieved = manager.get_notification("1").unwrap().unwrap();
        assert_eq!(retrieved.id, "1");
    }

    #[test]
    fn test_notification_manager_get_unread_notifications() {
        let storage = Box::new(InMemoryNotificationStorage::default());
        let config = crate::notification::types::NotificationConfig::default();
        let manager = NotificationManager::new(storage, config);

        // 未読通知を追加
        let unread_notification = create_test_notification("1");
        manager.storage.save(&unread_notification).unwrap();

        // 既読通知を追加
        let mut read_notification = create_test_notification("2");
        read_notification.is_read = true;
        manager.storage.save(&read_notification).unwrap();

        let unread_notifications = manager.get_unread_notifications().unwrap();
        assert_eq!(unread_notifications.len(), 1);
        assert_eq!(unread_notifications[0].id, "1");
    }

    #[test]
    fn test_notification_manager_statistics() {
        let storage = Box::new(InMemoryNotificationStorage::default());
        let config = crate::notification::types::NotificationConfig::default();
        let manager = NotificationManager::new(storage, config);

        // 通知をいくつか追加
        for i in 1..=5 {
            let mut notification = create_test_notification(&i.to_string());
            if i <= 2 {
                notification.is_read = true;
            }
            manager.storage.save(&notification).unwrap();
        }

        let stats = manager.get_statistics().unwrap();
        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.read_count, 2);
        assert_eq!(stats.unread_count, 3);
    }

    #[test]
    fn test_user_notification_creation() {
        let notification = create_test_notification("1");

        assert_eq!(notification.id, "1");
        assert_eq!(notification.title, "Test Notification 1");
        assert_eq!(
            notification.priority,
            crate::notification::types::NotificationPriority::Normal
        );
        assert_eq!(
            notification.category,
            crate::notification::types::NotificationCategory::Other
        );
        assert!(!notification.is_read);
    }

    #[test]
    fn test_user_notification_with_actions() {
        let mut notification = create_test_notification("1");
        notification.actions = vec![crate::notification::types::NotificationAction {
            name: "Open".to_string(),
            url: "https://example.com".to_string(),
            action_type: crate::notification::types::NotificationActionType::OpenUrl,
        }];

        assert_eq!(notification.actions.len(), 1);
        assert_eq!(notification.actions[0].name, "Open");
        assert_eq!(notification.actions[0].url, "https://example.com");
    }

    #[test]
    fn test_desktop_notification_dispatcher() {
        let dispatcher = DesktopNotificationDispatcher;
        let notification = create_test_notification("1");

        // ディスパッチャーがエラーを返さずに動作することを確認
        assert!(dispatcher.dispatch(&notification).is_ok());
    }

    // テスト用の通知を作成するヘルパー関数
    fn create_test_notification(id: &str) -> UserNotification {
        UserNotification {
            id: id.to_string(),
            title: format!("Test Notification {}", id),
            body: "This is a test notification".to_string(),
            icon_url: Some("https://example.com/icon.png".to_string()),
            priority: crate::notification::types::NotificationPriority::Normal,
            category: crate::notification::types::NotificationCategory::Other,
            url: "https://example.com/notification/1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_read: false,
            read_at: None,
            actions: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_notification_priority_ordering() {
        use crate::notification::types::NotificationPriority;

        let high_priority = NotificationPriority::High;
        let normal_priority = NotificationPriority::Normal;
        let low_priority = NotificationPriority::Low;

        // 優先度の比較をテスト
        assert!(high_priority > normal_priority);
        assert!(normal_priority > low_priority);
        assert!(high_priority > low_priority);
    }

    #[test]
    fn test_notification_category_default() {
        use crate::notification::types::NotificationCategory;

        let category = NotificationCategory::default();
        assert_eq!(category, NotificationCategory::Other);
    }

    #[test]
    fn test_notification_action_type() {
        use crate::notification::types::NotificationActionType;

        let open_action = NotificationActionType::OpenUrl;
        let mark_read_action = NotificationActionType::MarkAsRead;

        // 型が正しく定義されていることを確認
        match open_action {
            NotificationActionType::OpenUrl => {}
            _ => panic!("Expected OpenUrl variant"),
        }

        match mark_read_action {
            NotificationActionType::MarkAsRead => {}
            _ => panic!("Expected MarkAsRead variant"),
        }
    }

    #[test]
    fn test_notification_config_default() {
        let config = crate::notification::types::NotificationConfig::default();

        assert_eq!(config.mark_as_read_on_notify(), false);
        assert_eq!(config.batch.batch_size, 0);
        assert_eq!(config.batch.batch_interval_sec, 30);
        assert!(!config.filter.exclude_private_repos);
        assert!(!config.filter.exclude_fork_repos);
    }

    #[test]
    fn test_notification_filter_default() {
        let filter = crate::notification::types::NotificationFilter::default();

        assert!(filter.exclude_repositories.is_empty());
        assert!(filter.include_repositories.is_empty());
        assert!(filter.include_reasons.is_empty());
        assert!(filter.title_contains.is_empty());
        assert!(filter.title_not_contains.is_empty());
        assert!(filter.repository_contains.is_empty());
        assert_eq!(filter.exclude_private_repos, false);
        assert_eq!(filter.exclude_fork_repos, false);
        assert_eq!(filter.exclude_draft_prs, false);
        assert_eq!(filter.exclude_participating, false);
        assert_eq!(filter.minimum_updated_time, None);
    }
}

impl NotificationDispatcher for DesktopNotificationDispatcher {
    fn dispatch(&self, notification: &UserNotification) -> Result<(), NotificationError> {
        // 実際の通知配信ロジックを実装
        // 簡略化のため、ログ出力のみ
        info!(
            "Sending desktop notification: {} - {}",
            notification.title, notification.body
        );

        // 実際には以下のようにnotify-rustなどを使用
        // notify_rust::Notification::new()
        //     .summary(notification.title)
        //     .body(notification.body)
        //     .show()?;

        Ok(())
    }
}

/// 通知サービス
pub struct NotificationService {
    manager: NotificationManager,
    dispatcher: Box<dyn NotificationDispatcher>,
}

impl NotificationService {
    /// 新しい通知サービスを作成
    pub fn new(manager: NotificationManager, dispatcher: Box<dyn NotificationDispatcher>) -> Self {
        Self {
            manager,
            dispatcher,
        }
    }

    /// 通知を完全に処理
    pub async fn process_notification(
        &self,
        notification: &UserNotification,
    ) -> Result<Option<UserNotification>, AppError> {
        // 通知を配信
        if let Err(e) = self.dispatcher.dispatch(notification) {
            error!("Failed to dispatch notification: {}", e);
            // 配信失敗でも処理は続行
        }

        // 既読設定が有効なら既読にする
        if self.manager.config.mark_as_read_on_notify()
            && let Err(e) = self.manager.mark_as_read(&notification.id)
        {
            warn!("Failed to mark notification as read: {}", e);
        }

        Ok(Some(notification.clone()))
    }

    /// バッチで通知を処理
    pub async fn process_batch_notifications(&self) -> Result<Vec<UserNotification>, AppError> {
        let notifications = self.manager.get_batch_notifications()?;
        let mut processed = Vec::new();

        for notification in notifications {
            if let Ok(Some(processed_notification)) = self.process_notification(&notification).await
            {
                processed.push(processed_notification);
            }
        }

        Ok(processed)
    }

    /// 統計情報を取得
    pub fn get_statistics(&self) -> Result<NotificationStatistics, AppError> {
        self.manager.get_statistics()
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.manager.update_config(config);
    }
}
