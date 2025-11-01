use crate::{AuthError, AuthManager, Notification};
use reqwest::Client;

pub struct GitHubClient {
    client: Client,
    auth_manager: AuthManager,
}

impl GitHubClient {
    pub fn new(auth_manager: AuthManager) -> Result<Self, AuthError> {
        let client = Client::builder()
            .build()
            .map_err(|e| AuthError::GeneralError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(GitHubClient {
            client,
            auth_manager,
        })
    }

    /// `/notifications` エンドポイントから通知を取得
    /// `if_modified_since` と `etag` はオプショナルで設定可能
    pub async fn get_notifications(
        &mut self,
        if_modified_since: Option<&str>,
        etag: Option<&str>,
    ) -> Result<Option<Vec<Notification>>, AuthError> {
        let token = self.auth_manager.get_valid_token().await?;
        let mut request_builder = self
            .client
            .get("https://api.github.com/notifications")
            .header("Authorization", format!("token {}", token));

        if let Some(ims) = if_modified_since {
            request_builder = request_builder.header("If-Modified-Since", ims);
        }

        if let Some(etag) = etag {
            request_builder = request_builder.header("If-None-Match", etag);
        }

        let response = request_builder.send().await?;

        // 304 Not Modified の場合は None を返す
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }

        // それ以外の場合は JSON をデシリアライズして返す
        let status = response.status();
        if status.is_success() {
            let notifications: Vec<Notification> = response.json().await?;
            Ok(Some(notifications))
        } else {
            let status_code = response.status();
            let text = response.text().await?;
            Err(AuthError::GeneralError(format!(
                "Failed to get notifications: {} - {}",
                status_code, text
            )))
        }
    }

    /// 通知を既読にする
    pub async fn mark_notification_as_read(
        &mut self,
        notification_id: &str,
    ) -> Result<(), AuthError> {
        let token = self.auth_manager.get_valid_token().await?;
        let url = format!(
            "https://api.github.com/notifications/threads/{}",
            notification_id
        );
        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("token {}", token))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let text = response.text().await?;
            Err(AuthError::GeneralError(format!(
                "Failed to mark notification as read: {} - {}",
                status, text
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_github_client_creation() {
        // AuthManagerが正しく初期化されればクライアントも作成可能
        let auth_manager = AuthManager::new().expect("AuthManager should be created");
        let client = GitHubClient::new(auth_manager);
        assert!(client.is_ok());
    }

    // 以下はマockサーバー等でのテストになるため、基本的な構造テストのみ
    #[test]
    fn test_notification_struct() {
        use serde_json;

        let json = r#"
            {
                "id": "123",
                "unread": true,
                "reason": "mention",
                "updated_at": "2023-01-01T00:00:00Z",
                "last_read_at": null,
                "subject": {
                    "title": "A new issue",
                    "url": "https://api.github.com/repos/user/repo/issues/1",
                    "latest_comment_url": "https://api.github.com/repos/user/repo/issues/comments/1",
                    "type": "Issue"
                },
                "repository": {
                    "id": 12345,
                    "node_id": "R_kgDOexample",
                    "name": "repo",
                    "full_name": "user/repo",
                    "private": false
                },
                "url": "https://api.github.com/notifications/threads/123",
                "subscription_url": "https://api.github.com/notifications/threads/123/subscription"
            }
        "#;

        let notification: Notification = serde_json::from_str(json).unwrap();
        assert_eq!(notification.id, "123");
        assert_eq!(notification.subject.title, "A new issue");
        assert_eq!(notification.repository.name, "repo");
    }
}
