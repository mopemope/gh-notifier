use crate::poller::Notifier;

/// Trait for handling process exits, allowing for testable exit behavior
pub trait ExitHandler: Send + Sync {
    fn exit(&self, code: i32);
}

/// Default implementation that calls std::process::exit
pub struct DefaultExitHandler;

impl ExitHandler for DefaultExitHandler {
    fn exit(&self, code: i32) {
        std::process::exit(code);
    }
}

/// Trait for handling printing messages to console, allowing for testable output
pub trait MessageHandler: Send + Sync {
    fn print(&self, message: &str);
    fn eprint(&self, message: &str);
}

/// Default implementation that calls println!/eprintln!
pub struct DefaultMessageHandler;

impl MessageHandler for DefaultMessageHandler {
    fn print(&self, message: &str) {
        println!("{}", message);
    }

    fn eprint(&self, message: &str) {
        eprintln!("{}", message);
    }
}

/// Trait for configuration loading, allowing for testable configuration
pub trait ConfigProvider: Send + Sync {
    fn load_config(&self) -> Result<crate::Config, Box<dyn std::error::Error>> {
        crate::config::load_config()
    }
}

/// Default implementation that loads from file
pub struct DefaultConfigProvider;

impl ConfigProvider for DefaultConfigProvider {
    fn load_config(&self) -> Result<crate::Config, Box<dyn std::error::Error>> {
        crate::config::load_config()
    }
}

/// Trait for the application initializer to allow for testing
pub trait ApplicationInitializer: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn initialize(&self) -> Result<crate::InitializedApp, Self::Error>;
}

/// Trait for the runtime controller
pub trait RuntimeController: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    fn run_with_shutdown(
        &self,
        config: crate::Config,
        github_client: crate::GitHubClient,
        state_manager: crate::StateManager,
        notifier: Box<dyn Notifier>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_default_exit_handler() {
        // We can't actually test the exit behavior since it terminates the process,
        // but we can test that the trait is properly implemented
        let _handler: Box<dyn ExitHandler> = Box::new(DefaultExitHandler);
        // We can't call handler.exit() in tests as it would terminate the test
        // Just verify the trait can be constructed
    }

    #[test]
    fn test_default_message_handler() {
        let handler: Box<dyn MessageHandler> = Box::new(DefaultMessageHandler);
        // Test that the trait methods are accessible
        handler.print("Test message");
        handler.eprint("Test error message");
        // Basic functionality test
    }

    #[test]
    fn test_default_config_provider() {
        let provider: Box<dyn ConfigProvider> = Box::new(DefaultConfigProvider);
        // This might fail if no config file exists, so we just check that the trait is implemented
        let _ = provider.load_config(); // May return Ok or Err depending on config availability
        // Just check the trait is implemented
    }

    #[test]
    fn test_mock_exit_handler() {
        struct MockExitHandler {
            called: Arc<Mutex<bool>>,
            code: Arc<Mutex<i32>>,
        }

        impl ExitHandler for MockExitHandler {
            fn exit(&self, code: i32) {
                *self.called.lock().unwrap() = true;
                *self.code.lock().unwrap() = code;
            }
        }

        let called = Arc::new(Mutex::new(false));
        let code = Arc::new(Mutex::new(0));
        let handler = MockExitHandler {
            called: called.clone(),
            code: code.clone(),
        };

        handler.exit(42);

        assert!(*called.lock().unwrap());
        assert_eq!(*code.lock().unwrap(), 42);
    }

    #[test]
    fn test_mock_message_handler() {
        struct MockMessageHandler {
            printed: Arc<Mutex<Vec<String>>>,
            eprinted: Arc<Mutex<Vec<String>>>,
        }

        impl MessageHandler for MockMessageHandler {
            fn print(&self, message: &str) {
                self.printed.lock().unwrap().push(message.to_string());
            }

            fn eprint(&self, message: &str) {
                self.eprinted.lock().unwrap().push(message.to_string());
            }
        }

        let printed = Arc::new(Mutex::new(Vec::new()));
        let eprinted = Arc::new(Mutex::new(Vec::new()));
        let handler = MockMessageHandler {
            printed: printed.clone(),
            eprinted: eprinted.clone(),
        };

        handler.print("Hello");
        handler.eprint("World");

        assert_eq!(printed.lock().unwrap().len(), 1);
        assert_eq!(printed.lock().unwrap()[0], "Hello");
        assert_eq!(eprinted.lock().unwrap().len(), 1);
        assert_eq!(eprinted.lock().unwrap()[0], "World");
    }
}
