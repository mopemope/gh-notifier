use crate::models::PersistedNotification;
use rusqlite::{Connection, Result as SqliteResult, params};
use std::path::Path;

/// 通知データを永続化するためのストレージ
pub struct NotificationStorage {
    conn: Connection,
}

impl NotificationStorage {
    /// 新しいNotificationStorageインスタンスを作成
    pub fn new(db_path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;

        // テーブルが存在しない場合は作成
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                url TEXT NOT NULL,
                repository TEXT NOT NULL,
                reason TEXT NOT NULL,
                subject_type TEXT NOT NULL,
                is_read BOOLEAN NOT NULL DEFAULT 0,
                received_at TEXT NOT NULL,
                marked_read_at TEXT
            )",
            params![],
        )?;

        Ok(NotificationStorage { conn })
    }

    /// 通知を保存
    pub fn save_notification(&self, notification: &PersistedNotification) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO notifications 
            (id, title, body, url, repository, reason, subject_type, is_read, received_at, marked_read_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                notification.id,
                notification.title,
                notification.body,
                notification.url,
                notification.repository,
                notification.reason,
                notification.subject_type,
                notification.is_read,
                notification.received_at,
                notification.marked_read_at
            ],
        )?;
        Ok(())
    }

    /// 未読通知をすべて取得
    pub fn get_unread_notifications(&self) -> SqliteResult<Vec<PersistedNotification>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, body, url, repository, reason, subject_type, 
                    is_read, received_at, marked_read_at 
             FROM notifications 
             WHERE is_read = 0 
             ORDER BY received_at DESC",
        )?;

        let notifications = stmt
            .query_map(params![], |row| {
                Ok(PersistedNotification {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    url: row.get(3)?,
                    repository: row.get(4)?,
                    reason: row.get(5)?,
                    subject_type: row.get(6)?,
                    is_read: row.get(7)?,
                    received_at: row.get(8)?,
                    marked_read_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<PersistedNotification>, _>>()?;

        Ok(notifications)
    }

    /// すべての通知を取得
    pub fn get_all_notifications(&self) -> SqliteResult<Vec<PersistedNotification>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, body, url, repository, reason, subject_type, 
                    is_read, received_at, marked_read_at 
             FROM notifications 
             ORDER BY received_at DESC",
        )?;

        let notifications = stmt
            .query_map(params![], |row| {
                Ok(PersistedNotification {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    url: row.get(3)?,
                    repository: row.get(4)?,
                    reason: row.get(5)?,
                    subject_type: row.get(6)?,
                    is_read: row.get(7)?,
                    received_at: row.get(8)?,
                    marked_read_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<PersistedNotification>, _>>()?;

        Ok(notifications)
    }

    /// 通知を既読にする
    pub fn mark_as_read(&self, notification_id: &str) -> SqliteResult<()> {
        use chrono::Utc;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE notifications SET is_read = 1, marked_read_at = ?1 WHERE id = ?2",
            params![now, notification_id],
        )?;
        Ok(())
    }

    /// すべての通知を既読にする
    pub fn mark_all_as_read(&self) -> SqliteResult<()> {
        use chrono::Utc;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE notifications SET is_read = 1, marked_read_at = ?1 WHERE is_read = 0",
            params![now],
        )?;
        Ok(())
    }

    /// 特定の通知を削除
    pub fn delete_notification(&self, notification_id: &str) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM notifications WHERE id = ?1",
            params![notification_id],
        )?;
        Ok(())
    }

    /// 通知が既に保存されているか確認
    pub fn notification_exists(&self, notification_id: &str) -> SqliteResult<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM notifications WHERE id = ?1)",
            params![notification_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// 保存された通知の数を取得
    pub fn get_notification_count(&self) -> SqliteResult<u32> {
        let count: u32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM notifications", params![], |row| {
                    row.get(0)
                })?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_notification_storage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let storage = NotificationStorage::new(&db_path).unwrap();

        // 通知を保存
        let notification = PersistedNotification {
            id: "test_id_1".to_string(),
            title: "Test Title".to_string(),
            body: "Test Body".to_string(),
            url: "https://example.com".to_string(),
            repository: "test/repo".to_string(),
            reason: "review_requested".to_string(),
            subject_type: "PullRequest".to_string(),
            is_read: false,
            received_at: "2023-01-01T00:00:00Z".to_string(),
            marked_read_at: None,
        };

        storage.save_notification(&notification).unwrap();

        // 保存された通知を取得
        let saved_notifications = storage.get_all_notifications().unwrap();
        assert_eq!(saved_notifications.len(), 1);
        assert_eq!(saved_notifications[0].id, "test_id_1");
        assert_eq!(saved_notifications[0].title, "Test Title");
        assert_eq!(saved_notifications[0].is_read, false);

        // 通知が既に存在するか確認
        assert!(storage.notification_exists("test_id_1").unwrap());
        assert!(!storage.notification_exists("non_existent_id").unwrap());

        // 通知を既読にする
        storage.mark_as_read("test_id_1").unwrap();

        let updated_notifications = storage.get_all_notifications().unwrap();
        assert_eq!(updated_notifications[0].is_read, true);
        assert!(updated_notifications[0].marked_read_at.is_some());

        // 通知のカウントを確認
        assert_eq!(storage.get_notification_count().unwrap(), 1);

        // 通知を削除
        storage.delete_notification("test_id_1").unwrap();
        assert_eq!(storage.get_notification_count().unwrap(), 0);
    }

    #[test]
    fn test_unread_notifications() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let storage = NotificationStorage::new(&db_path).unwrap();

        // 未読通知を作成
        let unread_notification = PersistedNotification {
            id: "test_id_unread".to_string(),
            title: "Unread Title".to_string(),
            body: "Unread Body".to_string(),
            url: "https://example.com/unread".to_string(),
            repository: "test/repo".to_string(),
            reason: "review_requested".to_string(),
            subject_type: "PullRequest".to_string(),
            is_read: false,
            received_at: "2023-01-01T00:00:00Z".to_string(),
            marked_read_at: None,
        };

        // 既読通知を作成
        let read_notification = PersistedNotification {
            id: "test_id_read".to_string(),
            title: "Read Title".to_string(),
            body: "Read Body".to_string(),
            url: "https://example.com/read".to_string(),
            repository: "test/repo".to_string(),
            reason: "mention".to_string(),
            subject_type: "Issue".to_string(),
            is_read: true,
            received_at: "2023-01-01T00:00:00Z".to_string(),
            marked_read_at: Some("2023-01-01T01:00:00Z".to_string()),
        };

        storage.save_notification(&unread_notification).unwrap();
        storage.save_notification(&read_notification).unwrap();

        // 未読通知のみを取得
        let unread_notifications = storage.get_unread_notifications().unwrap();
        assert_eq!(unread_notifications.len(), 1);
        assert_eq!(unread_notifications[0].id, "test_id_unread");
        assert_eq!(unread_notifications[0].is_read, false);

        // すべての通知を取得
        let all_notifications = storage.get_all_notifications().unwrap();
        assert_eq!(all_notifications.len(), 2);
    }
}
