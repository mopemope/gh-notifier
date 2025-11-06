use crate::models::{Notification, PersistedNotification};
use crate::storage::NotificationStorage;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 通知履歴を管理するためのマネージャー
#[derive(Clone)]
pub struct HistoryManager {
    storage: Arc<Mutex<NotificationStorage>>,
}

impl HistoryManager {
    /// 新しいHistoryManagerインスタンスを作成
    pub fn new(db_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = NotificationStorage::new(db_path)?;
        Ok(HistoryManager {
            storage: Arc::new(Mutex::new(storage)),
        })
    }

    /// 通知を履歴として保存（重複チェック付き）
    pub fn save_notification(
        &self,
        notification: &Notification,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();

        // 重複チェック
        if storage.notification_exists(&notification.id)? {
            return Ok(()); // 既に保存されている場合は何もしない
        }

        // Notification構造体からPersistedNotification構造体に変換
        let persisted_notification = PersistedNotification {
            id: notification.id.clone(),
            title: notification.subject.title.clone(),
            body: format!(
                "{}\nRepository: {}\nType: {}",
                notification.subject.title,
                notification.repository.full_name,
                notification.subject.kind
            ),
            url: notification
                .subject
                .url
                .clone()
                .unwrap_or_else(|| notification.url.clone()),
            repository: notification.repository.full_name.clone(),
            reason: notification.reason.clone(),
            subject_type: notification.subject.kind.clone(),
            is_read: !notification.unread, // GitHubのunreadがfalseなら既読
            received_at: notification.updated_at.clone(),
            marked_read_at: notification.last_read_at.clone(),
        };

        storage.save_notification(&persisted_notification)?;
        Ok(())
    }

    /// すべての通知履歴を取得
    pub fn get_all_notifications(
        &self,
    ) -> Result<Vec<PersistedNotification>, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get_all_notifications()?)
    }

    /// 未読通知のみを取得
    pub fn get_unread_notifications(
        &self,
    ) -> Result<Vec<PersistedNotification>, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get_unread_notifications()?)
    }

    /// リポジトリ名でフィルタリングして通知を取得
    pub fn get_notifications_by_repository(
        &self,
        repository: &str,
    ) -> Result<Vec<PersistedNotification>, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        // storageモジュールにフィルタリング機能を追加する必要がある
        let all_notifications = storage.get_all_notifications()?;
        let filtered = all_notifications
            .into_iter()
            .filter(|n| n.repository.contains(repository))
            .collect();
        Ok(filtered)
    }

    /// 通知理由でフィルタリングして通知を取得
    pub fn get_notifications_by_reason(
        &self,
        reason: &str,
    ) -> Result<Vec<PersistedNotification>, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        let all_notifications = storage.get_all_notifications()?;
        let filtered = all_notifications
            .into_iter()
            .filter(|n| n.reason == reason)
            .collect();
        Ok(filtered)
    }

    /// 通知タイプでフィルタリングして通知を取得
    pub fn get_notifications_by_subject_type(
        &self,
        subject_type: &str,
    ) -> Result<Vec<PersistedNotification>, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        let all_notifications = storage.get_all_notifications()?;
        let filtered = all_notifications
            .into_iter()
            .filter(|n| n.subject_type == subject_type)
            .collect();
        Ok(filtered)
    }

    /// 通知を既読にする
    pub fn mark_as_read(&self, notification_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        storage.mark_as_read(notification_id)?;
        Ok(())
    }

    /// すべての通知を既読にする
    pub fn mark_all_as_read(&self) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        storage.mark_all_as_read()?;
        Ok(())
    }

    /// 通知を削除
    pub fn delete_notification(
        &self,
        notification_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        storage.delete_notification(notification_id)?;
        Ok(())
    }

    /// 保存された通知の数を取得
    pub fn get_notification_count(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get_notification_count()?)
    }

    /// 通知が存在するか確認
    pub fn notification_exists(
        &self,
        notification_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.notification_exists(notification_id)?)
    }

    /// 通知が既読かどうか確認
    pub fn is_notification_read(
        &self,
        notification_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let storage = self.storage.lock().unwrap();
        let notifications = storage.get_all_notifications()?;
        let notification = notifications.iter().find(|n| n.id == notification_id);
        Ok(notification.map(|n| n.is_read).unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_history_manager() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_history.db");

        let history_manager = HistoryManager::new(&db_path).unwrap();

        // 通知を作成
        let notification = Notification {
            id: "test-notification-id".to_string(),
            unread: true,
            reason: "review_requested".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            last_read_at: None,
            subject: crate::models::NotificationSubject {
                title: "Test notification".to_string(),
                url: Some("https://github.com/test/repo/pull/1".to_string()),
                latest_comment_url: None,
                kind: "PullRequest".to_string(),
            },
            repository: crate::models::NotificationRepository {
                id: 1,
                node_id: "repo1".to_string(),
                name: "repo".to_string(),
                full_name: "test/repo".to_string(),
                private: false,
            },
            url: "https://api.github.com/notifications/threads/1".to_string(),
            subscription_url: "https://api.github.com/notifications/threads/1/subscription"
                .to_string(),
        };

        // 通知を保存
        history_manager.save_notification(&notification).unwrap();

        // 保存された通知を取得
        let saved_notifications = history_manager.get_all_notifications().unwrap();
        assert_eq!(saved_notifications.len(), 1);
        assert_eq!(saved_notifications[0].id, "test-notification-id");
        assert_eq!(saved_notifications[0].title, "Test notification");
        assert_eq!(saved_notifications[0].repository, "test/repo");
        assert_eq!(saved_notifications[0].reason, "review_requested");
        assert_eq!(saved_notifications[0].subject_type, "PullRequest");
        assert!(!saved_notifications[0].is_read);

        // 重複通知の保存を試みる（保存されないことを確認）
        history_manager.save_notification(&notification).unwrap();
        let duplicate_check = history_manager.get_all_notifications().unwrap();
        assert_eq!(duplicate_check.len(), 1); // 同じ通知は保存されない

        // 通知を既読にする
        history_manager
            .mark_as_read("test-notification-id")
            .unwrap();
        let updated_notifications = history_manager.get_all_notifications().unwrap();
        assert!(updated_notifications[0].is_read);

        // 未読通知の取得
        let unread_notifications = history_manager.get_unread_notifications().unwrap();
        assert_eq!(unread_notifications.len(), 0); // 既読にしたので未読は0

        // リポジトリでフィルタリング
        let repo_notifications = history_manager
            .get_notifications_by_repository("test")
            .unwrap();
        assert_eq!(repo_notifications.len(), 1);

        // 通知理由でフィルタリング
        let reason_notifications = history_manager
            .get_notifications_by_reason("review_requested")
            .unwrap();
        assert_eq!(reason_notifications.len(), 1);

        // 通知タイプでフィルタリング
        let type_notifications = history_manager
            .get_notifications_by_subject_type("PullRequest")
            .unwrap();
        assert_eq!(type_notifications.len(), 1);

        // 通知のカウント確認
        assert_eq!(history_manager.get_notification_count().unwrap(), 1);

        // 通知が存在するか確認
        assert!(
            history_manager
                .notification_exists("test-notification-id")
                .unwrap()
        );
        assert!(
            !history_manager
                .notification_exists("non-existent-id")
                .unwrap()
        );
    }
}
