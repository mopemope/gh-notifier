use keyring;
use thiserror::Error;

/// アプリケーション全体のエラー型
#[derive(Error, Debug)]
pub enum AppError {
    /// 認証関連エラー
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// GitHub API関連エラー
    #[error("GitHub API error: {0}")]
    GitHub(#[from] GitHubError),

    /// 設定関連エラー
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// 通知関連エラー
    #[error("Notification error: {0}")]
    Notification(#[from] NotificationError),

    /// ポーリング関連エラー
    #[error("Polling error: {0}")]
    Polling(#[from] PollingError),

    /// 汎用エラー
    #[error("{message}")]
    Generic { message: String },
}

/// 認証関連エラー
#[derive(Error, Debug)]
pub enum AuthError {
    /// HTTPリクエストエラー
    #[error("Request error: {source}")]
    RequestError {
        #[source]
        source: reqwest::Error,
    },

    /// JSONパースエラー
    #[error("JSON parsing error: {source}")]
    JsonError {
        #[source]
        source: serde_json::Error,
    },

    /// Keyring操作エラー
    #[error("Keyring error: {source}")]
    KeyringError {
        #[source]
        source: keyring::Error,
    },

    /// トークン取得エラー
    #[error("Failed to get token: {reason}")]
    TokenRetrievalError { reason: String },

    /// 認証初期化エラー
    #[error("Authentication initialization failed: {reason}")]
    InitializationError { reason: String },

    /// 汎用認証エラー
    #[error("{reason}")]
    Generic { reason: String },
}

/// GitHub API関連エラー
#[derive(Error, Debug)]
pub enum GitHubError {
    /// HTTPリクエストエラー
    #[error("HTTP request failed: {source}")]
    RequestError {
        #[source]
        source: reqwest::Error,
    },

    /// JSONパースエラー
    #[error("Response parsing failed: {source}")]
    ParseError {
        #[source]
        source: serde_json::Error,
    },

    /// APIレートリミット超過
    #[error("GitHub API rate limit exceeded. Please try again later.")]
    RateLimitExceeded,

    /// 認証エラー
    #[error("Authentication failed. Please check your GitHub token.")]
    AuthenticationError,

    /// リソースが見つからない
    #[error("Resource not found: {resource_type} {resource_id}")]
    NotFound {
        resource_type: String,
        resource_id: String,
    },

    /// サーバーエラー
    #[error("GitHub server error: {status} {message}")]
    ServerError { status: u16, message: String },

    /// ネットワークエラー
    #[error("Network error: {source}")]
    NetworkError {
        #[source]
        source: reqwest::Error,
    },

    /// APIレスポンスエラー
    #[error("API response error: {message}")]
    ApiError { message: String },

    /// 汎用GitHubエラー
    #[error("{message}")]
    Generic { message: String },
}

/// 設定関連エラー
#[derive(Error, Debug)]
pub enum ConfigError {
    /// 設定ファイル読み込みエラー
    #[error("Failed to load config file: {source}")]
    LoadError {
        #[source]
        source: std::io::Error,
    },

    /// 設定ファイルパースエラー
    #[error("Failed to parse config file: {source}")]
    ParseError {
        #[source]
        source: toml::de::Error,
    },

    /// 設定バリデーションエラー
    #[error("Configuration validation failed: {reason}")]
    ValidationError { reason: String },

    /// 設定ファイル書き込みエラー
    #[error("Failed to write config file: {source}")]
    WriteError {
        #[source]
        source: std::io::Error,
    },

    /// 汎用設定エラー
    #[error("{message}")]
    Generic { message: String },
}

/// 通知関連エラー
#[derive(Error, Debug)]
pub enum NotificationError {
    /// 通知送信エラー
    #[error("Failed to send notification: {source}")]
    SendError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// 通知作成エラー
    #[error("Failed to create notification: {reason}")]
    CreationError { reason: String },

    /// 通知フィルタリングエラー
    #[error("Notification filtering failed: {reason}")]
    FilterError { reason: String },

    /// 汎用通知エラー
    #[error("{message}")]
    Generic { message: String },
}

/// ポーリング関連エラー
#[derive(Error, Debug)]
pub enum PollingError {
    /// ポーリング間隔エラー
    #[error("Invalid polling interval: {interval}")]
    InvalidInterval { interval: u64 },

    /// ポーリング停止エラー
    #[error("Failed to stop polling: {source}")]
    StopError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// ポーリング再試行エラー
    #[error("Polling retry failed after {attempts} attempts")]
    RetryError { attempts: u32 },

    /// 汎用ポーリングエラー
    #[error("{message}")]
    Generic { message: String },
}

impl From<reqwest::Error> for GitHubError {
    fn from(error: reqwest::Error) -> Self {
        GitHubError::NetworkError { source: error }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::GitHub(GitHubError::NetworkError { source: error })
    }
}

impl From<serde_json::Error> for GitHubError {
    fn from(error: serde_json::Error) -> Self {
        GitHubError::ParseError { source: error }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::GitHub(GitHubError::ParseError { source: error })
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(error: reqwest::Error) -> Self {
        AuthError::RequestError { source: error }
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(error: serde_json::Error) -> Self {
        AuthError::JsonError { source: error }
    }
}

impl From<keyring::Error> for AuthError {
    fn from(error: keyring::Error) -> Self {
        AuthError::KeyringError { source: error }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        ConfigError::LoadError { source: error }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        ConfigError::ParseError { source: error }
    }
}
