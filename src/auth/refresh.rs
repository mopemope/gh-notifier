use crate::{AuthError, TokenInfo};
use secrecy::{ExposeSecret, SecretString};
use std::io::{self, Write};

impl super::AuthManager {
    /// Performs the PAT (Personal Access Token) authentication to get the token from user
    pub async fn authenticate(&mut self) -> Result<TokenInfo, AuthError> {
        println!("GitHub Personal Access Token Authentication");
        println!("Please enter your GitHub Personal Access Token.");
        println!("If you don't have one, create it at: https://github.com/settings/tokens");
        println!("Make sure to grant the 'notifications' scope for this application.");

        print!("GitHub Personal Access Token: ");
        io::stdout()
            .flush()
            .map_err(|e| AuthError::GeneralError(format!("Failed to flush stdout: {}", e)))?;

        // Read the PAT from standard input (without showing it on screen)
        let token = rpassword::read_password()
            .map_err(|e| AuthError::GeneralError(format!("Failed to read password: {}", e)))?;

        if token.trim().is_empty() {
            return Err(AuthError::GeneralError("Token cannot be empty".to_string()));
        }

        // Create a PAT token info (PATs don't expire by default and don't need refresh tokens)
        let token_info = TokenInfo {
            access_token: SecretString::new(token.trim().to_string()),
            token_type: "Bearer".to_string(),
            expires_at: None,    // PATs don't expire by default
            refresh_token: None, // No refresh token for PAT
            refresh_token_expires_at: None,
        };

        // Update our internal state
        self.token_info = Some(token_info.clone());

        Ok(token_info)
    }

    /// Gets the access token (PATs don't need refresh as they don't expire)
    pub async fn get_valid_token(&mut self) -> Result<String, AuthError> {
        // If no token is available, we need to authenticate first
        if self.token_info.is_none() {
            return Err(AuthError::GeneralError(
                "No token available, authentication required".to_string(),
            ));
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

    /// Gets a valid token, with authentication if needed
    /// This method handles the complete PAT authentication flow
    pub async fn get_valid_token_with_reauth(&mut self) -> Result<String, AuthError> {
        // First, check if we have any token
        if self.token_info.is_none() {
            // Try to load existing token from storage first
            if let Ok(Some(stored_token)) = self.load_token_from_storage() {
                self.token_info = Some(stored_token);
                println!("Using stored Personal Access Token");
            } else {
                println!(
                    "No stored Personal Access Token found. Starting authentication process..."
                );
                let token_info = self.authenticate().await?;

                // Save the new token to storage
                if let Err(e) = self.save_token_to_storage(&token_info) {
                    eprintln!("Failed to save new token to storage: {:?}", e);
                    eprintln!("WARNING: Authentication token will not persist between sessions.");
                    eprintln!(
                        "You will need to re-authenticate each time you run the application."
                    );
                }

                return Ok(token_info.access_token.expose_secret().clone());
            }
        }

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
                // Token is invalid, we need to re-authenticate
                println!(
                    "Stored token validation failed. Invalid or expired Personal Access Token. Re-authenticating..."
                );
                return self.reauthenticate().await;
            }
            Err(validation_error) => {
                // Validation failed due to network or other issues
                // Since PATs don't have expiration, we can still return the token if network issue
                match &validation_error {
                    AuthError::RequestError(_) => {
                        println!(
                            "Token validation failed due to network issues, but returning stored token."
                        );
                        // For network issues, we can still use the stored token
                        Ok(self
                            .token_info
                            .as_ref()
                            .unwrap()
                            .access_token
                            .expose_secret()
                            .clone())
                    }
                    _ => {
                        // For other validation errors, re-authenticate
                        println!(
                            "Token validation failed: {:?}. Re-authenticating...",
                            validation_error
                        );
                        self.reauthenticate().await
                    }
                }
            }
        }
    }

    /// Performs re-authentication with user notification
    async fn reauthenticate(&mut self) -> Result<String, AuthError> {
        // Delete the invalid token from storage
        if let Err(e) = self.delete_token_from_storage() {
            eprintln!(
                "Warning: failed to delete invalid token from storage: {:?}",
                e
            );
        }

        println!("Starting re-authentication process...");
        let token_info = self.authenticate().await?;

        // Save the new token to storage
        if let Err(e) = self.save_token_to_storage(&token_info) {
            eprintln!("Failed to save new token to storage: {:?}", e);
            eprintln!("WARNING: Authentication token will not persist between sessions.");
            eprintln!("You will need to re-authenticate each time you run the application.");
        }

        Ok(token_info.access_token.expose_secret().clone())
    }
}
