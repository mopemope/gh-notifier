use gh_notifier::{AuthManager, AuthError};

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    println!("GitHub Notifier starting...");
    
    let mut auth_manager = AuthManager::new()?;
    
    // Try to load existing token from keychain
    if let Ok(Some(token_info)) = auth_manager.load_token_from_keychain() {
        println!("Found existing token in keychain");
        // Check if token is expired
        if let Some(expires_at) = token_info.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
                
            if now >= expires_at {
                println!("Token has expired, refreshing...");
                match auth_manager.refresh_token().await {
                    Ok(new_token) => {
                        println!("Token refreshed successfully");
                        if let Err(e) = auth_manager.save_token_to_keychain(&new_token) {
                            eprintln!("Failed to save refreshed token to keychain: {:?}", e);
                        }
                    }
                    Err(e) => {
                        println!("Token refresh failed: {:?}, proceeding with re-authentication", e);
                    }
                }
            } else {
                println!("Existing token is still valid");
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
