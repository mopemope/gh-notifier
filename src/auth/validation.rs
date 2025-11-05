use crate::AuthError;
use secrecy::ExposeSecret;
use std::time::{SystemTime, UNIX_EPOCH};

impl super::AuthManager {
    /// Checks if the access token has expired
    pub fn is_access_token_expired(&self) -> bool {
        if let Some(ref token_info) = self.token_info {
            if let Some(expires_at) = token_info.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let expired = now >= expires_at;
                tracing::debug!(
                    "Token expiration check: now={}, expires_at={}, expired={}",
                    now,
                    expires_at,
                    expired
                );
                return expired;
            }
            // If token exists but has no expiration time, assume it doesn't expire
            tracing::debug!("Token has no expiration time, assuming not expired");
            return false;
        }
        // If there's no token at all, it's considered expired
        tracing::debug!("No token available, considering expired");
        true
    }

    /// Checks if the access token will expire soon (within the specified number of seconds)
    pub fn is_access_token_expiring_soon(&self, within_seconds: u64) -> bool {
        if let Some(ref token_info) = self.token_info {
            if let Some(expires_at) = token_info.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Check if the token will expire within the specified time window
                let expiring_soon = now + within_seconds >= expires_at;
                tracing::debug!(
                    "Token expiring soon check: now={}, expires_at={}, within_seconds={}, expiring_soon={}",
                    now,
                    expires_at,
                    within_seconds,
                    expiring_soon
                );
                return expiring_soon;
            }
            // If token exists but has no expiration time, it won't expire soon
            tracing::debug!("Token has no expiration time, assuming not expiring soon");
            return false;
        }
        // If there's no token at all, then it's expiring soon (meaning we need to get one)
        tracing::debug!("No token available, considering expiring soon");
        true
    }

    /// Checks if the refresh token has expired
    pub fn is_refresh_token_expired(&self) -> bool {
        if let Some(ref token_info) = self.token_info
            && let Some(refresh_expires_at) = token_info.refresh_token_expires_at
        {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            return now >= refresh_expires_at;
        }
        // If there's no refresh token or expiration, assume it's expired
        self.token_info
            .as_ref()
            .and_then(|t| t.refresh_token.as_ref())
            .is_none()
    }

    /// Validates the current token by making a request to GitHub's API
    /// This is useful to determine if a token is actually valid or if re-auth is needed
    pub async fn validate_token(&self) -> Result<bool, AuthError> {
        if let Some(ref token_info) = self.token_info {
            // First, check if token is expired before making network call
            if self.is_access_token_expired() {
                tracing::debug!("Token validation failed: token is expired");
                return Ok(false);
            }

            let client = reqwest::Client::builder()
                .user_agent(format!("gh-notifier/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| {
                    AuthError::GeneralError(format!("Failed to create HTTP client: {}", e))
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
                    AuthError::GeneralError(format!("Network error during token validation: {}", e))
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
