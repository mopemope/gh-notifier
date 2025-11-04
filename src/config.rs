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

    /// 通知フィルタの設定
    #[serde(default)]
    pub notification_filters: NotificationFilter,

    /// 通知バッチ処理の設定
    #[serde(default)]
    pub notification_batch_config: NotificationBatchConfig,

    /// ポーリング処理のエラーハンドリング設定
    #[serde(default)]
    pub polling_error_handling_config: PollingErrorHandlingConfig,

    /// GitHub OAuth Client ID（省略可）
    #[serde(default = "default_client_id")]
    pub client_id: String,

    /// ログレベル（省略可、デフォルト: info）
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// ログファイルのパス（省略可、デフォルト: データディレクトリ下の logs/gh-notifier.log）
    #[serde(default)]
    pub log_file_path: Option<String>,
}

// デフォルト値の定義
fn default_poll_interval_sec() -> u64 {
    30
}

fn default_mark_as_read_on_notify() -> bool {
    false
}

fn default_client_id() -> String {
    // 仕様書で示されたデフォルトクライアントID
    "Iv1.898a6d2a86c3f7aa".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            poll_interval_sec: default_poll_interval_sec(),
            mark_as_read_on_notify: default_mark_as_read_on_notify(),
            notification_filters: NotificationFilter::default(),
            notification_batch_config: NotificationBatchConfig::default(),
            polling_error_handling_config: PollingErrorHandlingConfig::default(),
            client_id: default_client_id(),
            log_level: default_log_level(),
            log_file_path: None,
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
        assert_eq!(config.client_id, "Iv1.898a6d2a86c3f7aa");
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("poll_interval_sec = 30"));
        assert!(serialized.contains("mark_as_read_on_notify = false"));
        assert!(serialized.contains("client_id = \"Iv1.898a6d2a86c3f7aa\""));
    }

    #[test]
    fn test_deserialize_config() {
        let toml_str = r#"
            poll_interval_sec = 60
            mark_as_read_on_notify = true
            client_id = "custom-client-id"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec, 60);
        assert!(config.mark_as_read_on_notify);
        assert_eq!(config.client_id, "custom-client-id");
    }

    #[test]
    fn test_deserialize_with_defaults() {
        let toml_str = r#"
            poll_interval_sec = 45
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec, 45);
        assert!(!config.mark_as_read_on_notify); // デフォルト
        assert_eq!(config.client_id, "Iv1.898a6d2a86c3f7aa"); // デフォルト
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
            client_id = "custom-client-id"
            log_level = "warn"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.log_level, "warn");
        assert_eq!(config.poll_interval_sec, 60);
        assert!(config.mark_as_read_on_notify);
        assert_eq!(config.client_id, "custom-client-id");
    }

    #[tokio::test]
    async fn test_load_default_config() {
        // 存在しないファイルパスでテスト
        let config = Config::default();
        assert_eq!(config.poll_interval_sec, 30);
        assert!(!config.mark_as_read_on_notify);
        assert_eq!(config.client_id, "Iv1.898a6d2a86c3f7aa");
        assert_eq!(config.log_level, "info");
    }
}
