pub mod auth_manager;
pub mod config;
pub mod errors;
pub mod github_client;
pub mod models;
pub mod poller;
pub mod polling;
pub mod state;

pub use auth_manager::AuthManager;
pub use config::Config;
pub use errors::AuthError;
pub use github_client::GitHubClient;
pub use models::{
    DeviceAuthResponse, ErrorResponse, Notification, NotificationRepository, NotificationSubject,
    TokenInfo, TokenResponse,
};
pub use poller::{DesktopNotifier, Poller};
pub use polling::{filter_new_notifications, handle_notification, run_polling_loop};
pub use state::{State, StateManager};
