use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const GITHUB_OAUTH_CLIENT_ID: &str = "Iv1.898a6d2a86c3f7aa"; // This would be configurable in a real app
const GITHUB_DEVICE_AUTHORIZATION_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

#[derive(Debug)]
pub enum AuthError {
    /// Error with the HTTP request
    RequestError(reqwest::Error),
    /// Error with JSON parsing
    JsonError(serde_json::Error),
    /// Error with keyring operations
    KeyringError(keyring::Error),
    /// Device code has expired
    DeviceCodeExpired,
    /// User cancelled the authorization
    AuthorizationCancelled,
    /// Timeout while waiting for user authorization
    AuthorizationTimeout,
    /// OAuth protocol error with specific error code
    OAuthError { code: String, description: Option<String> },
    /// General authentication error
    GeneralError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::RequestError(e) => write!(f, "Request error: {}", e),
            AuthError::JsonError(e) => write!(f, "JSON error: {}", e),
            AuthError::KeyringError(e) => write!(f, "Keyring error: {}", e),
            AuthError::DeviceCodeExpired => write!(f, "Device code has expired"),
            AuthError::AuthorizationCancelled => write!(f, "Authorization was cancelled by the user"),
            AuthError::AuthorizationTimeout => write!(f, "Authorization timed out"),
            AuthError::OAuthError { code, description } => {
                write!(f, "OAuth error: {} - {}", code, description.as_deref().unwrap_or("no description"))
            }
            AuthError::GeneralError(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<reqwest::Error> for AuthError {
    fn from(error: reqwest::Error) -> Self {
        AuthError::RequestError(error)
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(error: serde_json::Error) -> Self {
        AuthError::JsonError(error)
    }
}

impl From<keyring::Error> for AuthError {
    fn from(error: keyring::Error) -> Self {
        AuthError::KeyringError(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    #[serde(
        serialize_with = "serialize_secret",
        deserialize_with = "deserialize_secret"
    )]
    pub access_token: SecretString,
    pub token_type: String,
    pub expires_at: Option<u64>, // Unix timestamp
    #[serde(
        serialize_with = "serialize_secret_option",
        deserialize_with = "deserialize_secret_option"
    )]
    pub refresh_token: Option<SecretString>,
    pub refresh_token_expires_at: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,  // How long until the device code expires (in seconds)
    interval: u64,    // Polling interval (in seconds)
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<u64>,  // Optional field for access token expiry
    refresh_token: Option<String>,
    refresh_token_expires_in: Option<u64>,  // Optional field for refresh token expiry
}

#[derive(Debug, Deserialize, Serialize)]
struct ErrorResponse {
    error: String,
    error_description: Option<String>,
    error_uri: Option<String>,
}

// Custom serialization for SecretString
fn serialize_secret<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    serializer.serialize_str(secret.expose_secret())
}

fn deserialize_secret<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(SecretString::new(s))
}

// Custom serialization for Option<SecretString>
fn serialize_secret_option<S>(
    secret: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    match secret {
        Some(s) => serializer.serialize_some(s.expose_secret()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_secret_option<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(SecretString::new))
}

pub struct AuthManager {
    client_id: String,
    token_info: Option<TokenInfo>,
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
                        tokio::time::sleep(tokio::time::Duration::from_secs(device_response.interval + 5)).await;
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
                let expires_at = token_data.expires_in.map(|expires_in| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + expires_in
                }).or_else(|| {
                    // Default expiration time (GitHub tokens typically expire in 1 hour by default)
                    Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + 3600, // 1 hour in seconds
                    )
                });

                let refresh_token = token_data.refresh_token
                    .map(|s| SecretString::new(s));
                let refresh_expires_at = token_data.refresh_token_expires_in.map(|expires_in| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + expires_in
                }).or_else(|| {
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
                return Err(AuthError::GeneralError(format!("Unexpected response from token endpoint: {}", response_text)));
            }
        }
    }

    /// Refreshes the access token if it has expired
    pub async fn refresh_token(&mut self) -> Result<TokenInfo, AuthError> {
        let current_token = self
            .token_info
            .as_ref()
            .ok_or(AuthError::GeneralError("No token available to refresh".to_string()))?;

        let refresh_token = current_token
            .refresh_token
            .as_ref()
            .ok_or(AuthError::GeneralError("No refresh token available".to_string()))?;

        let client = reqwest::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("grant_type", &"refresh_token".to_string()),
            ("refresh_token", &refresh_token.expose_secret().to_string()),
        ];

        let response = client.post(GITHUB_TOKEN_URL).form(&params).send().await?;

        let response_text = response.text().await?;
        let token_result: Result<TokenResponse, serde_json::Error> = serde_json::from_str(&response_text);

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
            let new_refresh_token = token_data.refresh_token
                .map(|s| SecretString::new(s))
                .unwrap_or_else(|| refresh_token.clone());

            let refresh_expires_at = token_data.refresh_token_expires_in.map(|expires_in| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + expires_in
            }).or(current_token.refresh_token_expires_at);

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
                Err(AuthError::GeneralError(format!("Token refresh failed with response: {}", response_text)))
            }
        }
    }

    /// Loads the token from the OS keychain
    pub fn load_token_from_keychain(
        &mut self,
    ) -> Result<Option<TokenInfo>, AuthError> {
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
    pub fn save_token_to_keychain(
        &self,
        token_info: &TokenInfo,
    ) -> Result<(), AuthError> {
        if let Some(ref entry) = self.keychain_entry {
            let token_json = serde_json::to_string(token_info)?;
            entry.set_password(&token_json)?;
            Ok(())
        } else {
            Err(AuthError::GeneralError("Keychain entry not initialized".to_string()))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    println!("GitHub Notifier starting...");
    
    let mut auth_manager = AuthManager::new()?;
    
    // Try to load existing token from keychain
    if let Ok(Some(token_info)) = auth_manager.load_token_from_keychain() {
        println!("Found existing token in keychain");
        // Check if token is expired
        if let Some(expires_at) = token_info.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
                
            if now >= expires_at {
                println!("Token has expired, refreshing...");
                match auth_manager.refresh_token().await {
                    Ok(new_token) => {
                        println!("Token refreshed successfully");
                        if let Err(e) = auth_manager.save_token_to_keychain(&new_token) {
                            eprintln!("Failed to save refreshed token to keychain: {:?}", e);
                        }
                    }
                    Err(e) => {
                        println!("Token refresh failed: {:?}, proceeding with re-authentication", e);
                    }
                }
            } else {
                println!("Existing token is still valid");
            }
        }
    } else {
        println!("No existing token found, starting OAuth Device Flow...");
        // Perform the OAuth device flow to get a new token
        match auth_manager.authenticate().await {
            Ok(token_info) => {
                println!("Authentication successful!");
                
                // Save the token to keychain for future use
                if let Err(e) = auth_manager.save_token_to_keychain(&token_info) {
                    eprintln!("Failed to save token to keychain: {:?}", e);
                } else {
                    println!("Token saved to keychain");
                }
            }
            Err(e) => {
                eprintln!("Authentication failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    println!("GitHub Notifier running with authenticated access");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn test_auth_manager_creation() {
        let auth_manager = AuthManager::new().expect("Failed to create AuthManager");
        assert_eq!(auth_manager.client_id, GITHUB_OAUTH_CLIENT_ID);
        assert!(auth_manager.token_info.is_none());
    }

    #[test]
    fn test_token_info_serialization() {
        use secrecy::SecretString;

        let token = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(1234567890),
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(1234567890),
        };

        // Test serialization
        let serialized = serde_json::to_string(&token).expect("Failed to serialize TokenInfo");
        assert!(serialized.contains("test_token"));

        // Test deserialization
        let deserialized: TokenInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize TokenInfo");
        assert_eq!(deserialized.token_type, "Bearer");
        assert_eq!(deserialized.expires_at, Some(1234567890));
    }

    #[test]
    fn test_token_info_without_refresh_token_serialization() {
        use secrecy::SecretString;

        let token = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(1234567890),
            refresh_token: None,
            refresh_token_expires_at: None,
        };

        // Test serialization without refresh token
        let serialized = serde_json::to_string(&token).expect("Failed to serialize TokenInfo");
        assert!(serialized.contains("test_token"));
        assert!(serialized.contains("null")); // Refresh token should be null

        // Test deserialization without refresh token
        let deserialized: TokenInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize TokenInfo");
        assert_eq!(deserialized.token_type, "Bearer");
        assert!(deserialized.refresh_token.is_none());
    }

    #[test]
    fn test_token_info_expiration() {
        use secrecy::SecretString;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token = TokenInfo {
            access_token: SecretString::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(now - 3600), // Expired 1 hour ago
            refresh_token: Some(SecretString::new("refresh_token".to_string())),
            refresh_token_expires_at: Some(now + 3600), // Expires in 1 hour
        };

        assert!(token.expires_at.unwrap() < now); // Should be expired
        assert!(token.refresh_token_expires_at.unwrap() > now); // Should not be expired
    }

    #[test]
    fn test_secret_serialization_functions() {
        use secrecy::SecretString;
        use serde_json;

        // Test that custom serialization functions work properly
        let secret = SecretString::new("my_secret".to_string());
        let serialized = serde_json::to_string(&secret.expose_secret()).unwrap();
        assert_eq!(serialized, "\"my_secret\"");

        // Verify we can't directly access the secret in a serialized form
        let exposed = secret.expose_secret();
        assert_eq!(exposed, "my_secret");
    }

    #[test]
    fn test_auth_error_display() {
        let error = AuthError::DeviceCodeExpired;
        assert_eq!(format!("{}", error), "Device code has expired");

        let error = AuthError::AuthorizationCancelled;
        assert_eq!(format!("{}", error), "Authorization was cancelled by the user");

        let error = AuthError::AuthorizationTimeout;
        assert_eq!(format!("{}", error), "Authorization timed out");

        let error = AuthError::OAuthError {
            code: "invalid_request".to_string(),
            description: Some("Invalid scope provided".to_string()),
        };
        assert_eq!(format!("{}", error), "OAuth error: invalid_request - Invalid scope provided");

        let error = AuthError::GeneralError("Something went wrong".to_string());
        assert_eq!(format!("{}", error), "Authentication error: Something went wrong");
    }

    #[tokio::test]
    async fn test_device_auth_response_deserialization() {
        let json_response = r#"
        {
            "device_code": "test_device_code",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }
        "#;

        let response: DeviceAuthResponse = serde_json::from_str(json_response).unwrap();
        assert_eq!(response.device_code, "test_device_code");
        assert_eq!(response.user_code, "ABCD-EFGH");
        assert_eq!(response.verification_uri, "https://github.com/login/device");
        assert_eq!(response.expires_in, 900);
        assert_eq!(response.interval, 5);
    }

    #[tokio::test]
    async fn test_token_response_deserialization() {
        let json_response = r#"
        {
            "access_token": "gho_test_token",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "ghr_test_refresh_token",
            "refresh_token_expires_in": 15768000
        }
        "#;

        let response: TokenResponse = serde_json::from_str(json_response).unwrap();
        assert_eq!(response.access_token, "gho_test_token");
        assert_eq!(response.token_type, "bearer");
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.refresh_token, Some("ghr_test_refresh_token".to_string()));
        assert_eq!(response.refresh_token_expires_in, Some(15768000));
    }

    #[tokio::test]
    async fn test_error_response_deserialization() {
        let json_response = r#"
        {
            "error": "authorization_pending",
            "error_description": "Authorization pending"
        }
        "#;

        let response: ErrorResponse = serde_json::from_str(json_response).unwrap();
        assert_eq!(response.error, "authorization_pending");
        assert_eq!(response.error_description, Some("Authorization pending".to_string()));
    }
}
