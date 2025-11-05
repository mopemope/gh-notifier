use crate::{
    AuthError, Config, ConfigProvider, DesktopNotifier, ExitHandler, GitHubClient, InitializedApp,
    MessageHandler, StateManager, auth_manager::AuthManager,
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

        let mut auth_manager = AuthManager::new()?;

        // Load token from storage before attempting to get a valid token
        auth_manager.load_token_from_storage()?;

        // Try to load existing token from keychain and ensure it's valid
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                tracing::info!("Token is valid and ready for use");
                self.message_handler.print(
                    "Token is valid and ready for use. The application is now running in the background."
                );
                self.message_handler
                    .print("It will continuously check GitHub for new notifications.");
                self.message_handler
                    .print("Press Ctrl+C to stop the application.");
            }
            Err(e) => {
                tracing::error!("Failed to get valid token: {}", e);
                self.message_handler.eprint(
                    "Failed to validate existing token. This may be due to an invalid or unconfigured GitHub OAuth client ID."
                );
                self.message_handler.eprint(
                    "The existing token may be invalid, or the client ID may need to be updated.",
                );
                self.message_handler.eprint(
                    "Please check your configuration and ensure you have a valid client_id set.",
                );
                self.exit_handler.exit(1);
            }
        }

        // Load config
        let config = self.config_provider.load_config().unwrap_or_else(|e| {
            self.message_handler
                .eprint(&format!("Failed to load config: {}", e));
            self.exit_handler.exit(1);
            // This line won't be reached due to exit, but Rust requires it
            Config::default()
        });

        // Initialize clients and services
        let github_client = GitHubClient::new(auth_manager).unwrap();
        let state_manager = StateManager::new().unwrap();
        let notifier = Box::new(DesktopNotifier);

        tracing::info!("GitHub Notifier running with authenticated access");

        Ok(InitializedApp {
            config,
            github_client,
            state_manager,
            notifier,
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
