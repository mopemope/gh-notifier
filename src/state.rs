use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    /// 最終確認日時（ISO 8601形式）
    pub last_checked_at: Option<String>,
    /// ETagのマップ（URL -> ETag）
    pub etags: HashMap<String, String>,
}

pub struct StateManager {
    state_file_path: PathBuf,
    pub state: State,
}

impl StateManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let state_file_path = Self::state_file_path();
        let parent_dir = state_file_path.parent().unwrap();

        // 状態ファイルの親ディレクトリが存在しない場合は作成
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }

        // 状態ファイルが存在すれば読み込み、なければデフォルト状態
        let state = if state_file_path.exists() {
            let contents = fs::read_to_string(&state_file_path)?;
            serde_json::from_str(&contents)?
        } else {
            State::default()
        };

        Ok(StateManager {
            state_file_path,
            state,
        })
    }

    fn state_file_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| {
            std::env::current_dir().expect("現在のディレクトリが取得できません")
        });
        path.push("gh-notifier");
        path.push("state.json");
        path
    }

    /// 状態をファイルに保存
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_file_path, contents)?;
        Ok(())
    }

    /// 最終確認日時を取得
    pub fn get_last_checked_at(&self) -> Option<&str> {
        self.state.last_checked_at.as_deref()
    }

    /// 最終確認日時を更新
    pub fn update_last_checked_at(&mut self, timestamp: String) {
        self.state.last_checked_at = Some(timestamp);
    }

    /// ETagを取得
    pub fn get_etag(&self, url: &str) -> Option<&str> {
        self.state.etags.get(url).map(|s| s.as_str())
    }

    /// ETagを更新
    pub fn update_etag(&mut self, url: String, etag: String) {
        self.state.etags.insert(url, etag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert!(state.last_checked_at.is_none());
        assert!(state.etags.is_empty());
    }

    #[test]
    fn test_state_serialization() {
        use std::collections::HashMap;

        let state = State {
            last_checked_at: Some("2023-01-01T00:00:00Z".to_string()),
            etags: {
                let mut map = HashMap::new();
                map.insert(
                    "https://api.github.com/notifications".to_string(),
                    "etag123".to_string(),
                );
                map
            },
        };

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: State = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized.last_checked_at,
            Some("2023-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            deserialized
                .etags
                .get("https://api.github.com/notifications"),
            Some(&"etag123".to_string())
        );
    }

    #[test]
    fn test_state_manager_new_with_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        let state_manager = StateManager {
            state_file_path: temp_path.to_path_buf(),
            state: State::default(),
        };
        // ファイルに保存
        state_manager.save().unwrap();
        // ファイルから読み込み
        let state_manager2 = StateManager::new().unwrap();
        assert!(state_manager2.state.last_checked_at.is_none());
        assert!(state_manager2.state.etags.is_empty());
    }

    #[test]
    fn test_state_manager_save_error() {
        // 書き込み権限がないディレクトリを指定
        let state_manager = StateManager {
            state_file_path: std::path::PathBuf::from("/"),
            state: State::default(),
        };
        let result = state_manager.save();
        assert!(result.is_err());
    }

    #[test]
    fn test_state_manager_get_set_last_checked_at() {
        let mut state_manager = StateManager {
            state_file_path: std::path::PathBuf::new(),
            state: State::default(),
        };
        let timestamp = "2023-01-01T00:00:00Z".to_string();
        state_manager.update_last_checked_at(timestamp.clone());
        assert_eq!(
            state_manager.get_last_checked_at(),
            Some(timestamp.as_str())
        );
    }

    #[test]
    fn test_state_manager_get_set_etag() {
        let mut state_manager = StateManager {
            state_file_path: std::path::PathBuf::new(),
            state: State::default(),
        };
        let url = "https://api.github.com/notifications".to_string();
        let etag = "etag123".to_string();
        state_manager.update_etag(url.clone(), etag.clone());
        assert_eq!(state_manager.get_etag(&url), Some(etag.as_str()));
    }
}
