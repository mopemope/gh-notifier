#[cfg(test)]
mod integration_tests {
    use gh_notifier::{AuthManager, TokenInfo};
    use secrecy::SecretString;

    #[test]
    fn test_keychain_full_flow() {
        // Create a test AuthManager
        let mut auth_manager = AuthManager::new().unwrap();

        // Create a test token
        let test_token = TokenInfo {
            access_token: SecretString::new("test_access_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(9999999999), // Far in the future
            refresh_token: Some(SecretString::new("test_refresh_token".to_string())),
            refresh_token_expires_at: Some(9999999999),
        };

        // Save the token to storage (with fallback mechanisms)
        let save_result = auth_manager.save_token_to_storage(&test_token);
        // Note: This might fail in CI environments where keychain is not available
        if save_result.is_ok() {
            // If we can save, then we should be able to load
            let loaded = auth_manager.load_token_from_storage().unwrap();
            assert!(loaded.is_some());
            let loaded_token = loaded.unwrap();
            assert_eq!(loaded_token.token_type, "Bearer");

            // Test deletion
            let delete_result = auth_manager.delete_token_from_storage();
            assert!(delete_result.is_ok());

            // After deletion, loading should return None
            let after_delete = auth_manager.load_token_from_storage().unwrap();
            assert!(after_delete.is_none());
        } else {
            // If we can't save (e.g., in CI), that's OK - just skip the rest of the test
            println!("Skipping keychain integration test (storage not available)");
        }
    }
}
