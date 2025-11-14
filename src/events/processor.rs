use crate::config::NotificationConfig;
use crate::errors::AppError;
use crate::github::types::*;
use crate::notification::{NotificationManager, UserNotification};
use chrono::{DateTime, Utc};
use tracing::{info, warn};

/// イベントプロセッサー
pub struct EventProcessor {
    notification_manager: NotificationManager,
    config: NotificationConfig,
}

impl EventProcessor {
    /// 新しいイベントプロセッサーを作成
    pub fn new(notification_manager: NotificationManager, config: NotificationConfig) -> Self {
        Self {
            notification_manager,
            config,
        }
    }

    /// GitHub通知イベントを処理
    pub async fn process_github_notification(
        &self,
        github_notification: &Notification,
    ) -> Result<Option<UserNotification>, AppError> {
        info!(
            "Processing GitHub notification event: {}",
            github_notification.id
        );

        // 通知マネージャーを使用して処理
        self.notification_manager
            .process_github_notification(github_notification)
    }

    /// Webhookイベントを処理
    pub async fn process_webhook_event(
        &self,
        webhook_event: &WebhookEvent,
    ) -> Result<Vec<UserNotification>, AppError> {
        info!("Processing webhook event: {}", webhook_event.event_type);

        let mut processed_notifications = Vec::new();

        match &webhook_event.event_type {
            WebhookEventType::PullRequest => {
                if let WebhookPayload::PullRequestOpened { pull_request } = &webhook_event.payload
                    && let Some(notification) = self
                        .create_pull_request_notification(pull_request, webhook_event)
                        .await?
                {
                    processed_notifications.push(notification);
                }
            }
            WebhookEventType::PullRequestReview => {
                // PRレビューイベントの処理
                // 簡略化のため実装は割愛
            }
            WebhookEventType::IssueComment => {
                // Issueコメントイベントの処理
                // 簡略化のため実装は割愛
            }
            _ => {
                // その他のイベントは無視
                warn!(
                    "Unhandled webhook event type: {:?}",
                    webhook_event.event_type
                );
            }
        }

        Ok(processed_notifications)
    }

    /// プルリクエスト通知を作成
    async fn create_pull_request_notification(
        &self,
        pull_request: &PullRequest,
        webhook_event: &WebhookEvent,
    ) -> Result<Option<UserNotification>, AppError> {
        // PR作成者やレビューリクエストされたユーザー向けの通知を作成
        // 簡略化のため、基本的な通知のみ作成

        let title = format!("Pull Request: {}", pull_request.title);

        let body = format!(
            "Repository: {}\nAuthor: {}\nState: {}",
            webhook_event.repository.full_name, pull_request.user.login, pull_request.state
        );

        let user_notification = UserNotification {
            id: format!("pr_{}", pull_request.number),
            title,
            body,
            icon_url: Some(pull_request.user.avatar_url.clone().unwrap_or_default()),
            priority: calculate_pr_priority(pull_request),
            category: crate::notification::types::NotificationCategory::PullRequest,
            url: pull_request.html_url.clone(),
            created_at: pull_request.created_at,
            updated_at: pull_request.updated_at,
            is_read: false,
            read_at: None,
            actions: create_pr_actions(pull_request),
            metadata: create_pr_metadata(pull_request),
        };

        // フィルタリングチェック
        if should_show_pr_notification(&user_notification, &self.config) {
            Ok(Some(user_notification))
        } else {
            Ok(None)
        }
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config;
    }

    /// 統計情報を取得
    pub fn get_statistics(
        &self,
    ) -> Result<crate::notification::manager::NotificationStatistics, AppError> {
        self.notification_manager.get_statistics()
    }
}

/// 重要度を計算
fn calculate_pr_priority(
    pull_request: &PullRequest,
) -> crate::notification::types::NotificationPriority {
    // ドラフトPRは低優先度
    if pull_request.draft == Some(true) {
        return crate::notification::types::NotificationPriority::Low;
    }

    // レビューリクエストがある場合は高優先度
    // 簡略化のため、常に通常優先度を返す
    crate::notification::types::NotificationPriority::Normal
}

/// PRアクションを作成
fn create_pr_actions(
    pull_request: &PullRequest,
) -> Vec<crate::notification::types::NotificationAction> {
    let mut actions = Vec::new();

    // URLを開くアクション
    actions.push(crate::notification::types::NotificationAction {
        name: "Open PR".to_string(),
        url: pull_request.html_url.clone(),
        action_type: crate::notification::types::NotificationActionType::OpenUrl,
    });

    actions
}

/// PRメタデータを作成
fn create_pr_metadata(pull_request: &PullRequest) -> serde_json::Value {
    serde_json::json!({
        "pr_number": pull_request.number,
        "pr_state": pull_request.state,
        "pr_draft": pull_request.draft,
        "pr_merged": pull_request.merged,
        "author": pull_request.user.login,
        "base_branch": pull_request.base.ref_name,
        "head_branch": pull_request.head.ref_name,
    })
}

/// PR通知を表示すべきかチェック
fn should_show_pr_notification(
    _notification: &UserNotification,
    _config: &NotificationConfig,
) -> bool {
    // フィルタ設定に基づいて表示判定
    // 簡略化のため、常に表示する
    true
}

/// 通知ハンドラー（レガシーコンパチビリティ）
pub struct NotificationHandler {
    processor: EventProcessor,
}

impl NotificationHandler {
    /// 新しい通知ハンドラーを作成
    pub fn new(notification_manager: NotificationManager, config: NotificationConfig) -> Self {
        Self {
            processor: EventProcessor::new(notification_manager, config),
        }
    }

    /// 通知を処理して表示
    pub async fn handle_notification(
        &self,
        github_notification: &Notification,
    ) -> Result<Option<UserNotification>, AppError> {
        self.processor
            .process_github_notification(github_notification)
            .await
    }

    /// 通知のタイトルを作成
    pub fn create_notification_title(notification: &Notification) -> String {
        let reason_text = get_reason_display_text(&notification.reason.to_string());
        let repo_name = if notification.repository.r#private {
            format!("🔒 {}", notification.repository.full_name)
        } else {
            notification.repository.full_name.clone()
        };

        format!("{} - {}", repo_name, reason_text)
    }

    /// 通知本文を作成
    pub fn create_notification_body(notification: &Notification) -> String {
        let time_ago_text = format_time_ago(&notification.updated_at);
        let url = notification.html_url.as_ref().unwrap_or(&notification.url);

        format!(
            "{}\n\nRepository: {} | Type: {} | Updated: {}\nURL: {}",
            notification.subject.title,
            notification.repository.full_name,
            notification.subject.subject_type,
            time_ago_text,
            url
        )
    }
}

/// リーズン表示テキストを取得
fn get_reason_display_text(reason: &str) -> String {
    match reason {
        "assign" => "_assigned to you_".to_string(),
        "author" => "authored by you".to_string(),
        "comment" => "commented on".to_string(),
        "invitation" => "invited you".to_string(),
        "manual" => "mentioned you".to_string(),
        "mention" => "mentioned you".to_string(),
        "review_requested" => "_Review Requested_".to_string(),
        "security_alert" => "_Security Alert_".to_string(),
        "state_change" => "state changed".to_string(),
        "subscribed" => "subscribed".to_string(),
        "team_mention" => "team mentioned".to_string(),
        _ => "unknown".to_string(),
    }
}

/// 時間差分テキストを作成
fn format_time_ago(updated_at: &DateTime<Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*updated_at);

    if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NotificationConfig;
    use chrono::Utc;

    #[test]
    fn test_event_processor_new() {
        let storage =
            Box::new(crate::notification::manager::InMemoryNotificationStorage::default());
        let notification_manager = crate::notification::manager::NotificationManager::new(
            storage,
            crate::notification::types::NotificationConfig::default(),
        );
        let config = crate::config::NotificationConfig::default();

        let processor = EventProcessor::new(notification_manager, config);
        assert!(processor.get_statistics().is_ok());
    }

    #[test]
    fn test_notification_handler_new() {
        let storage =
            Box::new(crate::notification::manager::InMemoryNotificationStorage::default());
        let notification_manager = crate::notification::manager::NotificationManager::new(
            storage,
            crate::notification::types::NotificationConfig::default(),
        );
        let config = crate::config::NotificationConfig::default();

        let handler = NotificationHandler::new(notification_manager, config);
        assert!(handler.processor.get_statistics().is_ok());
    }

    #[test]
    fn test_reason_display_text() {
        assert_eq!(
            get_reason_display_text("review_requested"),
            "_Review Requested_"
        );
        assert_eq!(get_reason_display_text("mention"), "mentioned you");
        assert_eq!(get_reason_display_text("comment"), "commented on");
        assert_eq!(get_reason_display_text("assign"), "_assigned to you_");
        assert_eq!(get_reason_display_text("unknown"), "unknown");
    }

    #[test]
    fn test_format_time_ago() {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let one_day_ago = now - chrono::Duration::days(1);
        let one_minute_ago = now - chrono::Duration::minutes(1);

        let result = format_time_ago(&one_hour_ago);
        assert!(result.contains("1 hours ago"));

        let result = format_time_ago(&one_day_ago);
        assert!(result.contains("1 days ago"));

        let result = format_time_ago(&one_minute_ago);
        assert!(result.contains("1 minutes ago"));

        let result = format_time_ago(&now);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_format_subject_kind() {
        use crate::utils::format_subject_kind;
        assert_eq!(format_subject_kind("PullRequest"), "PR");
        assert_eq!(format_subject_kind("Issue"), "Issue");
        assert_eq!(format_subject_kind("Commit"), "Commit");
        assert_eq!(format_subject_kind("Release"), "Release");
        assert_eq!(format_subject_kind("Unknown"), "Unknown");
    }

    #[test]
    fn test_event_filter_new() {
        let config = NotificationConfig::default();
        let filter = EventFilter::new(config);

        assert!(filter.config.filters.exclude_private_repos == false);
        assert!(filter.config.filters.exclude_fork_repos == false);
    }

    #[test]
    fn test_event_filter_should_process_notification() {
        let config = NotificationConfig::default();
        let filter = EventFilter::new(config);

        let github_notification = create_test_github_notification();

        // デフォルト設定では通知を処理するべき
        assert!(filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_exclude_private_repository() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.exclude_private_repos = true;
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.repository.r#private = true;

        // プライベートリポジトリを除外する設定では処理しない
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_exclude_repository() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.exclude_repositories = vec!["test/repo".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.repository.full_name = "test/repo".to_string();

        // 除外リストにあるリポジトリの通知は処理しない
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_include_repository() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.include_repositories = vec!["test/repo".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.repository.full_name = "test/repo".to_string();

        // 含めるリストにあるリポジトリの通知は処理する
        assert!(filter.should_process_notification(&github_notification));

        // 含めるリストにないリポジトリの通知は処理しない
        github_notification.repository.full_name = "other/repo".to_string();
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_exclude_reason() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.exclude_reasons = vec!["review_requested".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.reason = NotificationReason::ReviewRequested;

        // 除外リストにある理由の通知は処理しない
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_include_reason() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.include_reasons = vec!["review_requested".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.reason = NotificationReason::ReviewRequested;

        // 含めるリストにある理由の通知は処理する
        assert!(filter.should_process_notification(&github_notification));

        // 含めるリストにない理由の通知は処理しない
        github_notification.reason = NotificationReason::Comment;
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_exclude_subject_type() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.exclude_subject_types = vec!["PullRequest".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.subject.subject_type = "PullRequest".to_string();

        // 除外リストにあるタイプの通知は処理しない
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_event_filter_include_subject_type() {
        let mut config = crate::config::NotificationConfig::default();
        config.filters.include_subject_types = vec!["PullRequest".to_string()];
        let filter = EventFilter::new(config);

        let mut github_notification = create_test_github_notification();
        github_notification.subject.subject_type = "PullRequest".to_string();

        // 含めるリストにあるタイプの通知は処理する
        assert!(filter.should_process_notification(&github_notification));

        // 含めるリストにないタイプの通知は処理しない
        github_notification.subject.subject_type = "Issue".to_string();
        assert!(!filter.should_process_notification(&github_notification));
    }

    #[test]
    fn test_notification_handler_create_title() {
        let mut github_notification = create_test_github_notification();
        github_notification.repository.r#private = false;
        github_notification.reason = NotificationReason::ReviewRequested;

        let title = NotificationHandler::create_notification_title(&github_notification);
        assert!(title.contains("test/repo"));
        assert!(title.contains("_Review Requested_"));
    }

    #[test]
    fn test_notification_handler_create_title_private_repo() {
        let mut github_notification = create_test_github_notification();
        github_notification.repository.r#private = true;
        github_notification.reason = NotificationReason::Comment;

        let title = NotificationHandler::create_notification_title(&github_notification);
        assert!(title.contains("🔒 test/repo"));
        assert!(title.contains("commented on"));
    }

    #[test]
    fn test_notification_handler_create_body() {
        let github_notification = create_test_github_notification();
        let body = NotificationHandler::create_notification_body(&github_notification);

        assert!(body.contains("Test Title"));
        assert!(body.contains("test/repo"));
        assert!(body.contains("PullRequest"));
        assert!(body.contains("URL:"));
    }

    // テスト用のGitHub通知を作成するヘルパー関数
    fn create_test_github_notification() -> Notification {
        Notification {
            id: "12345".to_string(),
            repository: Repository {
                id: 67890,
                full_name: "test/repo".to_string(),
                owner: User {
                    id: 12345,
                    login: "testuser".to_string(),
                    avatar_url: None,
                    html_url: "https://github.com/testuser".to_string(),
                    r#type: "User".to_string(),
                    site_admin: None,
                },
                description: Some("A test repository".to_string()),
                html_url: "https://github.com/test/repo".to_string(),
                r#private: false,
                fork: false,
                parent: None,
                template_repository: None,
                default_branch: "main".to_string(),
                master_branch: None,
                permissions: None,
                is_template: None,
                network_count: None,
                subscribers_count: None,
            },
            subject: NotificationSubject {
                title: "Test Title".to_string(),
                subject_type: "PullRequest".to_string(),
                kind: "PullRequest".to_string(), // Add the missing kind field
                url: Some("https://github.com/test/repo/pull/1".to_string()),
                latest_comment_url: None,
                html_url: Some("https://github.com/test/repo/pull/1".to_string()),
            },
            reason: NotificationReason::ReviewRequested,
            unread: true,
            updated_at: Utc::now(),
            last_read_at: None,
            url: "https://github.com/test/repo/pull/1".to_string(),
            api_url: "https://api.github.com/notifications/threads/12345".to_string(),
            html_url: Some("https://github.com/test/repo/pull/1".to_string()),
        }
    }

    #[test]
    fn test_event_filter_update_config() {
        let config = NotificationConfig::default();
        let mut filter = EventFilter::new(config);

        // 設定を更新
        let new_config = NotificationConfig::default();
        filter.update_config(new_config);

        assert!(filter.config.filters.exclude_private_repos == false);
    }

    #[test]
    fn test_notification_handler_update_config() {
        let storage =
            Box::new(crate::notification::manager::InMemoryNotificationStorage::default());
        let notification_manager = crate::notification::manager::NotificationManager::new(
            storage,
            crate::notification::types::NotificationConfig::default(),
        );
        let config = crate::config::NotificationConfig::default();
        let mut handler = NotificationHandler::new(notification_manager, config);

        // 設定を更新
        let new_config = crate::config::NotificationConfig {
            mark_as_read_on_notify: true,
            persistent_notifications: false,
            batch: Default::default(),
            filters: Default::default(),
        };
        handler.processor.update_config(new_config);

        assert!(handler.processor.config.mark_as_read_on_notify == true);
    }
}

/// イベントフィルタ
pub struct EventFilter {
    config: NotificationConfig,
}

impl EventFilter {
    /// 新しいイベントフィルタを作成
    pub fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    /// 通知をフィルタリング
    pub fn should_process_notification(&self, notification: &Notification) -> bool {
        // リポジトリ除外チェック
        if self
            .config
            .filters
            .exclude_repositories
            .contains(&notification.repository.full_name)
        {
            return false;
        }

        // リポジトリ含めチェック
        if !self.config.filters.include_repositories.is_empty()
            && !self
                .config
                .filters
                .include_repositories
                .contains(&notification.repository.full_name)
        {
            return false;
        }

        // プライベートリポジトリ除外チェック
        if self.config.filters.exclude_private_repos && notification.repository.r#private {
            return false;
        }

        // 通知理由除外チェック
        if self
            .config
            .filters
            .exclude_reasons
            .contains(&notification.reason.to_string())
        {
            return false;
        }

        // 通知理由含めチェック
        if !self.config.filters.include_reasons.is_empty()
            && !self
                .config
                .filters
                .include_reasons
                .contains(&notification.reason.to_string())
        {
            return false;
        }

        // 通知タイプ除外チェック
        if self
            .config
            .filters
            .exclude_subject_types
            .contains(&notification.subject.subject_type)
        {
            return false;
        }

        // 通知タイプ含めチェック
        if !self.config.filters.include_subject_types.is_empty()
            && !self
                .config
                .filters
                .include_subject_types
                .contains(&notification.subject.subject_type)
        {
            return false;
        }

        true
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config;
    }
}
