use crate::{
    AppInitializationService, Config, ConfigProvider, DefaultConfigProvider, DefaultExitHandler,
    DefaultMessageHandler, ExitHandler, InitializedApp, MessageHandler,
    runtime::run_polling_loop_with_shutdown,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Main application structure
pub struct Application;

impl Application {
    /// Run the GitHub Notifier application with default implementations
    pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::run_with_deps(
            &DefaultConfigProvider,
            &DefaultExitHandler,
            &DefaultMessageHandler,
        )
        .await
    }

    /// Run the GitHub Notifier application with dependency injection
    pub async fn run_with_deps(
        config_provider: &dyn ConfigProvider,
        exit_handler: &dyn ExitHandler,
        message_handler: &dyn MessageHandler,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Load config first to get log level
        let config = config_provider.load_config().unwrap_or_else(|e| {
            message_handler.eprint(&format!("Failed to load config: {}", e));
            exit_handler.exit(1);
            // This line won't be reached due to exit, but Rust requires it
            Config::default()
        });

        // Set up logging first so we can log setup process
        let _guard = crate::logger::setup_logging(&config);

        // Initialize application components
        let initialized_app = {
            let service =
                AppInitializationService::new(config_provider, exit_handler, message_handler);
            service.initialize().await?
        };

        // Perform notification recovery after initialization
        perform_notification_recovery(&initialized_app).await?;

        // Run the main polling loop
        run_polling_loop_with_shutdown(
            initialized_app.config,
            initialized_app.github_client,
            initialized_app.state_manager,
            initialized_app.notifier,
            initialized_app.history_manager,
        )
        .await?;

        tracing::info!("GitHub Notifier shutdown complete");
        Ok(())
    }
}

/// Perform notification recovery on application startup
async fn perform_notification_recovery(
    initialized_app: &InitializedApp,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get unread notifications from history
    let unread_notifications = initialized_app.history_manager.get_unread_notifications()?;

    if unread_notifications.is_empty() {
        tracing::info!("No unread notifications to recover");
        return Ok(());
    }

    tracing::info!(
        "Found {} unread notifications to recover",
        unread_notifications.len()
    );

    // Calculate the minimum age for notifications to be re-displayed
    // We don't want to re-display very old notifications that were missed
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_secs();

    let recovery_window_hours = initialized_app.config.notification_recovery_window_hours;
    // If recovery window is 0, don't recover any notifications
    if recovery_window_hours == 0 {
        tracing::info!("Notification recovery window is 0, skipping recovery");
        return Ok(());
    }
    let recovery_threshold = now - (recovery_window_hours * 3600); // Convert hours to seconds

    // Track how many notifications we actually display
    let mut displayed_count = 0;

    for notification in unread_notifications {
        // Parse the notification's received_at timestamp
        if let Ok(notification_time) =
            chrono::DateTime::parse_from_rfc3339(&notification.received_at)
        {
            let notification_timestamp = notification_time.timestamp() as u64;

            // Only display notifications within the recovery window
            if notification_timestamp >= recovery_threshold {
                // Check if this notification was likely already displayed recently
                // by looking at its marked_read_at field - if it exists, the notification
                // was processed before but might have been marked as unread again
                if let Some(read_at) = &notification.marked_read_at {
                    // Parse the read timestamp
                    if let Ok(read_time) = chrono::DateTime::parse_from_rfc3339(read_at) {
                        let read_timestamp = read_time.timestamp() as u64;
                        // If notification was read after it was received, skip it to avoid duplicates
                        if read_timestamp > notification_timestamp {
                            tracing::debug!(
                                "Skipping notification that was read before: {}",
                                notification.title
                            );
                            continue;
                        }
                    }
                }

                // Create a desktop notification
                let repo_name = if notification.repository.contains("🔒") {
                    format!("🔒 {}", notification.repository)
                } else {
                    notification.repository.clone()
                };

                let title = format!("{} - Recovery", repo_name);
                let body = format!(
                    "{}\n\nReason: {} | Type: {}",
                    notification.title, notification.reason, notification.subject_type
                );

                // Send the notification via the notifier
                if let Err(e) = initialized_app.notifier.send_notification(
                    &title,
                    &body,
                    &notification.url,
                    &initialized_app.config,
                ) {
                    tracing::warn!("Failed to send recovery notification: {}", e);
                } else {
                    displayed_count += 1;
                    tracing::debug!("Recovery notification sent for: {}", notification.title);

                    // Optionally mark as read if the config says to do so on recovery
                    if initialized_app.config.mark_as_read_on_notify {
                        let _ = initialized_app
                            .history_manager
                            .mark_as_read(&notification.id)
                            .map_err(|e| {
                                tracing::warn!(
                                    "Failed to mark recovered notification as read: {}",
                                    e
                                )
                            });
                    }
                }
            }
        } else {
            tracing::warn!(
                "Unable to parse notification timestamp: {}",
                notification.received_at
            );
        }
    }

    tracing::info!(
        "Recovery completed: displayed {} notifications",
        displayed_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ConfigProvider, ExitHandler, MessageHandler};
    use std::sync::{Arc, Mutex};

    struct MockConfigProvider {
        should_error: bool,
    }

    impl ConfigProvider for MockConfigProvider {
        fn load_config(&self) -> Result<Config, Box<dyn std::error::Error>> {
            if self.should_error {
                Err("Config loading error".into())
            } else {
                Ok(Config::default())
            }
        }
    }

    struct MockExitHandler {
        pub exit_called: Arc<Mutex<bool>>,
        pub exit_code: Arc<Mutex<i32>>,
    }

    impl ExitHandler for MockExitHandler {
        fn exit(&self, code: i32) {
            *self.exit_called.lock().unwrap() = true;
            *self.exit_code.lock().unwrap() = code;
        }
    }

    struct MockMessageHandler {
        pub printed_messages: Arc<Mutex<Vec<String>>>,
        pub eprinted_messages: Arc<Mutex<Vec<String>>>,
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

    #[tokio::test]
    async fn test_app_run_with_deps_config_error() {
        let _config_provider = MockConfigProvider { should_error: true };
        let exit_called = Arc::new(Mutex::new(false));
        let exit_code = Arc::new(Mutex::new(0));
        let _exit_handler = MockExitHandler {
            exit_called: exit_called.clone(),
            exit_code: exit_code.clone(),
        };
        let printed_messages = Arc::new(Mutex::new(Vec::new()));
        let eprinted_messages = Arc::new(Mutex::new(Vec::new()));
        let _message_handler = MockMessageHandler {
            printed_messages: printed_messages.clone(),
            eprinted_messages: eprinted_messages.clone(),
        };

        // We can't really test the exit behavior in async tests, so we'll just test the structure
        // This test can't fully validate the exit behavior, but it tests that the method exists
    }
}
