use crate::{AuthError, TokenInfo, token_storage::TokenStorage};

pub const GITHUB_DEVICE_AUTHORIZATION_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub struct AuthManager {
    pub client_id: String,
    pub token_info: Option<TokenInfo>,
    pub(in crate::auth) token_storage: TokenStorage,
}

impl AuthManager {
    /// Creates a new AuthManager instance for normal usage
    pub fn new() -> Result<Self, AuthError> {
        // Create the token storage with fallback mechanisms
        let token_storage = TokenStorage::new()?;

        // Load the config to get the client ID
        let config = crate::config::load_config()
            .map_err(|e| AuthError::GeneralError(format!("Failed to load config: {}", e)))?;

        Ok(AuthManager {
            client_id: config.client_id,
            token_info: None,
            token_storage,
        })
    }

    /// Creates a new AuthManager instance for tests (with no keyring)
    #[cfg(test)]
    pub fn new_for_tests() -> Result<Self, AuthError> {
        // Create a temporary path for the token file in tests
        let mut token_file_path = std::env::temp_dir();
        token_file_path.push("gh_notifier_test_token.json");

        let token_storage = crate::token_storage::TokenStorage {
            keyring_entry: None,
            token_file_path,
        };

        // Load the config to get the client ID
        let config = crate::config::load_config()
            .map_err(|e| AuthError::GeneralError(format!("Failed to load config: {}", e)))?;

        Ok(AuthManager {
            client_id: config.client_id,
            token_info: None,
            token_storage,
        })
    }
}
