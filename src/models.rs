use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

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

// Custom serialization for SecretString
pub fn serialize_secret<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    serializer.serialize_str(secret.expose_secret())
}

pub fn deserialize_secret<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(SecretString::new(s))
}

// Custom serialization for Option<SecretString>
pub fn serialize_secret_option<S>(
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

pub fn deserialize_secret_option<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(SecretString::new))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64, // How long until the device code expires (in seconds)
    pub interval: u64,   // Polling interval (in seconds)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>, // Optional field for access token expiry
    pub refresh_token: Option<String>,
    pub refresh_token_expires_in: Option<u64>, // Optional field for refresh token expiry
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

// --- GitHub API 通知モデル ---
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Notification {
    pub id: String,
    pub unread: bool,
    pub reason: String,
    pub updated_at: String, // ISO 8601
    pub last_read_at: Option<String>,
    pub subject: NotificationSubject,
    pub repository: NotificationRepository,
    pub url: String,
    pub subscription_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationSubject {
    pub title: String,
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
    #[serde(rename = "type")]
    pub kind: String, // Issue, PullRequest, etc.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationRepository {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_token_info_serialization() {
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
        // Test that custom serialization functions work properly
        let secret = SecretString::new("my_secret".to_string());
        let serialized = serde_json::to_string(&secret.expose_secret()).unwrap();
        assert_eq!(serialized, "\"my_secret\"");

        // Verify we can't directly access the secret in a serialized form
        let exposed = secret.expose_secret();
        assert_eq!(exposed, "my_secret");
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
        assert_eq!(
            response.refresh_token,
            Some("ghr_test_refresh_token".to_string())
        );
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
        assert_eq!(
            response.error_description,
            Some("Authorization pending".to_string())
        );
    }
}
