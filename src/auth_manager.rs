use crate::{AuthError, DeviceAuthResponse, ErrorResponse, TokenInfo, TokenResponse};
use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

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
}
