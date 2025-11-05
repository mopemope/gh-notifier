use crate::{AuthError, DeviceAuthResponse, ErrorResponse, TokenInfo, TokenResponse};
use secrecy::{ExposeSecret, SecretString};

impl super::AuthManager {
    /// Performs the OAuth Device Flow to authenticate the user and obtain an access token
    pub async fn authenticate(&mut self) -> Result<TokenInfo, AuthError> {
        // Step 1: Request device code from GitHub
        let client = reqwest::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("scope", &"notifications".to_string()),
        ];

        let response = client
            .post(crate::auth::core::GITHUB_DEVICE_AUTHORIZATION_URL)
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
                .post(crate::auth::core::GITHUB_TOKEN_URL)
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
            .post(super::core::GITHUB_TOKEN_URL)
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
