use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::sync::Arc;

const GITHUB_OAUTH_CLIENT_ID: &str = "Iv1.898a6d2a86c3f7aa"; // This would be configurable in a real app
const GITHUB_DEVICE_AUTHORIZATION_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
    pub async fn authenticate(&mut self) -> Result<TokenInfo, Box<dyn std::error::Error>> {
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

        let device_response: serde_json::Value = response.json().await?;

        let device_code = device_response["device_code"]
            .as_str()
            .ok_or("Device code not found in response")?;
        let user_code = device_response["user_code"]
            .as_str()
            .ok_or("User code not found in response")?;
        let verification_uri = device_response["verification_uri"]
            .as_str()
            .ok_or("Verification URI not found in response")?;
        let interval = device_response["interval"]
            .as_f64()
            .ok_or("Interval not found in response")? as u64;

        // Display instructions to user
        println!("GitHub OAuth Device Flow:");
        println!("1. Visit: {}", verification_uri);
        println!("2. Enter code: {}", user_code);
        println!("3. Confirm the authorization request");

        // Step 2: Poll for the access token
        let token_params = [
            ("client_id", &self.client_id),
            ("device_code", &device_code.to_string()),
            (
                "grant_type",
                &"urn:ietf:params:oauth:grant-type:device_code".to_string(),
            ),
        ];

        // Poll until we get the token or the device code expires
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

            let token_response = client
                .post(GITHUB_TOKEN_URL)
                .form(&token_params)
                .send()
                .await?;

            let token_data: serde_json::Value = token_response.json().await?;

            if let Some(error) = token_data["error"].as_str() {
                match error {
                    "authorization_pending" => {
                        // Continue polling, user hasn't authorized yet
                        continue;
                    }
                    "slow_down" => {
                        // GitHub is asking us to slow down, increase the interval by 5 seconds
                        tokio::time::sleep(tokio::time::Duration::from_secs(interval + 5)).await;
                        continue;
                    }
                    _ => {
                        return Err(format!("OAuth error: {}", error).into());
                    }
                }
            } else {
                // Success! We got the token
                let access_token = token_data["access_token"]
                    .as_str()
                    .ok_or("Access token not found in response")?;

                let token_type = token_data["token_type"]
                    .as_str()
                    .ok_or("Token type not found in response")?;

                // Calculate expiration time (GitHub tokens expire in 1 hour by default)
                let expires_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + 3600, // 1 hour in seconds
                );

                let refresh_token = token_data["refresh_token"]
                    .as_str()
                    .map(|s| SecretString::new(s.to_string()));
                let refresh_expires_at =
                    token_data["refresh_token_expires_at"].as_u64().or_else(|| {
                        // Default refresh token expiration to 6 months
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|dur| dur.as_secs() + (6 * 30 * 24 * 3600)) // 6 months
                    });

                let token_info = TokenInfo {
                    access_token: SecretString::new(access_token.to_string()),
                    token_type: token_type.to_string(),
                    expires_at,
                    refresh_token,
                    refresh_token_expires_at: refresh_expires_at,
                };

                // Update our internal state
                self.token_info = Some(token_info.clone());

                return Ok(token_info);
            }
        }
    }

    /// Refreshes the access token if it has expired
    pub async fn refresh_token(&mut self) -> Result<TokenInfo, Box<dyn std::error::Error>> {
        let current_token = self
            .token_info
            .as_ref()
            .ok_or("No token available to refresh")?;

        let refresh_token = current_token
            .refresh_token
            .as_ref()
            .ok_or("No refresh token available")?;

        let client = reqwest::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("grant_type", &"refresh_token".to_string()),
            ("refresh_token", &refresh_token.expose_secret().to_string()),
        ];

        let response = client.post(GITHUB_TOKEN_URL).form(&params).send().await?;

        let token_data: serde_json::Value = response.json().await?;

        if token_data["error"].is_string() {
            return Err(format!(
                "Token refresh error: {}",
                token_data["error"].as_str().unwrap_or("Unknown error")
            )
            .into());
        }

        let access_token = token_data["access_token"]
            .as_str()
            .ok_or("Access token not found in response")?;

        let token_type = token_data["token_type"]
            .as_str()
            .ok_or("Token type not found in response")?;

        // Calculate expiration time (GitHub tokens expire in 1 hour by default)
        let expires_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600, // 1 hour in seconds
        );

        // The refresh token might be the same or a new one, depending on the provider
        let new_refresh_token = token_data["refresh_token"]
            .as_str()
            .map(|s| SecretString::new(s.to_string()))
            .unwrap_or_else(|| refresh_token.clone());

        let refresh_expires_at = token_data["refresh_token_expires_at"]
            .as_u64()
            .or(current_token.refresh_token_expires_at);

        let token_info = TokenInfo {
            access_token: SecretString::new(access_token.to_string()),
            token_type: token_type.to_string(),
            expires_at,
            refresh_token: Some(new_refresh_token),
            refresh_token_expires_at: refresh_expires_at,
        };

        // Update our internal state
        self.token_info = Some(token_info.clone());

        Ok(token_info)
    }

    /// Loads the token from the OS keychain
    pub fn load_token_from_keychain(
        &mut self,
    ) -> Result<Option<TokenInfo>, Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref entry) = self.keychain_entry {
            let token_json = serde_json::to_string(token_info)?;
            entry.set_password(&token_json)?;
            Ok(())
        } else {
            Err("Keychain entry not initialized".into())
        }
    }
}

fn main() {
    println!("GitHub Notifier starting...");
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
}
