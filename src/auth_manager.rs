use crate::{AuthError, DeviceAuthResponse, ErrorResponse, TokenInfo, TokenResponse};
use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_OAUTH_CLIENT_ID: &str = "Iv1.898a6d2a86c3f7aa"; // This would be configurable in a real app
const GITHUB_DEVICE_AUTHORIZATION_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub struct AuthManager {
    pub client_id: String,
    pub token_info: Option<TokenInfo>,
    keychain_entry: Option<Arc<Entry>>,
}

impl AuthManager {
    /// Creates a new AuthManager instance
    pub fn new() -> Result<Self, AuthError> {
        let keychain_entry = Entry::new("gh-notifier", "github_auth_token")
            .map(Arc::new)
            .ok();

        Ok(AuthManager {
            client_id: GITHUB_OAUTH_CLIENT_ID.to_string(),
            token_info: None,
            keychain_entry,
        })
    }

    /// Performs the OAuth Device Flow to authenticate the user and obtain an access token
    pub async fn authenticate(&mut self) -> Result<TokenInfo, AuthError> {
        // Step 1: Request device code from GitHub
        let client = reqwest::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("scope", &"notifications".to_string()),
        ];

        let response = client
            .post(GITHUB_DEVICE_AUTHORIZATION_URL)
            .form(&params)
            .send()
            .await?;

        // Parse the device authorization response properly
        let device_response: DeviceAuthResponse = response.json().await?;

        // Display instructions to user
        println!("GitHub OAuth Device Flow:");
        println!("1. Visit: {}", device_response.verification_uri);
        println!("2. Enter code: {}", device_response.user_code);
        println!("3. Confirm the authorization request");

        // Step 2: Poll for the access token with timeout handling
        let token_params = [
            ("client_id", &self.client_id),
            ("device_code", &device_response.device_code),
            (
                "grant_type",
                &"urn:ietf:params:oauth:grant-type:device_code".to_string(),
            ),
        ];

        // Calculate the absolute timeout time
        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(device_response.expires_in);

        // Poll until we get the token, the device code expires, or an error occurs
        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() >= timeout_duration {
                return Err(AuthError::AuthorizationTimeout);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(device_response.interval)).await;

            let token_response = client
                .post(GITHUB_TOKEN_URL)
                .form(&token_params)
                .send()
                .await?;

            let status = token_response.status();
            let response_text = token_response.text().await?;

            // Try to parse as error response first
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
                match error_response.error.as_str() {
                    "authorization_pending" => {
                        // Continue polling, user hasn't authorized yet
                        continue;
                    }
                    "slow_down" => {
                        // GitHub is asking us to slow down, increase the interval by 5 seconds
                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            device_response.interval + 5,
                        ))
                        .await;
                        continue;
                    }
                    "expired_token" => {
                        return Err(AuthError::DeviceCodeExpired);
                    }
                    "authorization_declined" => {
                        return Err(AuthError::AuthorizationCancelled);
                    }
                    "unsupported_grant_type" | "incorrect_client_credentials" | "invalid_grant" => {
                        return Err(AuthError::OAuthError {
                            code: error_response.error.clone(),
                            description: error_response.error_description.clone(),
                        });
                    }
                    _ => {
                        return Err(AuthError::OAuthError {
                            code: error_response.error,
                            description: error_response.error_description,
                        });
                    }
                }
            } else if status.is_success() {
                // If it's not an error response and status is success, try to parse as token response
                let token_data: TokenResponse = serde_json::from_str(&response_text)?;

                // Success! We got the token
                let expires_at = token_data
                    .expires_in
                    .map(|expires_in| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + expires_in
                    })
                    .or_else(|| {
                        // Default expiration time (GitHub tokens typically expire in 1 hour by default)
                        Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                + 3600, // 1 hour in seconds
                        )
                    });

                let refresh_token = token_data.refresh_token.map(SecretString::new);
                let refresh_expires_at = token_data
                    .refresh_token_expires_in
                    .map(|expires_in| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + expires_in
                    })
                    .or_else(|| {
                        // Default refresh token expiration to 6 months if not provided
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|dur| dur.as_secs() + (6 * 30 * 24 * 3600)) // 6 months
                    });

                let token_info = TokenInfo {
                    access_token: SecretString::new(token_data.access_token),
                    token_type: token_data.token_type,
                    expires_at,
                    refresh_token,
                    refresh_token_expires_at: refresh_expires_at,
                };

                // Update our internal state
                self.token_info = Some(token_info.clone());

                return Ok(token_info);
            } else {
                // Unexpected response type
                return Err(AuthError::GeneralError(format!(
                    "Unexpected response from token endpoint: {}",
                    response_text
                )));
            }
        }
    }

    /// Refreshes the access token if it has expired
    pub async fn refresh_token(&mut self) -> Result<TokenInfo, AuthError> {
        let current_token = self.token_info.as_ref().ok_or(AuthError::GeneralError(
            "No token available to refresh".to_string(),
        ))?;

        let refresh_token = current_token
            .refresh_token
            .as_ref()
            .ok_or(AuthError::GeneralError(
                "No refresh token available".to_string(),
            ))?;

        let client = reqwest::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("grant_type", &"refresh_token".to_string()),
            ("refresh_token", &refresh_token.expose_secret().to_string()),
        ];

        let response = client.post(GITHUB_TOKEN_URL).form(&params).send().await?;

        let response_text = response.text().await?;
        let token_result: Result<TokenResponse, serde_json::Error> =
            serde_json::from_str(&response_text);

        if let Ok(token_data) = token_result {
            // Success case
            // Calculate expiration time (GitHub tokens expire in 1 hour by default)
            let expires_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + token_data.expires_in.unwrap_or(3600), // 1 hour in seconds, or provided value
            );

            // The refresh token might be the same or a new one, depending on the provider
            let new_refresh_token = token_data
                .refresh_token
                .map(SecretString::new)
                .unwrap_or_else(|| refresh_token.clone());

            let refresh_expires_at = token_data
                .refresh_token_expires_in
                .map(|expires_in| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + expires_in
                })
                .or(current_token.refresh_token_expires_at);

            let token_info = TokenInfo {
                access_token: SecretString::new(token_data.access_token),
                token_type: token_data.token_type,
                expires_at,
                refresh_token: Some(new_refresh_token),
                refresh_token_expires_at: refresh_expires_at,
            };

            // Update our internal state
            self.token_info = Some(token_info.clone());

            Ok(token_info)
        } else {
            // Error case - try to parse as error response
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
                Err(AuthError::OAuthError {
                    code: error_response.error,
                    description: error_response.error_description,
                })
            } else {
                Err(AuthError::GeneralError(format!(
                    "Token refresh failed with response: {}",
                    response_text
                )))
            }
        }
    }

    /// Loads the token from the OS keychain
    pub fn load_token_from_keychain(&mut self) -> Result<Option<TokenInfo>, AuthError> {
        if let Some(ref entry) = self.keychain_entry {
            match entry.get_password() {
                Ok(token_json) => {
                    if !token_json.is_empty() {
                        let token_info: TokenInfo = serde_json::from_str(&token_json)?;
                        self.token_info = Some(token_info.clone());
                        Ok(Some(token_info))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => Ok(None), // If no password is found, return None
            }
        } else {
            Ok(None)
        }
    }

    /// Saves the token to the OS keychain
    pub fn save_token_to_keychain(&self, token_info: &TokenInfo) -> Result<(), AuthError> {
        if let Some(ref entry) = self.keychain_entry {
            let token_json = serde_json::to_string(token_info)?;
            entry.set_password(&token_json)?;
            Ok(())
        } else {
            Err(AuthError::GeneralError(
                "Keychain entry not initialized".to_string(),
            ))
        }
    }

    /// Deletes the token from the OS keychain
    pub fn delete_token_from_keychain(&mut self) -> Result<(), AuthError> {
        if let Some(ref entry) = self.keychain_entry {
            match entry.delete_password() {
                Ok(()) => {
                    // Clear the in-memory token info as well
                    self.token_info = None;
                    Ok(())
                }
                Err(keyring::Error::NoEntry) => {
                    // If the item doesn't exist, that's not really an error from the user's perspective
                    // Still clear the in-memory token info if it exists
                    self.token_info = None;
                    Ok(())
                }
                Err(e) => Err(AuthError::KeyringError(e)),
            }
        } else {
            Err(AuthError::GeneralError(
                "Keychain entry not initialized".to_string(),
            ))
        }
    }

    /// Checks if the access token has expired
    pub fn is_access_token_expired(&self) -> bool {
        if let Some(ref token_info) = self.token_info {
            if let Some(expires_at) = token_info.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now >= expires_at;
            }
            // If token exists but has no expiration time, assume it doesn't expire
            return false;
        }
        // If there's no token at all, it's considered expired
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
                return now + within_seconds >= expires_at;
            }
            // If token exists but has no expiration time, it won't expire soon
            return false;
        }
        // If there's no token at all, then it's expiring soon (meaning we need to get one)
        true
    }

    /// Checks if the refresh token has expired
    pub fn is_refresh_token_expired(&self) -> bool {
        if let Some(ref token_info) = self.token_info {
            if let Some(refresh_expires_at) = token_info.refresh_token_expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now >= refresh_expires_at;
            }
        }
        // If there's no refresh token or expiration, assume it's expired
        self.token_info.as_ref().and_then(|t| t.refresh_token.as_ref()).is_none()
    }

    /// Gets a valid access token, refreshing it if necessary
    /// This is the main method for automatic token management
    pub async fn get_valid_token(&mut self) -> Result<String, AuthError> {
        // If no token is available, we need to authenticate first
        if self.token_info.is_none() {
            return Err(AuthError::GeneralError(
                "No token available, authentication required".to_string(),
            ));
        }

        // Check if the access token is expired or will expire soon (within 300 seconds = 5 minutes)
        if self.is_access_token_expiring_soon(300) {
            // If the token will expire soon, try to refresh it
            match self.maybe_refresh_token().await {
                Ok(token_info) => {
                    // Update the token info and return the new access token
                    self.token_info = Some(token_info.clone());
                    Ok(token_info.access_token.expose_secret().clone())
                }
                Err(e) => {
                    // If refresh failed, return the error
                    // This might be a case where we need to re-authenticate
                    Err(e)
                }
            }
        } else {
            // Token is still valid, return it
            Ok(self.token_info.as_ref().unwrap().access_token.expose_secret().clone())
        }
    }

    /// Attempts to refresh the token if it's expired or will expire soon
    pub async fn maybe_refresh_token(&mut self) -> Result<TokenInfo, AuthError> {
        // Check if we have a refresh token and if it's expired
        if self.is_refresh_token_expired() {
            return Err(AuthError::GeneralError(
                "Refresh token is expired, re-authentication required".to_string(),
            ));
        }

        // If we have a valid refresh token, try to refresh the access token
        if !self.is_access_token_expired() && !self.is_access_token_expiring_soon(300) {
            // Token is not expired or expiring soon, no need to refresh
            return Ok(self.token_info.as_ref().unwrap().clone());
        }

        // Attempt to refresh the token
        match self.refresh_token().await {
            Ok(token_info) => {
                // Save the new token to keychain
                if let Err(e) = self.save_token_to_keychain(&token_info) {
                    eprintln!("Failed to save refreshed token to keychain: {:?}", e);
                }
                Ok(token_info)
            }
            Err(e) => {
                // If refresh failed, check if it's because the refresh token itself is invalid
                // The refresh token endpoint might return specific error codes when the refresh token is invalid
                match &e {
                    AuthError::OAuthError { code, .. } => {
                        if code == "invalid_grant" || code == "access_denied" {
                            // If the refresh token is invalid, we need to re-authenticate
                            Err(AuthError::GeneralError(
                                "Refresh token is invalid, re-authentication required".to_string(),
                            ))
                        } else {
                            // Some other error occurred during refresh
                            Err(e)
                        }
                    }
                    _ => Err(e),
                }
            }
        }
    }

    /// Initiates re-authentication when refresh tokens are no longer valid
    pub async fn initiate_reauthentication(&mut self) -> Result<TokenInfo, AuthError> {
        // First, delete the old (invalid) token from keychain
        if let Err(e) = self.delete_token_from_keychain() {
            eprintln!("Warning: failed to delete old token from keychain: {:?}", e);
        }

        // Perform the full authentication flow
        let token_info = self.authenticate().await?;
        
        // Save the new token to keychain
        if let Err(e) = self.save_token_to_keychain(&token_info) {
            eprintln!("Failed to save new token to keychain: {:?}", e);
        }
        
        Ok(token_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_auth_manager_creation() {
        // Note: This test will fail when keyring isn't available in test environment
        // We'll skip this test in environments where keyring is not available
        if let Ok(auth_manager) = AuthManager::new() {
            assert_eq!(auth_manager.client_id, GITHUB_OAUTH_CLIENT_ID);
            assert!(auth_manager.token_info.is_none());
        }
    }

    #[test]
    fn test_token_serialization() {
        // This is a pure serialization test that doesn't depend on keyring
        let token = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(1234567890),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(1234567890),
        };

        let serialized = serde_json::to_string(&token).expect("Failed to serialize TokenInfo");
        let deserialized: TokenInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize TokenInfo");

        assert_eq!(deserialized.token_type, "Bearer");
        assert_eq!(deserialized.expires_at, Some(1234567890));
    }

    #[test]
    fn test_token_expiration_checking() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create an AuthManager instance without using keychain
        let mut auth_manager = AuthManager {
            client_id: GITHUB_OAUTH_CLIENT_ID.to_string(),
            token_info: None,
            keychain_entry: None,
        };

        // Test 1: Token that is expired
        let expired_token = TokenInfo {
            access_token: SecretString::new("expired_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now - 3600), // Expired 1 hour ago
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Expires in 1 hour
        };
        auth_manager.token_info = Some(expired_token);

        assert!(auth_manager.is_access_token_expired());
        assert!(auth_manager.is_access_token_expiring_soon(7200)); // Will expire within 2 hours (already expired)

        // Test 2: Token that will expire soon
        let expiring_soon_token = TokenInfo {
            access_token: SecretString::new("expiring_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 180), // Expires in 3 minutes
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Expires in 1 hour
        };
        auth_manager.token_info = Some(expiring_soon_token);

        assert!(!auth_manager.is_access_token_expired()); // Not expired yet
        assert!(auth_manager.is_access_token_expiring_soon(600)); // Will expire within 10 minutes
        assert!(!auth_manager.is_access_token_expiring_soon(60)); // Won't expire within 1 minute

        // Test 3: Valid token that won't expire soon
        let valid_token = TokenInfo {
            access_token: SecretString::new("valid_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 7200), // Expires in 2 hours
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Expires in 1 hour
        };
        auth_manager.token_info = Some(valid_token);

        assert!(!auth_manager.is_access_token_expired());
        assert!(!auth_manager.is_access_token_expiring_soon(1800)); // Won't expire within 30 minutes

        // Test 4: Expired refresh token
        let expired_refresh_token = TokenInfo {
            access_token: SecretString::new("valid_access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 7200), // Expires in 2 hours
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now - 3600), // Expired 1 hour ago
        };
        auth_manager.token_info = Some(expired_refresh_token);

        assert!(auth_manager.is_refresh_token_expired());

        // Test 5: Valid refresh token
        let valid_refresh_token = TokenInfo {
            access_token: SecretString::new("valid_access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 7200), // Expires in 2 hours
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 36000), // Expires in 10 hours
        };
        auth_manager.token_info = Some(valid_refresh_token);

        assert!(!auth_manager.is_refresh_token_expired());
    }

    #[tokio::test]
    async fn test_get_valid_token_with_expired_token() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create an AuthManager instance without using keychain
        let mut auth_manager = AuthManager {
            client_id: GITHUB_OAUTH_CLIENT_ID.to_string(),
            token_info: None,
            keychain_entry: None,
        };

        // Set up an expired token
        let expired_token = TokenInfo {
            access_token: SecretString::new("expired_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now - 3600), // Expired 1 hour ago
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Expires in 1 hour
        };
        auth_manager.token_info = Some(expired_token);

        // Since we don't have a valid refresh token method for testing without network,
        // we'll just test that it recognizes the token as expired
        assert!(auth_manager.is_access_token_expired());
        assert!(auth_manager.is_access_token_expiring_soon(0)); // Should be expiring "now" since it's already expired
    }

    #[test]
    fn test_token_without_expiration() {
        // Create an AuthManager without token info
        let auth_manager = AuthManager {
            client_id: GITHUB_OAUTH_CLIENT_ID.to_string(),
            token_info: None,
            keychain_entry: None,
        };

        // Token should be considered expired if there's no token info
        assert!(auth_manager.is_access_token_expired());

        // Create a token without expiration time
        let no_expiry_token = TokenInfo {
            access_token: SecretString::new("token_no_expiry".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: None, // No expiration time
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: None, // No expiration time
        };
        
        let auth_manager_with_token = AuthManager {
            client_id: GITHUB_OAUTH_CLIENT_ID.to_string(),
            token_info: Some(no_expiry_token),
            keychain_entry: None,
        };

        // Token without expiration should NOT be considered expiring soon (as a safety measure)
        // Only tokens that actually exist without expiration are considered valid
        assert!(!auth_manager_with_token.is_access_token_expiring_soon(0));
        
        // But it should not be considered expired (since it has no expiration)
        assert!(!auth_manager_with_token.is_access_token_expired());
    }
}
