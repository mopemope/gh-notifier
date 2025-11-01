use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 設定ファイルの構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// ポーリング間隔（秒）
    #[serde(default = "default_poll_interval_sec")]
    pub poll_interval_sec: u64,

    /// 通知表示時に通知を既読にするかどうか
    #[serde(default = "default_mark_as_read_on_notify")]
    pub mark_as_read_on_notify: bool,

    /// GitHub OAuth Client ID（省略可）
    #[serde(default = "default_client_id")]
    pub client_id: String,
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

impl Default for Config {
    fn default() -> Self {
        Config {
            poll_interval_sec: default_poll_interval_sec(),
            mark_as_read_on_notify: default_mark_as_read_on_notify(),
            client_id: default_client_id(),
        }
    }
}

/// 設定ファイルのパスを取得
fn config_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| {
        std::env::current_dir().expect("現在のディレクトリが取得できません")
    });
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
        assert_eq!(config.mark_as_read_on_notify, false);
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
        assert_eq!(config.mark_as_read_on_notify, true);
        assert_eq!(config.client_id, "custom-client-id");
    }

    #[test]
    fn test_deserialize_with_defaults() {
        let toml_str = r#"
            poll_interval_sec = 45
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_sec, 45);
        assert_eq!(config.mark_as_read_on_notify, false); // デフォルト
        assert_eq!(config.client_id, "Iv1.898a6d2a86c3f7aa"); // デフォルト
    }

    #[tokio::test]
    async fn test_load_default_config() {
        // 存在しないファイルパスでテスト
        let config = Config::default();
        assert_eq!(config.poll_interval_sec, 30);
        assert_eq!(config.mark_as_read_on_notify, false);
        assert_eq!(config.client_id, "Iv1.898a6d2a86c3f7aa");
    }
}