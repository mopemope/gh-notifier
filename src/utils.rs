use crate::config::Config;
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use std::path::PathBuf;
use tracing::warn;

/// データベースパスを取得
pub fn get_database_path() -> PathBuf {
    // Try to get config directory, fallback to current directory
    let mut db_path = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"));
    db_path.push("gh-notifier");
    db_path.push("gh-notifier.db");
    db_path
}

/// 設定ファイルのパスを取得
pub fn get_config_path() -> PathBuf {
    let mut config_path = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"));
    config_path.push("gh-notifier");
    config_path.push("config.toml");
    config_path
}

/// ログファイルのパスを取得
pub fn get_log_file_path(config: &Config) -> PathBuf {
    if let Some(log_file_path) = config.log_file_path() {
        PathBuf::from(log_file_path)
    } else {
        let mut log_path = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"));
        log_path.push("gh-notifier");
        log_path.push("logs");
        std::fs::create_dir_all(&log_path).unwrap_or_default();
        log_path.push("gh-notifier.log");
        log_path
    }
}

/// ISO 8601形式の日時文字列をDateTime<Utc>に変換
pub fn parse_iso8601(date_str: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let dt = DateTime::parse_from_rfc3339(date_str)?;
    Ok(dt.timestamp() as u64)
}

/// 時間差分を人間 readable な形式に変換
pub fn format_time_ago(updated_at: &DateTime<Utc>) -> String {
    let now = Utc::now();
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

/// 通知理由をユーザー向けテキストに変換
pub fn get_reason_display_text(reason: &str) -> String {
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
        _ => reason.to_string(),
    }
}

/// 件名タイプをフォーマット
pub fn format_subject_kind(subject_type: &str) -> String {
    match subject_type {
        "PullRequest" => "PR".to_string(),
        "Issue" => "Issue".to_string(),
        "Commit" => "Commit".to_string(),
        "Release" => "Release".to_string(),
        _ => subject_type.to_string(),
    }
}

/// プライベートリポジトリかどうかをチェック
pub fn is_private_repository(repository_name: &str) -> bool {
    // 簡略化：リポジトリ名に🔒が含まれているかで判断
    repository_name.starts_with("🔒")
}

/// GitHub Personal Access Tokenの形式を検証
pub fn validate_github_token(token: &SecretString) -> Result<(), AppError> {
    let token_str = token.expose_secret();

    // 基本的な形式チェック
    if token_str.is_empty() {
        return Err(AppError::GitHub(
            crate::errors::GitHubError::AuthenticationError,
        ));
    }

    // GitHubのPersonal Access Tokenは通常ghp_で始まる
    if !token_str.starts_with("ghp_") && !token_str.starts_with("github_pat_") {
        warn!("Token doesn't start with expected prefix (ghp_ or github_pat_)");
    }

    // 長さチェック（最低限）
    if token_str.len() < 20 {
        return Err(AppError::GitHub(
            crate::errors::GitHubError::AuthenticationError,
        ));
    }

    Ok(())
}

/// URLのバリデーション
pub fn validate_url(url: &str) -> Result<(), AppError> {
    if url.is_empty() {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "URL cannot be empty".to_string(),
        }));
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "URL must start with http:// or https://".to_string(),
        }));
    }

    Ok(())
}

/// リトライ回数とインターバルのバリデーション
pub fn validate_retry_config(retry_count: u32, retry_interval_sec: u64) -> Result<(), AppError> {
    if retry_count > 10 {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "Retry count cannot exceed 10".to_string(),
        }));
    }

    if retry_interval_sec > 300 {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "Retry interval cannot exceed 300 seconds".to_string(),
        }));
    }

    Ok(())
}

/// ポーリング間隔のバリデーション
pub fn validate_poll_interval(interval_sec: u64) -> Result<(), AppError> {
    if interval_sec < 5 {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "Polling interval cannot be less than 5 seconds".to_string(),
        }));
    }

    if interval_sec > 3600 {
        return Err(AppError::GitHub(crate::errors::GitHubError::ApiError {
            message: "Polling interval cannot exceed 3600 seconds".to_string(),
        }));
    }

    Ok(())
}

/// 通知バッチサイズのバリデーション
pub fn validate_batch_size(batch_size: usize) -> Result<(), AppError> {
    if batch_size > 100 {
        return Err(AppError::Notification(
            crate::errors::NotificationError::Generic {
                message: "Batch size cannot exceed 100".to_string(),
            },
        ));
    }

    Ok(())
}

/// 通知バッチ間隔のバリデーション
pub fn validate_batch_interval(batch_interval_sec: u64) -> Result<(), AppError> {
    if batch_interval_sec > 300 {
        return Err(AppError::Notification(
            crate::errors::NotificationError::Generic {
                message: "Batch interval cannot exceed 300 seconds".to_string(),
            },
        ));
    }

    Ok(())
}

/// フィルタ設定のバリデーション
pub fn validate_notification_filter(
    filter: &crate::notification::types::NotificationFilter,
) -> Result<(), AppError> {
    // フォークリポジトリとプライベートリポジトリの両方が除外されているかチェック
    if filter.exclude_fork_repos && filter.exclude_private_repos {
        // これは許容するが、ログを出す
        warn!(
            "Both fork repos and private repos are excluded - this may result in very few notifications"
        );
    }

    // 含めるリストと除外リストが競合していないかチェック
    if !filter.include_repositories.is_empty() && !filter.exclude_repositories.is_empty() {
        let common: Vec<_> = filter
            .include_repositories
            .iter()
            .filter(|repo| filter.exclude_repositories.contains(repo))
            .collect();

        if !common.is_empty() {
            warn!(
                "Some repositories are in both include and exclude lists: {:?}",
                common
            );
        }
    }

    Ok(())
}

/// 環境変数から設定を読み込むヘルパー
pub fn get_github_token_from_env() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_NOTIFIER_TOKEN"))
        .ok()
}

/// User-Agent文字列を生成
pub fn create_user_agent() -> String {
    format!(
        "gh-notifier/{} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::var("GH_NOTIFIER_USER_AGENT").unwrap_or_else(|_| "unknown".to_string())
    )
}

/// アプリケーションのバージョン情報を取得
pub fn get_version_info() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit_hash = option_env!("GIT_COMMIT_HASH").unwrap_or("unknown");
    let build_date = std::env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string());

    format!(
        "gh-notifier {}\ncommit: {}\nbuild date: {}",
        version, commit_hash, build_date
    )
}

/// システム情報の取得
pub fn get_system_info() -> String {
    format!(
        "OS: {}\nArch: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

/// 設定の整合性チェック
pub fn validate_config(config: &Config) -> Result<(), AppError> {
    // GitHub設定のバリデーション
    if let Some(token) = &config.github_token() {
        validate_github_token(&SecretString::new(token.to_string()))?;
    }

    // ポーリング設定のバリデーション
    validate_poll_interval(config.poll_interval_sec())?;

    // 通知設定のバリデーション
    validate_batch_size(config.notification_batch_config().batch_size)?;
    validate_batch_interval(config.notification_batch_config().batch_interval_sec)?;
    // Convert config NotificationFilter to notification types NotificationFilter
    let notification_filter = crate::notification::types::NotificationFilter {
        exclude_repositories: config.notification_filters().exclude_repositories.clone(),
        exclude_reasons: config.notification_filters().exclude_reasons.clone(),
        include_repositories: config.notification_filters().include_repositories.clone(),
        include_organizations: config.notification_filters().include_organizations.clone(),
        exclude_organizations: config.notification_filters().exclude_organizations.clone(),
        exclude_private_repos: config.notification_filters().exclude_private_repos,
        exclude_fork_repos: config.notification_filters().exclude_fork_repos,
        include_subject_types: config.notification_filters().include_subject_types.clone(),
        exclude_subject_types: config.notification_filters().exclude_subject_types.clone(),
        include_reasons: config.notification_filters().include_reasons.clone(),
        title_contains: config.notification_filters().title_contains.clone(),
        title_not_contains: Vec::new(),  // Default empty
        repository_contains: Vec::new(), // Default empty
        exclude_participating: false,    // Default false
        minimum_updated_time: None,      // Default None
        exclude_draft_prs: false,        // Default false
    };
    validate_notification_filter(&notification_filter)?;

    // API設定のバリデーション
    validate_retry_config(config.github.retry_count, config.github.retry_interval_sec)?;

    Ok(())
}

/// デバッグモードかどうかをチェック
pub fn is_debug_mode() -> bool {
    std::env::var("RUST_LOG")
        .map(|log| log.contains("debug"))
        .unwrap_or(false)
}

/// テストモードかどうかをチェック
pub fn is_test_mode() -> bool {
    std::env::var("GH_NOTIFIER_TEST")
        .map(|test| test == "1" || test.to_lowercase() == "true")
        .unwrap_or(false)
}

/// 現在の設定をマスクしてログ出力（セキュリティ対策）
pub fn mask_sensitive_config(config: &Config) -> String {
    format!(
        "Config {{ \
         poll_interval_sec: {}, \
         mark_as_read_on_notify: {}, \
         github_api_url: {}, \
         log_level: {}, \
         persistent_notifications: {}, \
         api_enabled: {}, \
         api_port: {} \
         }}",
        config.poll_interval_sec(),
        config.mark_as_read_on_notify(),
        config.github.api_base_url,
        config.log_level(),
        config.persistent_notifications(),
        config.api_enabled(),
        config.api_port(),
    )
}
