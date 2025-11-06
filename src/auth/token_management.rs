use crate::{AuthError, TokenInfo};

impl super::AuthManager {
    /// Loads the token from storage (with fallback mechanisms)
    pub fn load_token_from_storage(&mut self) -> Result<Option<TokenInfo>, AuthError> {
        match &self.token_storage {
            Some(storage) => match storage.load_token() {
                Ok(Some(token_info)) => {
                    tracing::info!("Token successfully loaded from storage");
                    self.token_info = Some(token_info.clone());
                    Ok(Some(token_info))
                }
                Ok(None) => {
                    tracing::debug!("No token found in storage");
                    Ok(None)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load token from storage: {:?}. Tokens will not be loaded.",
                        e
                    );
                    Ok(None)
                }
            },
            None => {
                tracing::debug!("Token storage not available, skipping load from storage");
                Ok(None)
            }
        }
    }

    /// Saves the token to storage (with fallback mechanisms)
    pub fn save_token_to_storage(&self, token_info: &TokenInfo) -> Result<(), AuthError> {
        match &self.token_storage {
            Some(storage) => storage.save_token(token_info),
            None => {
                tracing::debug!("Token storage not available, skipping save to storage");
                // Returning Ok here is appropriate since the token is provided in config
                Ok(())
            }
        }
    }

    /// Deletes the token from storage (with fallback mechanisms)
    pub fn delete_token_from_storage(&mut self) -> Result<(), AuthError> {
        match &mut self.token_storage {
            Some(storage) => {
                storage.delete_token()?;
                // Clear the in-memory token info as well
                self.token_info = None;
                Ok(())
            }
            None => {
                tracing::debug!("Token storage not available, skipping deletion from storage");
                // Clear the in-memory token info as well
                self.token_info = None;
                Ok(())
            }
        }
    }
}
