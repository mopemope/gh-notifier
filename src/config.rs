use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// GitHub API設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub Personal Access Token (Classic PAT)
    #[serde(default)]
    pub token: Option<String>,

    /// APIベースURL（省略可、デフォルト: https://api.github.com）
    #[serde(default = "default_github_api_url")]
    pub api_base_url: String,

    /// APIレートリミット exceeded時の再試行回数
    #[serde(default = "default_github_retry_count")]
    pub retry_count: u32,

    /// 再試行間隔（秒）
    #[serde(default = "default_github_retry_interval_sec")]
    pub retry_interval_sec: u64,
}

fn default_github_api_url() -> String {
    "https://api.github.com".to_string()
}

fn default_github_retry_count() -> u32 {
    3
}

fn default_github_retry_interval_sec() -> u64 {
    5
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            token: None,
            api_base_url: default_github_api_url(),
            retry_count: default_github_retry_count(),
            retry_interval_sec: default_github_retry_interval_sec(),
        }
    }
}

/// 通知設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// 通知表示時に通知を既読にするかどうか
    #[serde(default = "default_mark_as_read_on_notify")]
    pub mark_as_read_on_notify: bool,

    /// 通知を永続的に表示するかどうか（自動消去しない）
    #[serde(default = "default_persistent_notifications")]
    pub persistent_notifications: bool,

    /// 通知フィルタの設定
    #[serde(default)]
    pub filters: NotificationFilter,

    /// 通知バッチ処理の設定
    #[serde(default)]
    pub batch: NotificationBatchConfig,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            mark_as_read_on_notify: default_mark_as_read_on_notify(),
            persistent_notifications: default_persistent_notifications(),
            filters: Default::default(),
            batch: Default::default(),
        }
    }
}

/// ポーリング設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// ポーリング間隔（秒）
    #[serde(default = "default_poll_interval_sec")]
    pub interval_sec: u64,

    /// ポーリング処理のエラーハンドリング設定
    #[serde(default)]
    pub error_handling: PollingErrorHandlingConfig,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval_sec: default_poll_interval_sec(),
            error_handling: Default::default(),
        }
    }
}

/// APIサーバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// APIサーバーを有効にするかどうか
    #[serde(default = "default_api_enabled")]
    pub enabled: bool,

    /// APIサーバーのポート番号
    #[serde(default = "default_api_port")]
    pub port: u16,

    /// 既読通知を表示するかどうか
    #[serde(default = "default_show_read_notifications")]
    pub show_read_notifications: bool,

    /// 通知の最大保持期間（日数）
    #[serde(default = "default_max_notification_age_days")]
    pub max_notification_age_days: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: default_api_enabled(),
            port: default_api_port(),
            show_read_notifications: default_show_read_notifications(),
            max_notification_age_days: default_max_notification_age_days(),
        }
    }
}

/// ログ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// ログレベル（省略可、デフォルト: info）
    #[serde(default = "default_log_level")]
    pub level: String,

    /// ログファイルのパス（省略可、デフォルト: データディレクトリ下の logs/gh-notifier.log）
    #[serde(default)]
    pub file_path: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_path: None,
        }
    }
}

/// 通知フィルタの設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationFilter {
    /// 除外するリポジトリのリスト
    #[serde(default)]
    pub exclude_repositories: Vec<String>,

    /// 除外する通知の理由のリスト（例: "mention", "comment", "subscribed" など）
    #[serde(default)]
    pub exclude_reasons: Vec<String>,

    // 新しいリポジトリベースのフィルター
    /// 含めるリポジトリのリスト（指定がある場合、このリストに含まれるリポジトリのみ通知）
    #[serde(default)]
    pub include_repositories: Vec<String>,

    /// 含める組織のリスト（指定がある場合、このリストに含まれる組織のリポジトリのみ通知）
    #[serde(default)]
    pub include_organizations: Vec<String>,

    /// 除外する組織のリスト（このリストに含まれる組織のリポジトリは通知されない）
    #[serde(default)]
    pub exclude_organizations: Vec<String>,

    /// プライベートリポジトリの通知を除外するかどうか
    #[serde(default)]
    pub exclude_private_repos: bool,

    /// フォークリポジトリの通知を除外するかどうか
    #[serde(default)]
    pub exclude_fork_repos: bool,

    // 新しいタイプベースのフィルター
    /// 含める通知の種類のリスト（例: "Issue", "PullRequest", "Commit", "Release" など）
    #[serde(default)]
    pub include_subject_types: Vec<String>,

    /// 除外する通知の種類のリスト（例: "Commit", "Release" など）
    #[serde(default)]
    pub exclude_subject_types: Vec<String>,

    /// 含める通知の理由のリスト（指定がある場合、このリストに含まれる理由のみ通知）
    #[serde(default)]
    pub include_reasons: Vec<String>,

    // 新しいコンテンツベースのフィルター
    /// 通知タイトルに含まれるべきキーワードのリスト
    #[serde(default)]
    pub title_contains: Vec<String>,

    /// 通知タイトルに含まれてはいけないキーワードのリスト
    #[serde(default)]
    pub title_not_contains: Vec<String>,

    /// リポジトリ名に含まれるべきキーワードのリスト
    #[serde(default)]
    pub repository_contains: Vec<String>,

    /// 参加したスレッドの通知を除外するかどうか
    #[serde(default)]
    pub exclude_participating: bool,

    // 新しい高度なフィルター
    /// 最小更新時間（例: "1h", "30m", "2d" など）
    #[serde(default)]
    pub minimum_updated_time: Option<String>,

    /// ドラフトPRの通知を除外するかどうか
    #[serde(default)]
    pub exclude_draft_prs: bool,
}

/// 通知バッチ処理の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBatchConfig {
    /// 通知バッチの最大数（0の場合はバッチ処理を行わない）
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// バッチ処理の間隔（秒）
    #[serde(default = "default_batch_interval_sec")]
    pub batch_interval_sec: u64,
}

fn default_batch_size() -> usize {
    0 // バッチ処理を無効にするデフォルト
}

fn default_batch_interval_sec() -> u64 {
    30
}

impl Default for NotificationBatchConfig {
    fn default() -> Self {
        NotificationBatchConfig {
            batch_size: default_batch_size(),
            batch_interval_sec: default_batch_interval_sec(),
        }
    }
}

/// ポーリング処理のエラーハンドリング設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingErrorHandlingConfig {
    /// エラー発生時の再試行回数
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// 再試行間隔（秒）
    #[serde(default = "default_retry_interval_sec")]
    pub retry_interval_sec: u64,
}

fn default_retry_count() -> u32 {
    3
}

fn default_retry_interval_sec() -> u64 {
    5
}

impl Default for PollingErrorHandlingConfig {
    fn default() -> Self {
        PollingErrorHandlingConfig {
            retry_count: default_retry_count(),
            retry_interval_sec: default_retry_interval_sec(),
        }
    }
}

/// メイン設定構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// GitHub API設定
    #[serde(default)]
    pub github: GitHubConfig,

    /// 通知設定
    #[serde(default)]
    pub notification: NotificationConfig,

    /// ポーリング設定
    #[serde(default)]
    pub polling: PollingConfig,

    /// APIサーバー設定
    #[serde(default)]
    pub api: ApiConfig,

    /// ログ設定
    #[serde(default)]
    pub logging: LoggingConfig,
}

// デフォルト値の定義（互換性のため残す）
fn default_poll_interval_sec() -> u64 {
    30
}

fn default_mark_as_read_on_notify() -> bool {
    false
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_persistent_notifications() -> bool {
    false // デフォルトでは現在の挙動（自動消去）を維持
}

fn default_api_enabled() -> bool {
    false
}

fn default_api_port() -> u16 {
    8080
}

fn default_show_read_notifications() -> bool {
    true
}

fn default_max_notification_age_days() -> u32 {
    30
}

impl Default for Config {
    fn default() -> Self {
        // デフォルトでは自分宛てのPRレビュー依頼の通知のみを表示
        let notification_filters = NotificationFilter {
            include_reasons: vec!["review_requested".to_string()],
            include_subject_types: vec!["PullRequest".to_string()],
            ..Default::default()
        };

        Self {
            github: GitHubConfig::default(),
            notification: NotificationConfig {
                mark_as_read_on_notify: default_mark_as_read_on_notify(),
                persistent_notifications: default_persistent_notifications(),
                filters: notification_filters,
                batch: NotificationBatchConfig::default(),
            },
            polling: PollingConfig::default(),
            api: ApiConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// 互換性のためのgetterメソッド
    /// 将来的には新しい階層化された設定を使用することを推奨
    pub fn poll_interval_sec(&self) -> u64 {
        self.polling.interval_sec
    }

    pub fn mark_as_read_on_notify(&self) -> bool {
        self.notification.mark_as_read_on_notify
    }

    pub fn github_token(&self) -> Option<&str> {
        self.github.token.as_deref()
    }

    pub fn notification_filters(&self) -> &NotificationFilter {
        &self.notification.filters
    }

    pub fn log_level(&self) -> &str {
        &self.logging.level
    }

    pub fn persistent_notifications(&self) -> bool {
        self.notification.persistent_notifications
    }

    pub fn api_enabled(&self) -> bool {
        self.api.enabled
    }

    pub fn api_port(&self) -> u16 {
        self.api.port
    }

    // 旧来のフィールド名の互換性サポート
    pub fn notification_batch_config(&self) -> &NotificationBatchConfig {
        &self.notification.batch
    }

    pub fn polling_error_handling_config(&self) -> &PollingErrorHandlingConfig {
        &self.polling.error_handling
    }

    pub fn show_read_notifications(&self) -> bool {
        self.api.show_read_notifications
    }

    pub fn notification_recovery_window_hours(&self) -> u64 {
        (self.api.max_notification_age_days * 24).into() // 互換性のためdaysをhoursに変換
    }

    pub fn log_file_path(&self) -> &Option<String> {
        &self.logging.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.poll_interval_sec(), 30);
        assert_eq!(config.mark_as_read_on_notify(), false);
        assert_eq!(config.github_token(), None);
        assert_eq!(config.log_level(), "info");
        assert_eq!(config.persistent_notifications(), false);
        assert_eq!(config.api_enabled(), false);
        assert_eq!(config.api_port(), 8080);
    }

    #[test]
    fn test_github_config_default() {
        let github_config = GitHubConfig::default();

        assert_eq!(github_config.api_base_url, "https://api.github.com");
        assert_eq!(github_config.retry_count, 3);
        assert_eq!(github_config.retry_interval_sec, 5);
        assert_eq!(github_config.token, None);
    }

    #[test]
    fn test_github_config_with_token() {
        let mut github_config = GitHubConfig::default();
        github_config.token = Some("ghp_testtoken".to_string());

        assert_eq!(github_config.token, Some("ghp_testtoken".to_string()));
    }

    #[test]
    fn test_notification_config_default() {
        let notification_config = NotificationConfig::default();

        assert_eq!(notification_config.mark_as_read_on_notify, false);
        assert_eq!(notification_config.persistent_notifications, false);
        assert_eq!(notification_config.filters.exclude_private_repos, false);
        assert_eq!(notification_config.filters.exclude_fork_repos, false);
        assert_eq!(notification_config.batch.batch_size, 0);
        assert_eq!(notification_config.batch.batch_interval_sec, 30);
    }

    #[test]
    fn test_polling_config_default() {
        let polling_config = PollingConfig::default();

        assert_eq!(polling_config.interval_sec, 30);
        assert_eq!(polling_config.error_handling.retry_count, 3);
        assert_eq!(polling_config.error_handling.retry_interval_sec, 5);
    }

    #[test]
    fn test_api_config_default() {
        let api_config = ApiConfig::default();

        assert_eq!(api_config.enabled, false);
        assert_eq!(api_config.port, 8080);
        assert_eq!(api_config.show_read_notifications, true);
        assert_eq!(api_config.max_notification_age_days, 30);
    }

    #[test]
    fn test_logging_config_default() {
        let logging_config = LoggingConfig::default();

        assert_eq!(logging_config.level, "info");
        assert_eq!(logging_config.file_path, None);
    }

    #[test]
    fn test_notification_filter_default() {
        let filter = NotificationFilter::default();

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

    #[test]
    fn test_notification_batch_config_default() {
        let batch_config = NotificationBatchConfig::default();

        assert_eq!(batch_config.batch_size, 0);
        assert_eq!(batch_config.batch_interval_sec, 30);
    }

    #[test]
    fn test_polling_error_handling_config_default() {
        let error_handling_config = PollingErrorHandlingConfig::default();

        assert_eq!(error_handling_config.retry_count, 3);
        assert_eq!(error_handling_config.retry_interval_sec, 5);
    }

    #[test]
    fn test_config_serialization_deserialization() {
        let config = Config::default();

        // シリアライズ
        let serialized = toml::to_string(&config).expect("Failed to serialize Config");
        assert!(serialized.contains("[polling]"));
        assert!(serialized.contains("interval_sec = 30"));
        assert!(serialized.contains("[api]"));
        assert!(serialized.contains("enabled = false"));

        // デシリアライズ
        let deserialized: Config =
            toml::from_str(&serialized).expect("Failed to deserialize Config");
        assert_eq!(deserialized.poll_interval_sec(), config.poll_interval_sec());
        assert_eq!(
            deserialized.mark_as_read_on_notify(),
            config.mark_as_read_on_notify()
        );
        assert_eq!(deserialized.api_enabled(), config.api_enabled());
    }

    #[test]
    fn test_config_with_custom_values() {
        let mut config = Config::default();
        config.github.token = Some("test_token".to_string());
        config.github.retry_count = 5;
        config.notification.mark_as_read_on_notify = true;
        config.polling.interval_sec = 60;

        assert_eq!(config.github_token(), Some("test_token"));
        assert_eq!(config.github.retry_count, 5);
        assert_eq!(config.mark_as_read_on_notify(), true);
        assert_eq!(config.poll_interval_sec(), 60);
    }

    #[test]
    fn test_notification_filter_with_values() {
        let mut filter = NotificationFilter::default();
        filter.exclude_repositories = vec!["test/repo".to_string()];
        filter.include_reasons = vec!["review_requested".to_string()];
        filter.exclude_private_repos = true;
        filter.exclude_draft_prs = true;

        assert_eq!(filter.exclude_repositories, vec!["test/repo"]);
        assert_eq!(filter.include_reasons, vec!["review_requested"]);
        assert_eq!(filter.exclude_private_repos, true);
        assert_eq!(filter.exclude_draft_prs, true);
    }

    #[test]
    fn test_api_config_with_custom_values() {
        let mut api_config = ApiConfig::default();
        api_config.enabled = true;
        api_config.port = 9090;
        api_config.show_read_notifications = false;
        api_config.max_notification_age_days = 7;

        assert_eq!(api_config.enabled, true);
        assert_eq!(api_config.port, 9090);
        assert_eq!(api_config.show_read_notifications, false);
        assert_eq!(api_config.max_notification_age_days, 7);
    }

    #[test]
    fn test_logging_config_with_custom_values() {
        let mut logging_config = LoggingConfig::default();
        logging_config.level = "debug".to_string();
        logging_config.file_path = Some("/tmp/test.log".to_string());

        assert_eq!(logging_config.level, "debug");
        assert_eq!(logging_config.file_path, Some("/tmp/test.log".to_string()));
    }

    #[test]
    fn test_default_config_simple() {
        let config = Config::default();
        assert_eq!(config.poll_interval_sec(), 30);
        assert!(!config.mark_as_read_on_notify());
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("[polling]"));
        assert!(serialized.contains("interval_sec = 30"));
        assert!(serialized.contains("mark_as_read_on_notify = false"));
    }

    #[test]
    fn test_deserialize_config() {
        let toml_str = r#"
            [polling]
            interval_sec = 60
            [notification]
            mark_as_read_on_notify = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec(), 60);
        assert!(config.mark_as_read_on_notify());
    }

    #[test]
    fn test_deserialize_with_defaults() {
        let toml_str = r#"
            [polling]
            interval_sec = 45
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec(), 45);
        assert!(!config.mark_as_read_on_notify()); // デフォルト
    }

    #[test]
    fn test_log_level_default() {
        let config = Config::default();
        assert_eq!(config.log_level(), "info");
    }

    #[test]
    fn test_log_level_custom() {
        let toml_str = r#"
            [logging]
            level = "debug"
            [polling]
            interval_sec = 30
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level(), "debug");
    }

    #[test]
    fn test_log_level_with_other_fields() {
        let toml_str = r#"
            [polling]
            interval_sec = 60
            [notification]
            mark_as_read_on_notify = true
            [logging]
            level = "warn"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level(), "warn");
        assert_eq!(config.poll_interval_sec(), 60);
        assert!(config.mark_as_read_on_notify());
    }

    #[tokio::test]
    async fn test_load_default_config() {
        // 存在しないファイルパスでテスト
        let config = Config::default();
        assert_eq!(config.poll_interval_sec(), 30);
        assert!(!config.mark_as_read_on_notify());
        assert_eq!(config.log_level(), "info");
    }
}

/// 設定ファイルのパスを取得
fn config_file_path() -> PathBuf {
    let mut path = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().expect("現在のディレクトリが取得できません"));
    path.push("gh-notifier");
    path.push("config.toml");
    path
}

/// 設定ファイルを読み込む
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = config_file_path();

    if config_path.exists() {
        let contents = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    } else {
        // ファイルが存在しない場合はデフォルト設定を返す
        Ok(Config::default())
    }
}

/// 設定ファイルを保存する
pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_file_path();
    let parent_dir = config_path.parent().unwrap();

    // 設定ディレクトリが存在しない場合は作成
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir)?;
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(config_path, contents)?;
    Ok(())
}
