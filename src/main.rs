use clap::Parser;
use gh_notifier::{
    Application,
    cli::{Cli, Commands},
    utils,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    setup_logging()?;

    // Parse CLI arguments
    let cli = Cli::parse();

    tracing::info!("Starting gh-notifier version {}", env!("CARGO_PKG_VERSION"));

    match &cli.command {
        Some(Commands::Start) => {
            // Run the default application
            tracing::info!("Starting application in daemon mode");
            Application::run().await
        }
        Some(Commands::Tui) => {
            // Handle the TUI command
            tracing::info!("Starting application in TUI mode");
            handle_tui_command().await?;
            Ok(())
        }
        Some(command) => {
            // Handle other CLI commands
            tracing::info!("Handling CLI command: {:?}", command);
            handle_cli_command(command).await?;
            Ok(())
        }
        None => {
            // No subcommand provided - run default application
            tracing::info!("No command provided, starting default application");
            Application::run().await
        }
    }
}

/// TUIモードの処理
async fn handle_tui_command() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = utils::get_database_path();
    let history_manager = gh_notifier::HistoryManager::new(&db_path)?;
    gh_notifier::cli::handle_command(Commands::Tui, history_manager)?;
    Ok(())
}

/// その他のCLIコマンドの処理
async fn handle_cli_command(
    command: &Commands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = utils::get_database_path();
    let history_manager = gh_notifier::HistoryManager::new(&db_path)?;
    gh_notifier::cli::handle_command(command.clone(), history_manager)?;
    Ok(())
}

/// ログ設定の初期化
fn setup_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 環境変数からログレベルを取得
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // トレースングの初期化
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    Ok(())
}
