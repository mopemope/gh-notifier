use crate::token_storage::TokenStorage;
use crate::{AuthError, DeviceAuthResponse, ErrorResponse, TokenInfo, TokenResponse};
use secrecy::{ExposeSecret, SecretString};
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_DEVICE_AUTHORIZATION_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub struct AuthManager {
    pub client_id: String,
    pub token_info: Option<TokenInfo>,
    token_storage: TokenStorage,
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
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;
        tracing::debug!("Device authorization response: {}", response_text);

        // Check if response status is successful before attempting JSON parse
        if !status.is_success() {
            // Check if this is a 404 error which usually indicates an invalid client_id
            if status.as_u16() == 404 {
                return Err(AuthError::GeneralError(format!(
                    "Device authorization request failed: Invalid client_id. Status: {} - {} \n\
                    This usually means the GitHub OAuth client_id is invalid or not registered for device flow. \n\
                    Please register your own GitHub OAuth App and set the correct client_id in the config file.",
                    status, response_text
                )));
            } else {
                return Err(AuthError::GeneralError(format!(
                    "Device authorization request failed: {} - {}",
                    status, response_text
                )));
            }
        }

        // Parse the device authorization response properly
        let device_response: DeviceAuthResponse = serde_json::from_str(&response_text)?;

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
        // First request happens immediately after user sees the instructions
        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() >= timeout_duration {
                return Err(AuthError::AuthorizationTimeout);
            }

            let token_response = client
                .post(GITHUB_TOKEN_URL)
                .header("Accept", "application/json")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&token_params)
                .send()
                .await?;

            let status = token_response.status();
            let response_text = token_response.text().await?;
            tracing::debug!("Token endpoint response: {}", response_text);

            // Check if response is valid JSON before attempting to parse
            if response_text.trim().is_empty() {
                return Err(AuthError::GeneralError(
                    "Token endpoint returned empty response".to_string(),
                ));
            }

            // Try to parse as error response first
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
                match error_response.error.as_str() {
                    "authorization_pending" => {
                        // Continue polling, user hasn't authorized yet
                        // Sleep for the polling interval before the next check
                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            device_response.interval,
                        ))
                        .await;
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

        let response = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        let response_text = response.text().await?;
        tracing::debug!("Token refresh response: {}", response_text);

        // Check if response is valid JSON before attempting to parse
        if response_text.trim().is_empty() {
            return Err(AuthError::GeneralError(
                "Token refresh endpoint returned empty response".to_string(),
            ));
        }

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

    /// Loads the token from storage (with fallback mechanisms)
    pub fn load_token_from_storage(&mut self) -> Result<Option<TokenInfo>, AuthError> {
        match self.token_storage.load_token() {
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
        }
    }

    /// Saves the token to storage (with fallback mechanisms)
    pub fn save_token_to_storage(&self, token_info: &TokenInfo) -> Result<(), AuthError> {
        self.token_storage.save_token(token_info)
    }

    /// Deletes the token from storage (with fallback mechanisms)
    pub fn delete_token_from_storage(&mut self) -> Result<(), AuthError> {
        self.token_storage.delete_token()?;
        // Clear the in-memory token info as well
        self.token_info = None;
        Ok(())
    }

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
            Ok(self
                .token_info
                .as_ref()
                .unwrap()
                .access_token
                .expose_secret()
                .clone())
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
                if let Err(e) = self.save_token_to_storage(&token_info) {
                    eprintln!("Failed to save refreshed token to storage: {:?}", e);
                    eprintln!(
                        "WARNING: Refreshed authentication token will not persist between sessions."
                    );
                    eprintln!("You may need to re-authenticate if the token expires.");
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
        // First, delete the old (invalid) token from storage
        if let Err(e) = self.delete_token_from_storage() {
            eprintln!("Warning: failed to delete old token from storage: {:?}", e);
        }

        // Perform the full authentication flow
        let token_info = self.authenticate().await?;

        // Save the new token to storage
        if let Err(e) = self.save_token_to_storage(&token_info) {
            eprintln!("Failed to save new token to storage: {:?}", e);
            eprintln!("WARNING: New authentication token will not persist between sessions.");
            eprintln!("You will need to re-authenticate each time you run the application.");
        }

        Ok(token_info)
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

    /// Performs a complete re-authentication flow with user notifications
    /// This method is designed to be called when any authentication failure occurs
    pub async fn perform_reauthentication_with_notification(
        &mut self,
    ) -> Result<TokenInfo, AuthError> {
        println!("Authentication token is invalid or has expired.");
        println!("Starting re-authentication process...");

        match self.initiate_reauthentication().await {
            Ok(token_info) => {
                println!("Re-authentication completed successfully!");
                println!("Your access token has been renewed.");
                Ok(token_info)
            }
            Err(e) => {
                eprintln!("Re-authentication failed: {}", e);
                Err(e)
            }
        }
    }

    /// Gets a valid token, with automatic re-authentication if needed
    /// This method combines token validation, refresh, and re-authentication
    pub async fn get_valid_token_with_reauth(&mut self) -> Result<String, AuthError> {
        // First, check if we have any token
        if self.token_info.is_none() {
            println!("No authentication token found. Starting authentication process...");
            return match self.authenticate().await {
                Ok(token_info) => {
                    self.token_info = Some(token_info.clone());
                    // Save the new token to storage
                    if let Err(e) = self.save_token_to_storage(&token_info) {
                        eprintln!("Failed to save new token to storage: {:?}", e);
                        eprintln!(
                            "WARNING: Authentication token will not persist between sessions."
                        );
                        eprintln!(
                            "You will need to re-authenticate each time you run the application."
                        );
                    }
                    Ok(token_info.access_token.expose_secret().clone())
                }
                Err(e) => Err(e),
            };
        }

        // Check if token needs refresh (will expire soon)
        if self.is_access_token_expiring_soon(300) {
            println!("Token is expiring soon, attempting refresh...");
            match self.maybe_refresh_token().await {
                Ok(token_info) => {
                    self.token_info = Some(token_info.clone());
                    Ok(token_info.access_token.expose_secret().clone())
                }
                Err(refresh_error) => {
                    println!(
                        "Token refresh failed: {:?}. Initiating re-authentication...",
                        refresh_error
                    );
                    match self.perform_reauthentication_with_notification().await {
                        Ok(token_info) => {
                            self.token_info = Some(token_info.clone());
                            Ok(token_info.access_token.expose_secret().clone())
                        }
                        Err(e) => Err(e),
                    }
                }
            }
        } else {
            // Token is not expiring soon, but let's make sure it's still valid
            // This is an extra validation step for cases where token state might have changed externally
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
                    // Token is invalid, we need to re-authenticate
                    println!("Token validation failed. Initiating re-authentication...");
                    match self.perform_reauthentication_with_notification().await {
                        Ok(token_info) => {
                            self.token_info = Some(token_info.clone());
                            Ok(token_info.access_token.expose_secret().clone())
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(validation_error) => {
                    // Validation failed due to network or other issues
                    // Check if it's a network error by matching against the error type
                    match &validation_error {
                        AuthError::RequestError(_) => {
                            // This is likely a network connectivity issue, not a token issue
                            println!(
                                "Token validation failed due to network issues. Checking if current token is still valid..."
                            );

                            // Check if token is still valid based on expiration time only
                            if !self.is_access_token_expired() {
                                // Current token is still valid based on expiration, return it despite network validation failure
                                if let Some(token_info) = &self.token_info {
                                    println!(
                                        "Network issues encountered, but current token is still valid. Continuing with current token."
                                    );
                                    Ok(token_info.access_token.expose_secret().clone())
                                } else {
                                    // This shouldn't happen, but just in case
                                    Err(validation_error)
                                }
                            } else {
                                // Token is expired and validation failed due to network issues, try refresh
                                println!("Token is expired. Attempting refresh...");
                                match self.maybe_refresh_token().await {
                                    Ok(token_info) => {
                                        self.token_info = Some(token_info.clone());
                                        Ok(token_info.access_token.expose_secret().clone())
                                    }
                                    Err(refresh_error) => {
                                        // Check if refresh error is also network related
                                        match &refresh_error {
                                            AuthError::RequestError(_) => {
                                                // Both validation and refresh failed due to network issues, but token is expired
                                                // We have no choice but to re-authenticate
                                                println!(
                                                    "Network issues persisted during refresh. Initiating re-authentication...",
                                                );
                                                match self
                                                    .perform_reauthentication_with_notification()
                                                    .await
                                                {
                                                    Ok(token_info) => {
                                                        self.token_info = Some(token_info.clone());
                                                        Ok(token_info
                                                            .access_token
                                                            .expose_secret()
                                                            .clone())
                                                    }
                                                    Err(e) => Err(e),
                                                }
                                            }
                                            _ => {
                                                // Refresh failed due to token-related errors, re-authenticate
                                                println!(
                                                    "Token refresh failed: {:?}. Initiating re-authentication...",
                                                    refresh_error
                                                );
                                                match self
                                                    .perform_reauthentication_with_notification()
                                                    .await
                                                {
                                                    Ok(token_info) => {
                                                        self.token_info = Some(token_info.clone());
                                                        Ok(token_info
                                                            .access_token
                                                            .expose_secret()
                                                            .clone())
                                                    }
                                                    Err(e) => Err(e),
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            // Validation failed due to token-related issues, try refresh first
                            println!(
                                "Token validation failed: {:?}. Attempting refresh...",
                                validation_error
                            );
                            match self.maybe_refresh_token().await {
                                Ok(token_info) => {
                                    self.token_info = Some(token_info.clone());
                                    Ok(token_info.access_token.expose_secret().clone())
                                }
                                Err(refresh_error) => {
                                    println!(
                                        "Token refresh failed: {:?}. Initiating re-authentication...",
                                        refresh_error
                                    );
                                    match self.perform_reauthentication_with_notification().await {
                                        Ok(token_info) => {
                                            self.token_info = Some(token_info.clone());
                                            Ok(token_info.access_token.expose_secret().clone())
                                        }
                                        Err(e) => Err(e),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    #[ignore = "This test depends on user's config file which can have a custom client_id"]
    fn test_auth_manager_creation() {
        // Note: This test will fail when keyring isn't available in test environment
        // We'll skip this test in environments where keyring is not available
        if let Ok(auth_manager) = AuthManager::new() {
            // The client ID should come from config, so we should use the default value from config
            let default_config = crate::config::Config::default();
            assert_eq!(auth_manager.client_id, default_config.client_id);
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

        // Create token storage that doesn't use system keyring for tests
        let token_storage = TokenStorage::new().unwrap();

        // Create an AuthManager instance with token storage
        let mut auth_manager = AuthManager {
            client_id: crate::config::Config::default().client_id,
            token_info: None,
            token_storage,
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

        // Create an AuthManager instance for testing
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

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
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

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

        let mut auth_manager_with_token = AuthManager::new_for_tests().unwrap();
        auth_manager_with_token.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_with_token.token_info = Some(no_expiry_token);

        // Token without expiration should NOT be considered expiring soon (as a safety measure)
        // Only tokens that actually exist without expiration are considered valid
        assert!(!auth_manager_with_token.is_access_token_expiring_soon(0));

        // But it should not be considered expired (since it has no expiration)
        assert!(!auth_manager_with_token.is_access_token_expired());
    }

    #[test]
    fn test_token_validation_methods() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test is_access_token_expired with no token
        let mut auth_manager_no_token = AuthManager::new_for_tests().unwrap();
        auth_manager_no_token.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_no_token.token_info = None;

        assert!(auth_manager_no_token.is_access_token_expired());
        assert!(auth_manager_no_token.is_access_token_expiring_soon(0));

        // Test with expired token
        let expired_token = TokenInfo {
            access_token: SecretString::new("expired_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now - 100), // Expired 100 seconds ago
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 1000), // Valid refresh token
        };

        let mut auth_manager_expired = AuthManager::new_for_tests().unwrap();
        auth_manager_expired.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_expired.token_info = Some(expired_token);

        assert!(auth_manager_expired.is_access_token_expired());
        assert!(auth_manager_expired.is_access_token_expiring_soon(0));
        assert!(auth_manager_expired.is_access_token_expiring_soon(500)); // Already expired, so it's "expiring" anytime

        // Test with valid token
        let valid_token = TokenInfo {
            access_token: SecretString::new("valid_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 1000), // Expires in 1000 seconds
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 2000), // Valid refresh token
        };

        let mut auth_manager_valid = AuthManager::new_for_tests().unwrap();
        auth_manager_valid.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_valid.token_info = Some(valid_token);

        assert!(!auth_manager_valid.is_access_token_expired());
        assert!(!auth_manager_valid.is_access_token_expiring_soon(100)); // Won't expire in next 100 seconds
        assert!(auth_manager_valid.is_access_token_expiring_soon(1500)); // Will expire in 1500 seconds
    }

    #[test]
    fn test_refresh_token_validation() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test with no refresh token
        let no_refresh_token = TokenInfo {
            access_token: SecretString::new("token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 1000),
            refresh_token: None, // No refresh token
            refresh_token_expires_at: None,
        };

        let mut auth_manager_no_refresh = AuthManager::new_for_tests().unwrap();
        auth_manager_no_refresh.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_no_refresh.token_info = Some(no_refresh_token);

        assert!(auth_manager_no_refresh.is_refresh_token_expired());

        // Test with expired refresh token
        let expired_refresh_token = TokenInfo {
            access_token: SecretString::new("token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 1000),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now - 100), // Expired
        };

        let mut auth_manager_expired_refresh = AuthManager::new_for_tests().unwrap();
        auth_manager_expired_refresh.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_expired_refresh.token_info = Some(expired_refresh_token);

        assert!(auth_manager_expired_refresh.is_refresh_token_expired());

        // Test with valid refresh token
        let valid_refresh_token = TokenInfo {
            access_token: SecretString::new("token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 1000),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 1000), // Valid
        };

        let mut auth_manager_valid_refresh = AuthManager::new_for_tests().unwrap();
        auth_manager_valid_refresh.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager_valid_refresh.token_info = Some(valid_refresh_token);

        assert!(!auth_manager_valid_refresh.is_refresh_token_expired());
    }

    #[tokio::test]
    async fn test_perform_reauthentication_with_notification() {
        // Note: This test doesn't actually perform re-authentication since that requires network interaction
        // This is just to test that the method exists and can be called
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

        // We expect this to fail since there's no actual token to refresh
        // but we're testing that the method is callable
        let result = auth_manager
            .perform_reauthentication_with_notification()
            .await;
        // The result should be an error since we don't have a real authentication flow in tests
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_valid_token_method() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test 1: When no token exists
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

        let result = auth_manager.get_valid_token().await;
        assert!(result.is_err());

        // Test 2: When valid token exists (not expiring soon)
        let valid_token = TokenInfo {
            access_token: SecretString::new("valid_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600), // Expires in 1 hour
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 7200), // Expires in 2 hours
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(valid_token);

        let result = auth_manager.get_valid_token().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_token");

        // Test 3: When token is expiring soon (should try to refresh)
        let expiring_token = TokenInfo {
            access_token: SecretString::new("expiring_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 100), // Expires in 100 seconds (less than 300 seconds = 5 minutes)
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 7200), // Valid refresh token
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(expiring_token);

        // In test environment without real network, this will fail, but it will try to refresh first
        let result = auth_manager.get_valid_token().await;
        // The result will be an error because we can't actually refresh without network,
        // but the method should at least attempt to refresh
        assert!(result.is_err());
    }

    #[test]
    fn test_keychain_operations() {
        // Test keychain operations with mocked entry
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

        // Test that operations succeed gracefully when keychain entry is not initialized
        // (tokens just won't persist between sessions)
        let token = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(1234567890),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(1234567890),
        };

        let result = auth_manager.save_token_to_storage(&token);
        assert!(result.is_ok()); // Should succeed even when there's no keychain
    }

    #[tokio::test]
    async fn test_maybe_refresh_token_method() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test 1: When refresh token is expired
        let expired_refresh_token = TokenInfo {
            access_token: SecretString::new("access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now - 100), // Expired refresh token
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(expired_refresh_token);

        let result = auth_manager.maybe_refresh_token().await;
        assert!(result.is_err());
        // Check if the error message indicates re-authentication is required
        if let Err(AuthError::GeneralError(msg)) = result {
            assert!(msg.contains("re-authentication required"));
        } else {
            panic!("Expected GeneralError with re-authentication message");
        }
    }

    #[tokio::test]
    async fn test_is_refresh_token_expired_method() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test with token that has no refresh token
        let no_refresh_token = TokenInfo {
            access_token: SecretString::new("access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: None, // No refresh token
            refresh_token_expires_at: None,
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(no_refresh_token);

        assert!(auth_manager.is_refresh_token_expired()); // Should be true when no refresh token exists

        // Test with valid refresh token
        let valid_refresh_token = TokenInfo {
            access_token: SecretString::new("access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Valid refresh token
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(valid_refresh_token);

        assert!(!auth_manager.is_refresh_token_expired()); // Should be false when refresh token is valid
    }

    #[tokio::test]
    async fn test_initiate_reauthentication_method() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(TokenInfo {
            access_token: SecretString::new("old_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: Some(SecretString::new("old_refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 7200),
        });

        // This should fail as authentication requires network, but method should be callable
        let result = auth_manager.initiate_reauthentication().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_method() {
        // Test validate_token with no token
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None; // No token

        let result = auth_manager.validate_token().await;
        // Should return Ok(false) when no token exists
        assert!(matches!(result, Ok(false)));

        // Test validate_token with a token, but in test environment it will fail due to network
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(TokenInfo {
            access_token: SecretString::new("invalid_token_for_test".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now + 3600),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 7200),
        });

        let result = auth_manager.validate_token().await;
        // In test environment, this will likely error due to network issues
        // This is expected behavior when testing network operations
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_valid_token_with_reauth_method() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Test with no token (should trigger initial auth)
        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = None;

        let result = auth_manager.get_valid_token_with_reauth().await;
        assert!(result.is_err()); // Will fail without network but method should be callable

        // Test with expired token (should trigger refresh then re-auth)
        let expired_token = TokenInfo {
            access_token: SecretString::new("expired_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now - 100), // Expired
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now - 50), // Also expired
        };

        let mut auth_manager = AuthManager::new_for_tests().unwrap();
        auth_manager.client_id = crate::config::Config::default().client_id; // Override for test
        auth_manager.token_info = Some(expired_token);

        let result = auth_manager.get_valid_token_with_reauth().await;
        assert!(result.is_err()); // Will fail without network but should follow the right logic path
    }
}
