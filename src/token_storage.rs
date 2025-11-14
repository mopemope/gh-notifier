use crate::{AuthError, TokenInfo};
use keyring::Entry;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct TokenStorage {
    pub keyring_entry: Option<Arc<Entry>>,
    pub token_file_path: PathBuf,
}

impl TokenStorage {
    pub fn new() -> Result<Self, AuthError> {
        // Try to create keyring entry
        let keyring_entry = match Entry::new("gh-notifier", "github_auth_token") {
            Ok(entry) => Some(Arc::new(entry)),
            Err(e) => {
                tracing::warn!(
                    "Keyring is not available on this system ({}), will use file-based storage.",
                    e
                );
                None
            }
        };

        // Create path for fallback token file
        let mut token_file_path = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"));
        token_file_path.push("gh-notifier");
        token_file_path.push("token.json");

        // Create directory if it doesn't exist
        if let Some(parent) = token_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AuthError::Generic {
                reason: format!("Failed to create config directory: {}", e),
            })?;
        }

        Ok(TokenStorage {
            keyring_entry,
            token_file_path,
        })
    }

    pub fn save_token(&self, token_info: &TokenInfo) -> Result<(), AuthError> {
        // Try keyring first
        if let Some(ref entry) = self.keyring_entry {
            match self.save_to_keyring(entry, token_info) {
                Ok(()) => {
                    tracing::info!("Token saved to keyring successfully");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to save token to keyring: {:?}. Trying fallback storage.",
                        e
                    );
                    // Continue to fallback
                }
            }
        } else {
            tracing::debug!("Keyring not available, using fallback storage");
        }

        // Try file-based storage as fallback
        self.save_to_file(token_info)
    }

    fn save_to_keyring(&self, entry: &Entry, token_info: &TokenInfo) -> Result<(), AuthError> {
        let token_json = serde_json::to_string(token_info)?;
        entry.set_password(&token_json)?;
        Ok(())
    }

    fn save_to_file(&self, token_info: &TokenInfo) -> Result<(), AuthError> {
        let token_json = serde_json::to_string(token_info).map_err(|e| AuthError::Generic {
            reason: format!("Failed to serialize token: {}", e),
        })?;

        fs::write(&self.token_file_path, token_json).map_err(|e| AuthError::Generic {
            reason: format!("Failed to write token file: {}", e),
        })?;

        tracing::info!("Token saved to file: {:?}", self.token_file_path);
        Ok(())
    }

    pub fn load_token(&self) -> Result<Option<TokenInfo>, AuthError> {
        // Try keyring first
        if let Some(ref entry) = self.keyring_entry {
            match self.load_from_keyring(entry) {
                Ok(Some(token_info)) => {
                    tracing::info!("Token loaded from keyring");
                    return Ok(Some(token_info));
                }
                Ok(None) => {
                    // No token in keyring, continue to check file
                    tracing::debug!("No token found in keyring, checking file storage");
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load token from keyring: {:?}. Checking file storage.",
                        e
                    );
                }
            }
        }

        // Try file-based storage as fallback
        match self.load_from_file() {
            Ok(Some(token_info)) => {
                tracing::info!("Token loaded from file");
                // Try to migrate to keyring if available
                if self.keyring_entry.is_some()
                    && let Err(e) =
                        self.save_to_keyring(self.keyring_entry.as_ref().unwrap(), &token_info)
                {
                    tracing::warn!("Failed to migrate token to keyring: {:?}", e);
                }
                Ok(Some(token_info))
            }
            Ok(None) => {
                tracing::debug!("No token found in file storage either");
                Ok(None)
            }
            Err(e) => {
                tracing::warn!("Failed to load token from file: {:?}", e);
                // If both methods fail, return error
                Err(e)
            }
        }
    }

    fn load_from_keyring(&self, entry: &Entry) -> Result<Option<TokenInfo>, AuthError> {
        match entry.get_password() {
            Ok(token_json) => {
                if !token_json.is_empty() {
                    let token_info: TokenInfo = serde_json::from_str(&token_json)?;
                    Ok(Some(token_info))
                } else {
                    Ok(None)
                }
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AuthError::Generic {
                reason: format!("Keyring error: {}", e),
            }),
        }
    }

    fn load_from_file(&self) -> Result<Option<TokenInfo>, AuthError> {
        if !self.token_file_path.exists() {
            return Ok(None);
        }

        let token_json =
            fs::read_to_string(&self.token_file_path).map_err(|e| AuthError::Generic {
                reason: format!("Failed to read token file: {}", e),
            })?;

        if token_json.trim().is_empty() {
            return Ok(None);
        }

        let token_info: TokenInfo =
            serde_json::from_str(&token_json).map_err(|e| AuthError::Generic {
                reason: format!("Failed to deserialize token: {}", e),
            })?;

        Ok(Some(token_info))
    }

    pub fn delete_token(&mut self) -> Result<(), AuthError> {
        // Try to delete from keyring
        if let Some(ref entry) = self.keyring_entry {
            match entry.delete_password() {
                Ok(()) => tracing::info!("Token deleted from keyring"),
                Err(keyring::Error::NoEntry) => {
                    tracing::debug!("No token entry found in keyring to delete")
                }
                Err(e) => tracing::warn!("Failed to delete token from keyring: {:?}", e),
            }
        }

        // Delete from file as well
        if self.token_file_path.exists() {
            fs::remove_file(&self.token_file_path).map_err(|e| AuthError::Generic {
                reason: format!("Failed to delete token file: {}", e),
            })?;
            tracing::info!("Token file deleted: {:?}", self.token_file_path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::{ExposeSecret, SecretString};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_token_storage_creation() {
        // This test will work if keyring is available in the test environment
        let result = TokenStorage::new();
        // We don't fail the test if keyring isn't available, we just continue
        if result.is_ok() {
            println!("TokenStorage created successfully");
        } else {
            println!(
                "Could not create TokenStorage with keyring, but this is expected in some environments"
            );
        }
    }

    #[test]
    fn test_token_serialization() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token_info = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 7200),
        };

        let serialized = serde_json::to_string(&token_info).expect("Failed to serialize");
        let deserialized: TokenInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.token_type, "Bearer");
        assert_eq!(deserialized.expires_at, Some(now + 3600));
        assert_eq!(deserialized.access_token.expose_secret(), "test_token");
    }
}
