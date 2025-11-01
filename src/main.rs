use gh_notifier::{AuthError, AuthManager};

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    println!("GitHub Notifier starting...");

    let mut auth_manager = AuthManager::new()?;

    // Try to load existing token from keychain
    if let Ok(Some(token_info)) = auth_manager.load_token_from_keychain() {
        println!("Found existing token in keychain");
        auth_manager.token_info = Some(token_info);
        
        // Use the automatic token management - this will refresh if needed
        match auth_manager.maybe_refresh_token().await {
            Ok(new_token) => {
                println!("Token is valid or was refreshed successfully");
                auth_manager.token_info = Some(new_token);
            }
            Err(e) => {
                println!("Token refresh failed: {:?}, proceeding with re-authentication", e);
                // If refresh failed (e.g., refresh token expired), initiate re-authentication
                match auth_manager.initiate_reauthentication().await {
                    Ok(new_token_info) => {
                        println!("Re-authentication successful!");
                        auth_manager.token_info = Some(new_token_info);
                    }
                    Err(e) => {
                        eprintln!("Re-authentication failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    } else {
        println!("No existing token found, starting OAuth Device Flow...");
        // Perform the OAuth device flow to get a new token
        match auth_manager.authenticate().await {
            Ok(token_info) => {
                println!("Authentication successful!");

                // Save the token to keychain for future use
                if let Err(e) = auth_manager.save_token_to_keychain(&token_info) {
                    eprintln!("Failed to save token to keychain: {:?}", e);
                } else {
                    println!("Token saved to keychain");
                    auth_manager.token_info = Some(token_info);
                }
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
