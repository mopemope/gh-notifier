pub mod api;
pub mod app;
pub mod auth;
pub mod auth_manager;
pub mod cli;
pub mod config;
pub mod errors;
pub mod github_client;
pub mod history_manager;
pub mod initialization_service;
pub mod initializer;
pub mod logger;
pub mod models;
pub mod poller;
pub mod polling;
pub mod runtime;
pub mod shutdown;
pub mod state;
pub mod storage;
pub mod token_storage;
pub mod traits;
pub mod tui;

pub use app::Application;
pub use auth::AuthManager;
pub use config::Config;
pub use errors::AuthError;
pub use github_client::GitHubClient;
pub use history_manager::HistoryManager;
pub use initialization_service::AppInitializationService;
pub use initializer::InitializedApp; // Keep this only if not redefined elsewhere
pub use logger::setup_logging;
pub use models::{Notification, NotificationRepository, NotificationSubject, TokenInfo};
pub use poller::{DesktopNotifier, Poller};
pub use polling::{filter_new_notifications, handle_notification, run_polling_loop};
pub use runtime::run_polling_loop_with_shutdown;
pub use shutdown::wait_for_shutdown_signal;
pub use state::{State, StateManager};
pub use traits::{
    ConfigProvider, DefaultConfigProvider, DefaultExitHandler, DefaultMessageHandler, ExitHandler,
    MessageHandler,
};
