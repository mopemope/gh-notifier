use gh_notifier::{AuthError, AuthManager};

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

    println!("GitHub Notifier running with authenticated access");
    Ok(())
}
