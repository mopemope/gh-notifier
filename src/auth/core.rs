use crate::{AuthError, TokenInfo, token_storage::TokenStorage};

pub struct AuthManager {
    pub token_info: Option<TokenInfo>,
    pub(in crate::auth) token_storage: TokenStorage,
}

impl AuthManager {
    /// Creates a new AuthManager instance for normal usage
    pub fn new() -> Result<Self, AuthError> {
        // Create the token storage with fallback mechanisms
        let token_storage = TokenStorage::new()?;

        Ok(AuthManager {
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

        Ok(AuthManager {
            token_info: None,
            token_storage,
        })
    }
}
