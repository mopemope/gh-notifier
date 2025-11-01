use gh_notifier::config::load_config;
use gh_notifier::{AuthError, GitHubClient, Poller, StateManager, auth_manager::AuthManager};

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    println!("GitHub Notifier starting...");

    let mut auth_manager = AuthManager::new()?;

    // Try to load existing token from keychain and ensure it's valid
    if let Ok(Some(token_info)) = auth_manager.load_token_from_keychain() {
        println!("Found existing token in keychain");
        auth_manager.token_info = Some(token_info);

        // Use the comprehensive token management - handles validation, refresh, and re-auth
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                println!("Token is valid and ready for use");
            }
            Err(e) => {
                eprintln!("Failed to get valid token: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("No existing token found, starting OAuth Device Flow...");
        // Perform the OAuth device flow to get a new token
        match auth_manager.get_valid_token_with_reauth().await {
            Ok(_) => {
                println!("Authentication successful!");
            }
            Err(e) => {
                eprintln!("Authentication failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 設定を読み込む
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    // GitHubクライアント、ステートマネージャー、通知マネージャーを初期化
    let github_client = GitHubClient::new(auth_manager).unwrap();
    let state_manager = StateManager::new().unwrap();
    let notifier = Box::new(gh_notifier::DesktopNotifier);

    // Pollerを初期化して実行
    let mut poller = Poller::new(config, github_client, state_manager, notifier);

    println!("GitHub Notifier running with authenticated access");
    poller.run().await.unwrap();

    Ok(())
}
