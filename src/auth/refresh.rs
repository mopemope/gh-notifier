use crate::AuthError;
use secrecy::ExposeSecret;

impl super::AuthManager {
    /// Gets the access token (PATs don't need refresh as they don't expire)
    pub async fn get_valid_token(&mut self) -> Result<String, AuthError> {
        // If no token is available, return error
        if self.token_info.is_none() {
            return Err(AuthError::Generic {
                reason: "No token available".to_string(),
            });
        }

        // PATs don't expire, so we can return the token directly
        Ok(self
            .token_info
            .as_ref()
            .unwrap()
            .access_token
            .expose_secret()
            .clone())
    }

    /// Gets a valid token that was set from config
    /// This method now simply validates that the token from config is valid
    pub async fn get_valid_token_with_reauth(&mut self) -> Result<String, AuthError> {
        // Validate the token works with GitHub API
        match self.validate_token().await {
            Ok(true) => {
                // Token is valid
                Ok(self
                    .token_info
                    .as_ref()
                    .unwrap()
                    .access_token
                    .expose_secret()
                    .clone())
            }
            Ok(false) => {
                // Token is invalid
                Err(AuthError::Generic {
                    reason: "PAT validation failed. The token may be invalid or have insufficient permissions.".to_string(),
                })
            }
            Err(validation_error) => {
                // Validation failed due to network or other issues
                Err(validation_error)
            }
        }
    }
}
