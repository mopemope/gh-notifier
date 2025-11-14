use crate::{
    AuthError, Config, ConfigProvider, DesktopNotifier, ExitHandler, GitHubClient, HistoryManager,
    InitializedApp, MessageHandler, StateManager, auth_manager::AuthManager,
};

/// Service that handles application initialization with dependency injection
pub struct AppInitializationService<'a> {
    config_provider: &'a dyn ConfigProvider,
    exit_handler: &'a dyn ExitHandler,
    message_handler: &'a dyn MessageHandler,
}

impl<'a> AppInitializationService<'a> {
    /// Create a new initialization service
    pub fn new(
        config_provider: &'a dyn ConfigProvider,
        exit_handler: &'a dyn ExitHandler,
        message_handler: &'a dyn MessageHandler,
    ) -> Self {
        Self {
            config_provider,
            exit_handler,
            message_handler,
        }
    }

    /// Initialize the application components
    pub async fn initialize(&self) -> Result<InitializedApp, AuthError> {
        tracing::info!("GitHub Notifier starting...");

        // Load config first to get PAT
        let config = self.config_provider.load_config().unwrap_or_else(|e| {
            self.message_handler
                .eprint(&format!("Failed to load config: {}", e));
            self.exit_handler.exit(1);
            // This line won't be reached due to exit, but Rust requires it
            Config::default()
        });

        let mut auth_manager = AuthManager::new()?;

        // Set the PAT from config if available
        if let Some(pat) = &config.github.token {
            if !pat.trim().is_empty() {
                use secrecy::SecretString;
                let token_info = crate::TokenInfo {
                    access_token: SecretString::new(pat.trim().to_string()),
                    token_type: "Bearer".to_string(),
                    expires_at: None,    // PATs don't expire by default
                    refresh_token: None, // No refresh token for PAT
                    refresh_token_expires_at: None,
                };
                auth_manager.token_info = Some(token_info);
            } else {
                tracing::error!("PAT is set in config but is empty");
                self.message_handler
                    .eprint("PAT is set in config but is empty.");
                self.exit_handler.exit(1);
            }
        } else {
            tracing::error!("No PAT found in config file");
            self.message_handler.eprint(
                "No PAT found in config file. Please add 'pat = \"your_token_here\"' to your config file."
            );
            self.message_handler.eprint(
                "Create a PAT at: https://github.com/settings/tokens with 'notifications' scope",
            );
            self.exit_handler.exit(1);
        }

        // Validate the PAT token
        match auth_manager.validate_token().await {
            Ok(true) => {
                tracing::info!("PAT is valid and ready for use");
                self.message_handler.print(
                    "PAT is valid and ready for use. The application is now running in the background."
                );
                self.message_handler
                    .print("It will continuously check GitHub for new notifications.");
                self.message_handler
                    .print("Press Ctrl+C to stop the application.");
            }
            Ok(false) => {
                tracing::error!(
                    "PAT validation failed - token is invalid or has insufficient permissions"
                );
                self.message_handler.eprint(
                    "PAT validation failed. This may be due to an invalid Personal Access Token.",
                );
                self.message_handler
                    .eprint("The PAT may be invalid or have insufficient permissions.");
                self.message_handler.eprint(
                    "Please make sure your Personal Access Token has the 'notifications' scope.",
                );
                self.exit_handler.exit(1);
            }
            Err(e) => {
                tracing::error!("PAT validation request failed: {}", e);
                self.message_handler
                    .eprint(&format!("PAT validation request failed: {}", e));
                self.exit_handler.exit(1);
            }
        }

        // Initialize clients and services
        let github_client = GitHubClient::new(config.github.clone()).unwrap();
        let state_manager = StateManager::new().unwrap();
        let notifier = Box::new(DesktopNotifier);

        // Initialize history manager
        let history_manager = {
            use dirs;

            // Create data directory path
            let mut data_dir = dirs::data_dir().unwrap_or_else(|| {
                std::env::current_dir().expect("Current directory not accessible")
            });
            data_dir.push("gh-notifier");

            // Create directory if it doesn't exist
            std::fs::create_dir_all(&data_dir).map_err(|e| {
                crate::errors::AuthError::InitializationError {
                    reason: format!("Failed to create data directory: {}", e),
                }
            })?;

            // Create database file path
            let db_path = data_dir.join("notifications.db");

            HistoryManager::new(&db_path).map_err(|e| {
                crate::errors::AuthError::InitializationError {
                    reason: format!("Failed to initialize HistoryManager: {}", e),
                }
            })?
        };

        tracing::info!("GitHub Notifier running with authenticated access");

        Ok(InitializedApp {
            config,
            github_client,
            state_manager,
            notifier,
            history_manager,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::traits::{ConfigProvider, ExitHandler, MessageHandler};
    use std::sync::{Arc, Mutex};

    // Test implementations
    struct MockConfigProvider {
        should_error: bool,
    }

    impl ConfigProvider for MockConfigProvider {
        fn load_config(&self) -> Result<crate::Config, Box<dyn std::error::Error>> {
            if self.should_error {
                Err("Config loading error".into())
            } else {
                Ok(crate::Config::default())
            }
        }
    }

    struct MockExitHandler {
        exit_called: Arc<Mutex<bool>>,
        exit_code: Arc<Mutex<i32>>,
    }

    impl ExitHandler for MockExitHandler {
        fn exit(&self, code: i32) {
            *self.exit_called.lock().unwrap() = true;
            *self.exit_code.lock().unwrap() = code;
        }
    }

    struct MockMessageHandler {
        printed_messages: Arc<Mutex<Vec<String>>>,
        eprinted_messages: Arc<Mutex<Vec<String>>>,
    }

    impl MessageHandler for MockMessageHandler {
        fn print(&self, message: &str) {
            self.printed_messages
                .lock()
                .unwrap()
                .push(message.to_string());
        }

        fn eprint(&self, message: &str) {
            self.eprinted_messages
                .lock()
                .unwrap()
                .push(message.to_string());
        }
    }

    #[test]
    fn test_mock_exit_handler() {
        let exit_called = Arc::new(Mutex::new(false));
        let exit_code = Arc::new(Mutex::new(0));
        let exit_handler = MockExitHandler {
            exit_called: exit_called.clone(),
            exit_code: exit_code.clone(),
        };

        exit_handler.exit(42);

        assert!(*exit_called.lock().unwrap());
        assert_eq!(*exit_code.lock().unwrap(), 42);
    }

    #[test]
    fn test_mock_message_handler() {
        let printed_messages = Arc::new(Mutex::new(Vec::new()));
        let eprinted_messages = Arc::new(Mutex::new(Vec::new()));
        let message_handler = MockMessageHandler {
            printed_messages: printed_messages.clone(),
            eprinted_messages: eprinted_messages.clone(),
        };

        message_handler.print("Test print message");
        message_handler.eprint("Test error message");

        assert_eq!(printed_messages.lock().unwrap().len(), 1);
        assert_eq!(printed_messages.lock().unwrap()[0], "Test print message");
        assert_eq!(eprinted_messages.lock().unwrap().len(), 1);
        assert_eq!(eprinted_messages.lock().unwrap()[0], "Test error message");
    }

    #[test]
    fn test_mock_config_provider() {
        let provider = MockConfigProvider {
            should_error: false,
        };

        let result = provider.load_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_config_provider_error() {
        let provider = MockConfigProvider { should_error: true };

        let result = provider.load_config();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Config loading error");
    }
}
