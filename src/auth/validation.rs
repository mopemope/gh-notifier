use crate::AuthError;
use secrecy::ExposeSecret;

impl super::AuthManager {
    /// Checks if the access token has expired
    /// For PATs, this always returns false since they don't expire by default
    pub fn is_access_token_expired(&self) -> bool {
        // PATs don't expire by default, so return false
        // Only return true if there's no token at all
        self.token_info.is_none()
    }

    /// Checks if the access token will expire soon (within the specified number of seconds)
    /// For PATs, this always returns false since they don't expire by default
    pub fn is_access_token_expiring_soon(&self, _within_seconds: u64) -> bool {
        // PATs don't expire by default, so return false
        // Only return true if there's no token at all
        self.token_info.is_none()
    }

    /// Checks if the refresh token has expired
    /// For PATs, this always returns true since they don't have refresh tokens
    pub fn is_refresh_token_expired(&self) -> bool {
        // PATs don't have refresh tokens
        true
    }

    /// Validates the current token by making a request to GitHub's API
    /// This is useful to determine if a token is actually valid or if re-auth is needed
    pub async fn validate_token(&self) -> Result<bool, AuthError> {
        if let Some(ref token_info) = self.token_info {
            let client = reqwest::Client::builder()
                .user_agent(format!("gh-notifier/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| AuthError::Generic {
                    reason: format!("Failed to create HTTP client: {}", e),
                })?;

            let response = client
                .get("https://api.github.com/user")
                .header(
                    "Authorization",
                    format!("token {}", token_info.access_token.expose_secret()),
                )
                .send()
                .await
                .map_err(|e| {
                    tracing::warn!("Token validation request failed: {}", e);
                    // Don't treat network errors as token invalidation
                    // Instead, return the error so the caller can handle it appropriately
                    AuthError::Generic {
                        reason: format!("Network error during token validation: {}", e),
                    }
                })?;

            let status = response.status();
            tracing::debug!("Token validation response status: {}", status);

            if status.is_success() {
                tracing::debug!("Token validation successful");
                Ok(true)
            } else {
                tracing::debug!("Token validation failed with status: {}", status);
                // Don't treat non-200 responses as errors, just as invalid token
                Ok(false)
            }
        } else {
            // No token to validate
            tracing::debug!("No token to validate");
            Ok(false)
        }
    }
}
