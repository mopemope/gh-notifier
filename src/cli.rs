use crate::HistoryManager;
use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// GitHub Notifier - A tool for receiving GitHub notifications
#[derive(Parser)]
#[command(name = "gh-notifier")]
#[command(about = "A GitHub notification client with history and management features", long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Logging level
    #[arg(short, long, value_name = "LEVEL", default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Start the notification polling service
    Start,

    /// View notification history
    History(HistoryArgs),

    /// Mark notifications as read
    MarkRead(MarkReadArgs),

    /// Delete notifications
    Delete(DeleteArgs),

    /// Filter and manage notifications
    Filter(FilterArgs),

    /// Show application info
    Info,

    /// Launch the TUI to view and manage notifications
    Tui,
}

#[derive(Args, Clone)]
pub struct HistoryArgs {
    /// Show only unread notifications
    #[arg(short, long)]
    pub unread: bool,

    /// Show only read notifications  
    #[arg(long)]
    pub read: bool,

    /// Filter by repository (e.g., "user/repo")
    #[arg(short, long)]
    pub repository: Option<String>,

    /// Filter by notification reason (e.g., "mention", "review_requested")
    #[arg(long)]
    pub reason: Option<String>,

    /// Filter by subject type (e.g., "Issue", "PullRequest")
    #[arg(short, long)]
    pub subject_type: Option<String>,

    /// Filter notifications by date range (e.g., "2023-01-01")
    #[arg(long)]
    pub since: Option<String>,

    /// Filter notifications by date range (e.g., "2023-12-31")
    #[arg(long)]
    pub until: Option<String>,

    /// Limit the number of notifications to show
    #[arg(short, long, default_value = "50")]
    pub limit: usize,

    /// Show detailed information for each notification
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args, Clone)]
pub struct MarkReadArgs {
    /// Mark all notifications as read
    #[arg(short, long)]
    pub all: bool,

    /// Notification ID to mark as read
    #[arg(value_name = "NOTIFICATION_ID")]
    pub notification_ids: Vec<String>,

    /// Mark all notifications for a specific repository as read
    #[arg(long)]
    pub repository: Option<String>,
}

#[derive(Args, Clone)]
pub struct DeleteArgs {
    /// Notification IDs to delete
    #[arg(value_name = "NOTIFICATION_ID")]
    pub notification_ids: Vec<String>,

    /// Delete all notifications
    #[arg(short, long)]
    pub all: bool,

    /// Delete notifications for a specific repository
    #[arg(long)]
    pub repository: Option<String>,
}

#[derive(Args, Clone)]
pub struct FilterArgs {
    /// Clear all notifications from history
    #[arg(long)]
    pub clear: bool,

    /// Filter notifications by date range
    #[arg(long)]
    pub since: Option<String>,

    /// Filter notifications by date range
    #[arg(long)]
    pub until: Option<String>,
}

pub fn handle_command(
    command: Commands,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        Commands::Start => {
            eprintln!("Use the default application to start the polling service.");
            std::process::exit(1);
        }
        Commands::History(args) => handle_history_command(args, history_manager)?,
        Commands::MarkRead(args) => handle_mark_read_command(args, history_manager)?,
        Commands::Delete(args) => handle_delete_command(args, history_manager)?,
        Commands::Filter(args) => handle_filter_command(args, history_manager)?,
        Commands::Info => handle_info_command(history_manager)?,
        Commands::Tui => handle_tui_command(history_manager)?,
    }

    Ok(())
}

fn handle_history_command(
    args: HistoryArgs,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let notifications = if args.unread {
        history_manager.get_unread_notifications()?
    } else if args.read {
        // Get all notifications and filter for read ones
        history_manager
            .get_all_notifications()?
            .into_iter()
            .filter(|n| n.is_read)
            .collect()
    } else {
        history_manager.get_all_notifications()?
    };

    // Apply additional filters
    let mut filtered_notifications = notifications;

    if let Some(repo) = args.repository {
        filtered_notifications.retain(|n| n.repository.contains(&repo));
    }

    if let Some(reason) = args.reason {
        filtered_notifications.retain(|n| n.reason == reason);
    }

    if let Some(subject_type) = args.subject_type {
        filtered_notifications.retain(|n| n.subject_type == subject_type);
    }

    // Filter by date range if specified
    if args.since.is_some() || args.until.is_some() {
        filtered_notifications.retain(|n| {
            let notification_time = DateTime::parse_from_rfc3339(&n.received_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            let since_ok = if let Some(since_str) = &args.since {
                if let Ok(since_dt) = DateTime::parse_from_rfc3339(since_str) {
                    notification_time >= since_dt.timestamp()
                } else {
                    // Try parsing as date only (YYYY-MM-DD)
                    if let Ok(since_date) = NaiveDate::parse_from_str(since_str, "%Y-%m-%d") {
                        let since_datetime = since_date.and_hms_opt(0, 0, 0).unwrap();
                        let since_utc =
                            DateTime::<Utc>::from_naive_utc_and_offset(since_datetime, Utc);
                        notification_time >= since_utc.timestamp()
                    } else {
                        true // If parsing fails, don't filter
                    }
                }
            } else {
                true
            };

            let until_ok = if let Some(until_str) = &args.until {
                if let Ok(until_dt) = DateTime::parse_from_rfc3339(until_str) {
                    notification_time <= until_dt.timestamp()
                } else {
                    // Try parsing as date only (YYYY-MM-DD)
                    if let Ok(until_date) = NaiveDate::parse_from_str(until_str, "%Y-%m-%d") {
                        let until_datetime = until_date.and_hms_opt(23, 59, 59).unwrap();
                        let until_utc =
                            DateTime::<Utc>::from_naive_utc_and_offset(until_datetime, Utc);
                        notification_time <= until_utc.timestamp()
                    } else {
                        true // If parsing fails, don't filter
                    }
                }
            } else {
                true
            };

            since_ok && until_ok
        });
    }

    // Apply limit
    filtered_notifications.truncate(args.limit);

    // Print notifications
    if filtered_notifications.is_empty() {
        println!("No notifications found.");
    } else {
        for notification in filtered_notifications {
            if args.verbose {
                println!("ID: {}", notification.id);
                println!("Title: {}", notification.title);
                println!("Repository: {}", notification.repository);
                println!("Reason: {}", notification.reason);
                println!("Type: {}", notification.subject_type);
                println!("URL: {}", notification.url);
                println!("Received: {}", notification.received_at);
                println!("Read: {}", if notification.is_read { "Yes" } else { "No" });
                if let Some(read_at) = &notification.marked_read_at {
                    println!("Read at: {}", read_at);
                }
                println!("---");
            } else {
                let status = if notification.is_read {
                    "READ"
                } else {
                    "UNREAD"
                };
                println!(
                    "[{}] {} - {} ({})",
                    status, notification.repository, notification.title, notification.reason
                );
            }
        }
    }

    Ok(())
}

fn handle_mark_read_command(
    args: MarkReadArgs,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.all {
        history_manager.mark_all_as_read()?;
        println!("All notifications marked as read.");
    } else if let Some(repo) = args.repository {
        let all_notifications = history_manager.get_all_notifications()?;
        let notifications_to_mark: Vec<String> = all_notifications
            .into_iter()
            .filter(|n| !n.is_read && n.repository.contains(&repo))
            .map(|n| n.id)
            .collect();

        for notification_id in &notifications_to_mark {
            history_manager.mark_as_read(notification_id)?;
        }

        println!(
            "Marked {} notifications from '{}' as read.",
            notifications_to_mark.len(),
            repo
        );
    } else if !args.notification_ids.is_empty() {
        for notification_id in &args.notification_ids {
            history_manager.mark_as_read(notification_id)?;
        }

        println!(
            "Marked {} notifications as read.",
            args.notification_ids.len()
        );
    } else {
        eprintln!("Please specify notification IDs, use --all, or use --repository.");
        std::process::exit(1);
    }

    Ok(())
}

fn handle_filter_command(
    args: FilterArgs,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.clear {
        let all_notifications = history_manager.get_all_notifications()?;
        let count = all_notifications.len();

        // Remove all notifications
        for notification in all_notifications {
            history_manager.delete_notification(&notification.id)?;
        }

        println!("Cleared {} notifications from history.", count);
    } else if args.since.is_some() || args.until.is_some() {
        // For now, just display a message about date filtering
        // In a full implementation, we would filter by date
        println!(
            "Date range filtering (since: {:?}, until: {:?}) is not fully implemented yet.",
            args.since, args.until
        );
    } else {
        eprintln!("Filter command supports --clear to remove all notifications.");
        eprintln!("Other filters like --since and --until are coming soon.");
    }

    Ok(())
}

fn handle_delete_command(
    args: DeleteArgs,
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.all {
        let all_notifications = history_manager.get_all_notifications()?;
        let count = all_notifications.len();

        for notification in all_notifications {
            history_manager.delete_notification(&notification.id)?;
        }

        println!("Deleted all {} notifications.", count);
    } else if let Some(repo) = args.repository {
        let all_notifications = history_manager.get_all_notifications()?;
        let notifications_to_delete: Vec<String> = all_notifications
            .into_iter()
            .filter(|n| n.repository.contains(&repo))
            .map(|n| n.id.clone())
            .collect();

        for notification_id in &notifications_to_delete {
            history_manager.delete_notification(notification_id)?;
        }

        println!(
            "Deleted {} notifications from '{}'.",
            notifications_to_delete.len(),
            repo
        );
    } else if !args.notification_ids.is_empty() {
        for notification_id in &args.notification_ids {
            history_manager.delete_notification(notification_id)?;
        }

        println!("Deleted {} notifications.", args.notification_ids.len());
    } else {
        eprintln!("Please specify notification IDs, use --all, or use --repository.");
        std::process::exit(1);
    }

    Ok(())
}

fn handle_info_command(
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let all_notifications = history_manager.get_all_notifications()?;
    let unread_notifications: Vec<_> = all_notifications.iter().filter(|n| !n.is_read).collect();
    let read_notifications: Vec<_> = all_notifications.iter().filter(|n| n.is_read).collect();

    println!("GitHub Notifier - Application Information");
    println!("========================================");
    println!("Total notifications: {}", all_notifications.len());
    println!("Unread notifications: {}", unread_notifications.len());
    println!("Read notifications: {}", read_notifications.len());

    if !all_notifications.is_empty() {
        // Find the oldest and newest notifications
        let mut sorted_notifications = all_notifications;
        sorted_notifications.sort_by(|a, b| a.received_at.cmp(&b.received_at));

        if let Some(oldest) = sorted_notifications.first() {
            println!(
                "Oldest notification: {} ({})",
                oldest.title, oldest.received_at
            );
        }

        if let Some(newest) = sorted_notifications.last() {
            println!(
                "Newest notification: {} ({})",
                newest.title, newest.received_at
            );
        }
    }

    Ok(())
}

fn handle_tui_command(
    history_manager: HistoryManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::TuiApp;

    println!("Starting GitHub Notifier TUI...");
    println!("Press 'q' to quit the interface at any time.");

    let mut app = TuiApp::new(history_manager)?;
    app.run()?;

    Ok(())
}
