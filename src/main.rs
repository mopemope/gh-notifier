use clap::Parser;
use gh_notifier::{
    Application,
    cli::{Cli, Commands},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Start) => {
            // Run the default application as before
            Application::run().await
        }
        Some(command) => {
            // Handle CLI commands
            let db_path = get_database_path();

            let history_manager = gh_notifier::HistoryManager::new(&db_path)?;
            gh_notifier::cli::handle_command(command.clone(), history_manager)?;
            Ok(()) // Return Ok explicitly after CLI command handling
        }
        None => {
            // No subcommand provided - run default application
            Application::run().await
        }
    }
}

fn get_database_path() -> std::path::PathBuf {
    // Try to get config directory, fallback to current directory
    let mut db_path = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().expect("Current directory not accessible"));
    db_path.push("gh-notifier");
    db_path.push("gh-notifier.db");
    db_path
}
