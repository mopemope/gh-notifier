use keyring;

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
    OAuthError {
        code: String,
        description: Option<String>,
    },
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
            AuthError::AuthorizationCancelled => {
                write!(f, "Authorization was cancelled by the user")
            }
            AuthError::AuthorizationTimeout => write!(f, "Authorization timed out"),
            AuthError::OAuthError { code, description } => {
                write!(
                    f,
                    "OAuth error: {} - {}",
                    code,
                    description.as_deref().unwrap_or("no description")
                )
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
