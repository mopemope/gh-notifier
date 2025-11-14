use crate::config::GitHubConfig;
use crate::errors::{AppError, GitHubError};
use crate::github::types::*;
use reqwest::{Client, Response, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

/// GitHub APIクライアint
pub struct GitHubClient {
    client: Client,
    config: GitHubConfig,
}

impl GitHubClient {
    /// 新しいGitHubクライアintを作成
    pub fn new(config: GitHubConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .user_agent(format!("gh-notifier/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| GitHubError::NetworkError { source: e })?;

        Ok(GitHubClient { client, config })
    }

    /// GitHub Personal Access Tokenを設定
    pub fn set_token(&mut self, token: SecretString) {
        self.config.token = Some(token.expose_secret().to_string());
    }

    /// 有効な認証トークンを取得
    pub fn get_token(&self) -> Result<&str, AppError> {
        self.config
            .token
            .as_deref()
            .ok_or_else(|| GitHubError::AuthenticationError.into())
    }

    /// APIベースURLを取得
    pub fn get_api_base_url(&self) -> &str {
        &self.config.api_base_url
    }

    /// 通知を取得
    pub async fn get_notifications(
        &self,
        if_modified_since: Option<&str>,
        etag: Option<&str>,
    ) -> Result<Option<Vec<Notification>>, AppError> {
        let token = self.get_token()?;
        let url = format!("{}/notifications", self.get_api_base_url());

        let mut request_builder = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(ims) = if_modified_since {
            request_builder = request_builder.header("If-Modified-Since", ims);
        }

        if let Some(etag) = etag {
            request_builder = request_builder.header("If-None-Match", etag);
        }

        let response = self.send_with_retry(request_builder).await?;

        // 304 Not Modified の場合は None を返す
        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(None);
        }

        self.handle_notification_response(response).await
    }

    /// 通知を既読にする
    pub async fn mark_notification_as_read(&self, notification_id: &str) -> Result<(), AppError> {
        let token = self.get_token()?;
        let url = format!(
            "{}/notifications/threads/{}",
            self.get_api_base_url(),
            notification_id
        );

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => Ok(()),
            StatusCode::NOT_FOUND => Err(GitHubError::NotFound {
                resource_type: "notification".to_string(),
                resource_id: notification_id.to_string(),
            }
            .into()),
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// プルリクエストを取得
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<PullRequest, AppError> {
        let token = self.get_token()?;
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.get_api_base_url(),
            owner,
            repo,
            pr_number
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => {
                let pr: PullRequest = response.json().await?;
                Ok(pr)
            }
            StatusCode::NOT_FOUND => Err(GitHubError::NotFound {
                resource_type: "pull_request".to_string(),
                resource_id: format!("{}/{}#{}", owner, repo, pr_number),
            }
            .into()),
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// Issueを取得
    pub async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<Issue, AppError> {
        let token = self.get_token()?;
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.get_api_base_url(),
            owner,
            repo,
            issue_number
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => {
                let issue: Issue = response.json().await?;
                Ok(issue)
            }
            StatusCode::NOT_FOUND => Err(GitHubError::NotFound {
                resource_type: "issue".to_string(),
                resource_id: format!("{}/{}#{}", owner, repo, issue_number),
            }
            .into()),
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// リポジトリを取得
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository, AppError> {
        let token = self.get_token()?;
        let url = format!("{}/repos/{}/{}", self.get_api_base_url(), owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => {
                let repo: Repository = response.json().await?;
                Ok(repo)
            }
            StatusCode::NOT_FOUND => Err(GitHubError::NotFound {
                resource_type: "repository".to_string(),
                resource_id: format!("{}/{}", owner, repo),
            }
            .into()),
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// レートリミット情報を取得
    pub async fn get_rate_limit(&self) -> Result<RateLimit, AppError> {
        let token = self.get_token()?;
        let url = format!("{}/rate_limit", self.get_api_base_url());

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => {
                let rate_limit: RateLimit = response.json().await?;
                Ok(rate_limit)
            }
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// 通知のレスポンスを処理
    async fn handle_notification_response(
        &self,
        response: Response,
    ) -> Result<Option<Vec<Notification>>, AppError> {
        match response.status() {
            status if status.is_success() => {
                let notifications: Vec<Notification> = response.json().await?;
                Ok(Some(notifications))
            }
            StatusCode::FORBIDDEN => {
                let text = response.text().await?;
                if text.contains("Bad credentials") || text.contains("Invalid token") {
                    Err(GitHubError::AuthenticationError.into())
                } else {
                    Err(GitHubError::ApiError { message: text }.into())
                }
            }
            _ => Err(self.handle_error_response(response).await),
        }
    }

    /// エラーレスポンスを処理
    async fn handle_error_response(&self, response: Response) -> AppError {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        GitHubError::ServerError {
            status: status.as_u16(),
            message: text,
        }
        .into()
    }

    /// リトライ付きでリクエスト送信
    async fn send_with_retry(
        &self,
        request_builder: reqwest::RequestBuilder,
    ) -> Result<Response, AppError> {
        let mut last_error = None;

        for attempt in 0..=self.config.retry_count {
            match request_builder.try_clone().unwrap().send().await {
                Ok(response) => {
                    // レートリミットチェック
                    if response.status() == StatusCode::FORBIDDEN {
                        let status_code = response.status().as_u16();
                        let text = response.text().await?;
                        if text.contains("API rate limit exceeded") {
                            if attempt < self.config.retry_count {
                                // レートリミット exceeded、リトライ
                                tokio::time::sleep(Duration::from_secs(
                                    self.config.retry_interval_sec,
                                ))
                                .await;
                                last_error = Some(GitHubError::RateLimitExceeded);
                                continue;
                            } else {
                                // All retries exhausted after rate limit error
                                return Err(GitHubError::RateLimitExceeded.into());
                            }
                        } else {
                            // 403 but not rate limit - return error response as text
                            return Err(GitHubError::ServerError {
                                status: status_code,
                                message: text,
                            }
                            .into());
                        }
                    } else {
                        // Status is not 403 - return the response
                        return Ok(response);
                    }
                }
                Err(e) => {
                    if attempt < self.config.retry_count {
                        tokio::time::sleep(Duration::from_secs(self.config.retry_interval_sec))
                            .await;
                        last_error = Some(GitHubError::NetworkError { source: e });
                        continue;
                    }
                }
            }
        }

        // If we reach here, all retries were exhausted and last_error contains the last error
        if let Some(error) = last_error {
            Err(error.into())
        } else {
            // This shouldn't happen, but provide a default error
            // If all retries are exhausted and no error was stored, return a generic error
            Err(GitHubError::Generic {
                message: "All retry attempts failed with no specific error".to_string(),
            }
            .into())
        }
    }
}
