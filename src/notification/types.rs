use crate::github::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通知の重要度
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
pub enum NotificationPriority {
    /// 低優先度
    Low,
    /// 通常優先度
    #[default]
    Normal,
    /// 高優先度
    High,
    /// 緊急
    Critical,
}

/// 通知アクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    /// アクション名
    pub name: String,
    /// アクションURL
    pub url: String,
    /// アクションタイプ
    pub action_type: NotificationActionType,
}

/// 通知アクションタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationActionType {
    /// URLを開く
    OpenUrl,
    /// 既読にする
    MarkAsRead,
    /// コメントする
    Comment,
    /// レビューする
    Review,
}

/// ユーザー向け通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNotification {
    /// 通知ID
    pub id: String,
    /// 通知タイトル
    pub title: String,
    /// 通知本文
    pub body: String,
    /// 通知アイコンURL
    pub icon_url: Option<String>,
    /// 通知重要度
    pub priority: NotificationPriority,
    /// 通知カテゴリ
    pub category: NotificationCategory,
    /// 関連URL
    pub url: String,
    /// 作成日時
    pub created_at: DateTime<Utc>,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
    /// 既読状態
    pub is_read: bool,
    /// 既読にされた日時
    pub read_at: Option<DateTime<Utc>>,
    /// 通知アクション
    pub actions: Vec<NotificationAction>,
    /// 追加のメタデータ
    pub metadata: serde_json::Value,
}

/// 通知カテゴリ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum NotificationCategory {
    /// プルリクエスト関連
    PullRequest,
    /// Issue関連
    Issue,
    /// コメント関連
    Comment,
    /// レビューコメント関連
    ReviewComment,
    /// リポジトリ関連
    Repository,
    /// その他
    #[default]
    Other,
}

/// 通知フィルタ条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationFilter {
    /// 除外するリポジトリ
    pub exclude_repositories: Vec<String>,
    /// 除外する通知理由
    pub exclude_reasons: Vec<String>,
    /// 含めるリポジトリ
    pub include_repositories: Vec<String>,
    /// 含める組織
    pub include_organizations: Vec<String>,
    /// 除外する組織
    pub exclude_organizations: Vec<String>,
    /// プライベートリポジトリを除外
    pub exclude_private_repos: bool,
    /// フォークリポジTORIを除外
    pub exclude_fork_repos: bool,
    /// 含める通知タイプ
    pub include_subject_types: Vec<String>,
    /// 除外する通知タイプ
    pub exclude_subject_types: Vec<String>,
    /// 含める通知理由
    pub include_reasons: Vec<String>,
    /// タイトルに含めるキーワード
    pub title_contains: Vec<String>,
    /// タイトルに含めないキーワード
    pub title_not_contains: Vec<String>,
    /// リポジトリ名に含めるキーワード
    pub repository_contains: Vec<String>,
    /// 参加したスレッドを除外
    pub exclude_participating: bool,
    /// 最小更新時間
    pub minimum_updated_time: Option<String>,
    /// ドラフトPRを除外
    pub exclude_draft_prs: bool,
}

/// 通知バッチ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBatchConfig {
    /// バッチサイズ
    pub batch_size: usize,
    /// バッチ間隔（秒）
    pub batch_interval_sec: u64,
}

impl Default for NotificationBatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 0, // デフォルトではバッチ処理を無効化
            batch_interval_sec: 30,
        }
    }
}

/// 通知設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    /// 既読設定
    pub mark_as_read_on_notify: bool,
    /// バッチ設定
    pub batch: NotificationBatchConfig,
    /// フィルタ設定
    pub filter: NotificationFilter,
}

impl NotificationConfig {
    /// 既読設定を取得
    pub fn mark_as_read_on_notify(&self) -> bool {
        self.mark_as_read_on_notify
    }
}

/// 通知コンバーター
pub struct NotificationConverter;

impl NotificationConverter {
    /// GitHub通知をユーザー通知に変換
    pub fn convert_github_notification(
        github_notification: &Notification,
        config: &NotificationConfig,
    ) -> Result<UserNotification, crate::errors::NotificationError> {
        // フィルタリングチェック
        if !Self::should_show_notification(github_notification, &config.filter) {
            return Err(crate::errors::NotificationError::FilterError {
                reason: "Notification filtered out".to_string(),
            });
        }

        let priority = Self::calculate_priority(github_notification);
        let category = Self::categorize_notification(github_notification);
        let actions = Self::create_actions(github_notification);
        let metadata = Self::create_metadata(github_notification);

        Ok(UserNotification {
            id: github_notification.id.clone(),
            title: Self::create_title(github_notification),
            body: Self::create_body(github_notification),
            icon_url: Self::get_icon_url(github_notification),
            priority,
            category,
            url: github_notification
                .html_url
                .clone()
                .unwrap_or_else(|| "https://github.com".to_string()),
            created_at: github_notification.updated_at,
            updated_at: github_notification.updated_at,
            is_read: !github_notification.unread,
            read_at: github_notification.last_read_at,
            actions,
            metadata,
        })
    }

    /// 通知が表示されるべきかチェック
    fn should_show_notification(notification: &Notification, filter: &NotificationFilter) -> bool {
        // リポジトリ除外チェック
        if filter
            .exclude_repositories
            .contains(&notification.repository.full_name)
        {
            return false;
        }

        // リポジトリ含めチェック
        if !filter.include_repositories.is_empty()
            && !filter
                .include_repositories
                .contains(&notification.repository.full_name)
        {
            return false;
        }

        // 組織除外チェック
        if !filter.exclude_organizations.is_empty() {
            // 組織名を取得してチェック（簡略化）
            let org_name = notification
                .repository
                .full_name
                .split('/')
                .next()
                .unwrap_or("");
            if filter.exclude_organizations.contains(&org_name.to_string()) {
                return false;
            }
        }

        // 組織含めチェック
        if !filter.include_organizations.is_empty() {
            let org_name = notification
                .repository
                .full_name
                .split('/')
                .next()
                .unwrap_or("");
            if !filter.include_organizations.contains(&org_name.to_string()) {
                return false;
            }
        }

        // プライベートリポジトリ除外チェック
        if filter.exclude_private_repos && notification.repository.r#private {
            return false;
        }

        // フォークリポジトリ除外チェック
        if filter.exclude_fork_repos && notification.repository.fork {
            return false;
        }

        // 通知理由除外チェック
        if filter
            .exclude_reasons
            .contains(&notification.reason.to_string())
        {
            return false;
        }

        // 通知理由含めチェック
        if !filter.include_reasons.is_empty()
            && !filter
                .include_reasons
                .contains(&notification.reason.to_string())
        {
            return false;
        }

        // 通知タイプ除外チェック
        if filter
            .exclude_subject_types
            .contains(&notification.subject.subject_type)
        {
            return false;
        }

        // 通知タイプ含めチェック
        if !filter.include_subject_types.is_empty()
            && !filter
                .include_subject_types
                .contains(&notification.subject.subject_type)
        {
            return false;
        }

        // 参加したスレッド除外チェック
        if filter.exclude_participating {
            // 簡略化：参加しているかどうかのチェックは実装しない
        }

        // ドラフトPR除外チェック
        if filter.exclude_draft_prs && notification.subject.subject_type == "PullRequest" {
            // ドラフトPRかどうかのチェックは簡略化
        }

        true
    }

    /// 重要度を計算
    fn calculate_priority(notification: &Notification) -> NotificationPriority {
        match notification.reason {
            crate::github::types::NotificationReason::ReviewRequested => NotificationPriority::High,
            crate::github::types::NotificationReason::Mention => NotificationPriority::High,
            crate::github::types::NotificationReason::Comment => NotificationPriority::Normal,
            _ => NotificationPriority::Low,
        }
    }

    /// 通知をカテゴリに分類
    fn categorize_notification(notification: &Notification) -> NotificationCategory {
        match notification.subject.subject_type.as_str() {
            "PullRequest" => NotificationCategory::PullRequest,
            "Issue" => NotificationCategory::Issue,
            "Commit" => NotificationCategory::Comment,
            _ => NotificationCategory::Other,
        }
    }

    /// 通知タイトルを作成
    fn create_title(notification: &Notification) -> String {
        format!(
            "{}: {}",
            notification.subject.subject_type, notification.subject.title
        )
    }

    /// 通知本文を作成
    fn create_body(notification: &Notification) -> String {
        format!(
            "Repository: {}\nReason: {}",
            notification.repository.full_name, notification.reason
        )
    }

    /// アイコンURLを取得
    fn get_icon_url(notification: &Notification) -> Option<String> {
        // 簡略化：実際のアイコンURL取得ロジック
        Some(format!("{}/favicon.ico", notification.repository.html_url))
    }

    /// アクションを作成
    fn create_actions(notification: &Notification) -> Vec<NotificationAction> {
        let mut actions = Vec::new();

        // URLを開くアクション
        actions.push(NotificationAction {
            name: "Open".to_string(),
            url: notification
                .html_url
                .clone()
                .unwrap_or_else(|| "https://github.com".to_string()),
            action_type: NotificationActionType::OpenUrl,
        });

        // 既読にするアクション
        if notification.unread {
            actions.push(NotificationAction {
                name: "Mark as Read".to_string(),
                url: format!(
                    "{}/notifications/threads/{}",
                    "https://api.github.com", notification.id
                ),
                action_type: NotificationActionType::MarkAsRead,
            });
        }

        actions
    }

    /// メタデータを作成
    fn create_metadata(notification: &Notification) -> serde_json::Value {
        serde_json::json!({
            "repository_id": notification.repository.id,
            "subject_type": notification.subject.subject_type,
            "reason": notification.reason.to_string(),
            "unread": notification.unread
        })
    }
}
