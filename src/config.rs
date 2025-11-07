use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

/// 設定ファイルの構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// ポーリング間隔（秒）
    #[serde(default = "default_poll_interval_sec")]
    pub poll_interval_sec: u64,

    /// 通知表示時に通知を既読にするかどうか
    #[serde(default = "default_mark_as_read_on_notify")]
    pub mark_as_read_on_notify: bool,

    /// GitHub Personal Access Token (Classic PAT)
    #[serde(default)]
    pub pat: Option<String>,

    /// 通知フィルタの設定
    #[serde(default)]
    pub notification_filters: NotificationFilter,

    /// 通知バッチ処理の設定
    #[serde(default)]
    pub notification_batch_config: NotificationBatchConfig,

    /// ポーリング処理のエラーハンドリング設定
    #[serde(default)]
    pub polling_error_handling_config: PollingErrorHandlingConfig,

    /// ログレベル（省略可、デフォルト: info）
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// ログファイルのパス（省略可、デフォルト: データディレクトリ下の logs/gh-notifier.log）
    #[serde(default)]
    pub log_file_path: Option<String>,

    /// 通知を永続的に表示するかどうか（自動消去しない）
    #[serde(default = "default_persistent_notifications")]
    pub persistent_notifications: bool,

    /// 重要度の高い通知の理由のリスト（例: "review_requested", "mention" など）
    /// このリストに含まれる理由の通知は、特別な処理（例：永続表示）の対象となる
    #[serde(default)]
    pub important_notification_reasons: Vec<String>,

    /// 重要度の高い通知を永続的に表示するかどうか（自動消去しない）
    #[serde(default = "default_persistent_important_notifications")]
    pub persistent_important_notifications: bool,

    /// APIサーバーを有効にするかどうか
    #[serde(default = "default_api_enabled")]
    pub api_enabled: bool,

    /// APIサーバーのポート番号
    #[serde(default = "default_api_port")]
    pub api_port: u16,

    /// 既読通知を表示するかどうか
    #[serde(default = "default_show_read_notifications")]
    pub show_read_notifications: bool,

    /// システム再起動時に再通知する通知の期間（時間）
    #[serde(default = "default_notification_recovery_window_hours")]
    pub notification_recovery_window_hours: u64,
}

// デフォルト値の定義
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
    true // デフォルトで通知を永続的に表示
}

fn default_persistent_important_notifications() -> bool {
    false // デフォルトでは重要度の高い通知も一時的に表示
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

fn default_notification_recovery_window_hours() -> u64 {
    24 // Default to 24 hours
}

impl Config {
    /// 通知の理由が重要度の高いものであるかを判定する
    pub fn is_important_notification_reason(&self, reason: &str) -> bool {
        self.important_notification_reasons
            .contains(&reason.to_string())
    }

    /// 通知が重要度の高いものであるかを判定し、それに応じた永続表示設定値を返す
    pub fn get_persistent_setting_for_notification(&self, reason: &str) -> bool {
        if self.is_important_notification_reason(reason) {
            self.persistent_important_notifications
        } else {
            self.persistent_notifications
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        // デフォルトでは自分宛てのPRレビュー依頼の通知のみを表示
        let notification_filters = NotificationFilter {
            include_reasons: vec!["review_requested".to_string()],
            include_subject_types: vec!["PullRequest".to_string()],
            ..Default::default()
        };

        Config {
            poll_interval_sec: default_poll_interval_sec(),
            mark_as_read_on_notify: default_mark_as_read_on_notify(),
            pat: None,
            notification_filters,
            notification_batch_config: NotificationBatchConfig::default(),
            polling_error_handling_config: PollingErrorHandlingConfig::default(),
            log_level: default_log_level(),
            log_file_path: None,
            persistent_notifications: default_persistent_notifications(),
            important_notification_reasons: Vec::new(), // デフォルトは空のリスト
            persistent_important_notifications: default_persistent_important_notifications(),
            api_enabled: default_api_enabled(),
            api_port: default_api_port(),
            show_read_notifications: default_show_read_notifications(),
            notification_recovery_window_hours: default_notification_recovery_window_hours(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.poll_interval_sec, 30);
        assert!(!config.mark_as_read_on_notify);
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("poll_interval_sec = 30"));
        assert!(serialized.contains("mark_as_read_on_notify = false"));
    }

    #[test]
    fn test_deserialize_config() {
        let toml_str = r#"
            poll_interval_sec = 60
            mark_as_read_on_notify = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec, 60);
        assert!(config.mark_as_read_on_notify);
    }

    #[test]
    fn test_deserialize_with_defaults() {
        let toml_str = r#"
            poll_interval_sec = 45
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec, 45);
        assert!(!config.mark_as_read_on_notify); // デフォルト
    }

    #[test]
    fn test_log_level_default() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_log_level_custom() {
        let toml_str = r#"
            log_level = "debug"
            poll_interval_sec = 30
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_log_level_with_other_fields() {
        let toml_str = r#"
            poll_interval_sec = 60
            mark_as_read_on_notify = true
            log_level = "warn"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level, "warn");
        assert_eq!(config.poll_interval_sec, 60);
        assert!(config.mark_as_read_on_notify);
    }

    #[tokio::test]
    async fn test_load_default_config() {
        // 存在しないファイルパスでテスト
        let config = Config::default();
        assert_eq!(config.poll_interval_sec, 30);
        assert!(!config.mark_as_read_on_notify);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_is_important_notification_reason() {
        let config = Config {
            important_notification_reasons: vec![
                "review_requested".to_string(),
                "mention".to_string(),
            ],
            ..Config::default()
        };

        assert!(config.is_important_notification_reason("review_requested"));
        assert!(config.is_important_notification_reason("mention"));
        assert!(!config.is_important_notification_reason("subscribed"));
        assert!(!config.is_important_notification_reason(""));
    }

    #[test]
    fn test_get_persistent_setting_for_notification() {
        let config = Config {
            important_notification_reasons: vec![
                "review_requested".to_string(),
                "mention".to_string(),
            ],
            persistent_important_notifications: true,
            persistent_notifications: false,
            ..Config::default()
        };

        // 重要度が高い場合
        assert!(config.get_persistent_setting_for_notification("review_requested"));
        assert!(config.get_persistent_setting_for_notification("mention"));

        // 重要度が低い場合
        assert!(!config.get_persistent_setting_for_notification("subscribed"));
        assert!(!config.get_persistent_setting_for_notification("comment"));

        // 重要度が高い場合でも、persistent_important_notificationsがfalseならfalse
        let non_persistent_config = Config {
            important_notification_reasons: vec![
                "review_requested".to_string(),
                "mention".to_string(),
            ],
            persistent_important_notifications: false,
            persistent_notifications: false,
            ..Config::default()
        };
        assert!(!non_persistent_config.get_persistent_setting_for_notification("review_requested"));

        // 重要度が低い場合はpersistent_notificationsの値が返る
        let normal_persistent_config = Config {
            important_notification_reasons: vec![
                "review_requested".to_string(),
                "mention".to_string(),
            ],
            persistent_important_notifications: false,
            persistent_notifications: true,
            ..Config::default()
        };
        assert!(normal_persistent_config.get_persistent_setting_for_notification("subscribed"));
    }

    #[test]
    fn test_config_deserialization_with_important_notification_settings() {
        let toml_str = r#"
            poll_interval_sec = 60
            mark_as_read_on_notify = true
            important_notification_reasons = ["review_requested", "mention", "assign"]
            persistent_important_notifications = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();

        assert_eq!(config.poll_interval_sec, 60);
        assert!(config.mark_as_read_on_notify);
        assert_eq!(
            config.important_notification_reasons,
            vec!["review_requested", "mention", "assign"]
        );
        assert!(config.persistent_important_notifications);
    }

    #[test]
    fn test_config_serialization_with_important_notification_settings() {
        let config = Config {
            important_notification_reasons: vec![
                "review_requested".to_string(),
                "mention".to_string(),
            ],
            persistent_important_notifications: true,
            ..Config::default()
        };

        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("review_requested"));
        assert!(serialized.contains("mention"));
        assert!(serialized.contains("persistent_important_notifications = true"));
    }
}
