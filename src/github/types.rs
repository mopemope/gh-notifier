use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHubユーザー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// ユーザーID
    pub id: u64,
    /// ユーザー名
    pub login: String,
    /// ユーザーアバターURL
    pub avatar_url: Option<String>,
    /// ユーザープロファイルURL
    pub html_url: String,
    /// 組織ユーザーかどうか
    pub r#type: String,
    /// サイト管理者かどうか
    pub site_admin: Option<bool>,
}

/// GitHubリポジトリ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// リポジトリID
    pub id: u64,
    /// リポジトリ名（owner/repo形式）
    pub full_name: String,
    /// リポジトリ所有者
    pub owner: User,
    /// リポジトリ説明
    pub description: Option<String>,
    /// リポジトリURL
    pub html_url: String,
    /// 組織リポジトリかどうか
    pub r#private: bool,
    /// フォークリポジトリかどうか
    pub fork: bool,
    /// 親リポジトリ（フォーク元）
    pub parent: Option<Box<Repository>>,
    /// 元のテンプレートリポジトリ
    pub template_repository: Option<Box<Repository>>,
    /// 最新のブランチ名
    pub default_branch: String,
    /// ブランチ数
    pub master_branch: Option<String>,
    /// 読み取り専用かどうか
    pub permissions: Option<RepositoryPermissions>,
    /// テンプレートとして使用可能かどうか
    pub is_template: Option<bool>,
    /// ネットワーク（フォーク）数
    pub network_count: Option<u64>,
    /// スター数
    pub subscribers_count: Option<u64>,
}

/// リポジトリ権限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPermissions {
    /// 読み取り権限
    pub pull: bool,
    /// 書き込み権限
    pub push: bool,
    /// 管理権限
    pub admin: bool,
    /// ブランチ保護の管理権限
    pub maintain: Option<bool>,
    /// プロジェクトの管理権限
    pub triage: Option<bool>,
}

/// プルリクエスト情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// プルリクエストID
    pub id: u64,
    /// プルリクエスト番号
    pub number: u32,
    /// プルリクエストタイトル
    pub title: String,
    /// プルリクエスト作成者
    pub user: User,
    /// プルリクエスト状態（open, closed, draft）
    pub state: String,
    /// プルリクエストがマージ済みかどうか
    pub merged: Option<bool>,
    /// マージ可能かどうか
    pub mergeable: Option<bool>,
    /// マージ可能状態
    pub mergeable_state: Option<String>,
    /// マージ済みかどうか
    pub merged_at: Option<DateTime<Utc>>,
    /// マージユーザー
    pub merged_by: Option<User>,
    /// マージコミットSHA
    pub merge_commit_sha: Option<String>,
    /// プルリクエストURL
    pub html_url: String,
    /// プルリクエストAPI URL
    pub url: String,
    /// プルリクエストAPIコメントURL
    pub comments_url: String,
    /// プルリクエストAPIレビューURL
    pub review_comments_url: String,
    /// プルリクエストAPIレビューURL
    pub review_comment_url: String,
    /// プルリクエストAPIステータスURL
    pub statuses_url: String,
    /// プルリクエストAPIコメントURL
    pub issue_url: String,
    /// プルリクエストAPIコメントURL
    pub commits_url: String,
    /// プルリクエストAPIコメントURL
    pub review_events_url: String,
    /// プルリクエストAPIコメントURL
    pub events_url: String,
    /// プルリクエストAPIコメントURL
    pub assignees_url: String,
    /// プルリクエストAPIコメントURL
    pub branches_url: String,
    /// プルリクエストAPIコメントURL
    pub tags_url: String,
    /// プルリクエストAPIコメントURL
    pub trees_url: String,
    /// プルリクエストAPIコメントURL
    pub svn_url: String,
    /// プルリクエストAPIコメントURL
    pub forks_url: String,
    /// プルリクエストAPIコメントURL
    pub collaborators_url: String,
    /// プルリクエストAPIコメントURL
    pub subscribers_url: String,
    /// プルリクエストAPIコメントURL
    pub subscription_url: String,
    /// プルリクエストコミット数
    pub commits_count: Option<u32>,
    /// プルリクエスト追加行数
    pub additions: Option<u32>,
    /// プルリクエスト削除行数
    pub deletions: Option<u32>,
    /// プルリクエスト変更ファイル数
    pub changed_files: Option<u32>,
    /// プルリクエスト作成日時
    pub created_at: DateTime<Utc>,
    /// プルリクエスト更新日時
    pub updated_at: DateTime<Utc>,
    /// プルリクエストクローズ日時
    pub closed_at: Option<DateTime<Utc>>,
    /// プルリクエストマージ日時（重複削除）
    /// プルリクエストのbaseブランチ情報
    pub base: BranchInfo,
    /// プルリクエストのheadブランチ情報
    pub head: BranchInfo,
    /// ラベルリスト
    pub labels: Option<Vec<Label>>,
    /// アイキャッチ画像
    pub assignees: Option<Vec<User>>,
    /// プルリクエストがドラフトかどうか
    pub draft: Option<bool>,
    /// プルリクエストがアクティブなレビューかどうか
    pub active_lock_reason: Option<String>,
    /// プルリクエストのbody
    pub body: Option<String>,
    /// プルリクエストがロックされているかどうか
    pub locked: Option<bool>,
    /// プルリクエストのauthor_association
    pub author_association: Option<String>,
    /// プルリクエストのmilestone
    pub milestone: Option<Milestone>,
    /// プルリクエストのreactions
    pub reactions: Option<Reactions>,
}

/// ブランチ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    /// ブランチ名
    pub label: String,
    /// ブランチリポジトリ
    pub ref_name: String,
    /// ブランチSHA
    pub sha: String,
    /// ブランチユーザー
    pub user: User,
    /// ブランチリポジトリ
    pub repo: Repository,
}

/// ラベル情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// ラベルID
    pub id: u64,
    /// ラベル名
    pub name: String,
    /// ラベル色
    pub color: String,
    /// ラベル説明
    pub description: Option<String>,
}

/// マイルストーン情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// マイルストーンID
    pub id: u64,
    /// マイルストーン番号
    pub number: u32,
    /// マイルストーンタイトル
    pub title: String,
    /// マイルストーン説明
    pub description: Option<String>,
    /// マイルストーン状態
    pub state: String,
    /// マイルストーン作成者
    pub creator: User,
    /// マイルストーン作成日時
    pub created_at: DateTime<Utc>,
    /// マイルストーン更新日時
    pub updated_at: DateTime<Utc>,
    /// マイルストーンクローズ日時
    pub closed_at: Option<DateTime<Utc>>,
    /// マイルストーン期日
    pub due_on: Option<DateTime<Utc>>,
    /// マイルストーンのopen issues数
    pub open_issues: u32,
    /// マイルストーンのclosed issues数
    pub closed_issues: u32,
    /// マイルストーンURL
    pub html_url: String,
    /// マイルストーンのlabel
    pub labels: Option<Vec<Label>>,
}

/// リアクション情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reactions {
    /// total count
    pub total_count: Option<u32>,
    /// plus_one count
    #[serde(rename = "+1")]
    pub plus_one: Option<u32>,
    /// minus_one count
    #[serde(rename = "-1")]
    pub minus_one: Option<u32>,
    /// laugh count
    pub laugh: Option<u32>,
    /// hooray count
    pub hooray: Option<u32>,
    /// confused count
    pub confused: Option<u32>,
    /// heart count
    pub heart: Option<u32>,
    /// rocket count
    pub rocket: Option<u32>,
    /// eyes count
    pub eyes: Option<u32>,
}

/// Issue情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Issue ID
    pub id: u64,
    /// Issue 番号
    pub number: u32,
    /// Issue タイトル
    pub title: String,
    /// Issue 作成者
    pub user: User,
    /// Issue 状態
    pub state: String,
    /// Issue 状態理由
    pub state_reason: Option<String>,
    /// Issue HTML URL
    pub html_url: String,
    /// Issue API URL
    pub url: String,
    /// Issue コメントURL
    pub comments_url: String,
    /// Issue Events URL
    pub events_url: String,
    /// Issue Assignees URL
    pub assignees_url: String,
    /// Issue Repository URL
    pub repository_url: String,
    /// Issue Labels URL
    pub labels_url: String,
    /// Issue Milestone
    pub milestone: Option<Milestone>,
    /// Issue Labels
    pub labels: Option<Vec<Label>>,
    /// Issue Assignees
    pub assignees: Option<Vec<User>>,
    /// Issue Body
    pub body: Option<String>,
    /// Issue Created At
    pub created_at: DateTime<Utc>,
    /// Issue Updated At
    pub updated_at: DateTime<Utc>,
    /// Issue Closed At
    pub closed_at: Option<DateTime<Utc>>,
    /// Issue Locked
    pub locked: bool,
    /// Issue Active Lock Reason
    pub active_lock_reason: Option<String>,
    /// Issue Comments Count
    pub comments: Option<u32>,
    /// Issue Reactions
    pub reactions: Option<Reactions>,
    /// Issue Assignee
    pub assignee: Option<User>,
    /// Issue Author Association
    pub author_association: String,
    /// Issue Pull Request
    pub pull_request: Option<PullRequestInfo>,
}

/// Pull Request 情報（Issue内でのみ使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    /// Pull Request HTML URL
    pub html_url: Option<String>,
    /// Pull Request Diff URL
    pub diff_url: Option<String>,
    /// Pull Request Patch URL
    pub patch_url: Option<String>,
    /// Pull Request Merged At
    pub merged_at: Option<DateTime<Utc>>,
}

/// Comment 情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Comment ID
    pub id: u64,
    /// Comment HTML URL
    pub html_url: String,
    /// Comment URL
    pub url: String,
    /// Comment Body
    pub body: String,
    /// Comment User
    pub user: User,
    /// Comment Created At
    pub created_at: DateTime<Utc>,
    /// Comment Updated At
    pub updated_at: DateTime<Utc>,
    /// Comment Author Association
    pub author_association: String,
    /// Comment Reactions
    pub reactions: Option<Reactions>,
}

/// Review Comment 情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Review Comment ID
    pub id: u64,
    /// Review Comment HTML URL
    pub html_url: String,
    /// Review Comment URL
    pub url: String,
    /// Review Comment Diff URL
    pub diff_url: String,
    /// Review Comment Pull Request URL
    pub pull_request_url: String,
    /// Review Comment Body
    pub body: String,
    /// Review Comment User
    pub user: User,
    /// Review Comment Created At
    pub created_at: DateTime<Utc>,
    /// Review Comment Updated At
    pub updated_at: DateTime<Utc>,
    /// Review Comment Author Association
    pub author_association: String,
    /// Review Comment Reactions
    pub reactions: Option<Reactions>,
    /// Review Comment Position
    pub position: Option<i32>,
    /// Review Comment Line
    pub line: Option<i32>,
    /// Review Comment Path
    pub path: Option<String>,
    /// Review Comment Commit ID
    pub commit_id: String,
    /// Review Comment Original Position
    pub original_position: Option<i32>,
    /// Review Comment Original Line
    pub original_line: Option<i32>,
    /// Review Comment Original Path
    pub original_path: Option<String>,
    /// Review Comment Original Commit ID
    pub original_commit_id: String,
}

/// Review 情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Review ID
    pub id: u64,
    /// Review Node ID
    pub node_id: String,
    /// Review State
    pub state: String,
    /// Review Body
    pub body: Option<String>,
    /// Review Submitted At
    pub submitted_at: DateTime<Utc>,
    /// Review Author Association
    pub author_association: String,
    /// Review HTML URL
    pub html_url: String,
    /// Review Pull Request URL
    pub pull_request_url: String,
    /// Review User
    pub user: Option<User>,
    /// Review Body HTML
    pub body_html: Option<String>,
    /// Review Body Text
    pub body_text: Option<String>,
    /// Review Comments
    pub comments: Option<Vec<Comment>>,
    /// Review Commit ID
    pub commit_id: Option<String>,
    /// Review Associated Pull Request
    pub pull_request: Option<AssociatedPullRequest>,
}

/// Associated Pull Request 情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedPullRequest {
    /// Pull Request URL
    pub url: String,
    /// Pull Request ID
    pub id: u64,
    /// Pull Request Number
    pub number: u32,
    /// Pull Request HTML URL
    pub html_url: String,
}

/// Webhook Event Type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    PullRequest,
    PullRequestReview,
    PullRequestReviewComment,
    Issue,
    IssueComment,
    CommitComment,
    Push,
    Create,
    Delete,
    Release,
    Fork,
    Star,
    Watch,
    WorkflowRun,
    Schedule,
    Repository,
    Member,
    Membership,
    Organization,
    Team,
    TeamAdd,
    #[serde(rename = "check_run")]
    CheckRun,
    #[serde(rename = "check_suite")]
    CheckSuite,
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "deployment")]
    Deployment,
    #[serde(rename = "deployment_status")]
    DeploymentStatus,
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: 12345,
            login: "testuser".to_string(),
            avatar_url: Some("https://github.com/testuser.png".to_string()),
            html_url: "https://github.com/testuser".to_string(),
            r#type: "User".to_string(),
            site_admin: Some(false),
        };

        assert_eq!(user.login, "testuser");
        assert_eq!(user.id, 12345);
    }

    #[test]
    fn test_repository_creation() {
        let user = User {
            id: 12345,
            login: "owner".to_string(),
            avatar_url: None,
            html_url: "https://github.com/owner".to_string(),
            r#type: "User".to_string(),
            site_admin: None,
        };

        let repository = Repository {
            id: 67890,
            full_name: "owner/test-repo".to_string(),
            owner: user,
            description: Some("A test repository".to_string()),
            html_url: "https://github.com/owner/test-repo".to_string(),
            r#private: false,
            fork: false,
            parent: None,
            template_repository: None,
            default_branch: "main".to_string(),
            master_branch: None,
            permissions: None,
            is_template: Some(false),
            network_count: Some(10),
            subscribers_count: Some(5),
        };

        assert_eq!(repository.full_name, "owner/test-repo");
        assert_eq!(repository.r#private, false);
    }

    #[test]
    fn test_pull_request_creation() {
        let now = Utc::now();

        let pull_request = PullRequest {
            id: 123456,
            number: 1,
            title: "Test PR".to_string(),
            user: User {
                id: 12345,
                login: "testuser".to_string(),
                avatar_url: None,
                html_url: "https://github.com/testuser".to_string(),
                r#type: "User".to_string(),
                site_admin: None,
            },
            state: "open".to_string(),
            merged: Some(false),
            mergeable: Some(true),
            mergeable_state: Some("clean".to_string()),
            merged_at: None,
            merged_by: None,
            merge_commit_sha: None,
            html_url: "https://github.com/owner/repo/pull/1".to_string(),
            url: "https://api.github.com/repos/owner/repo/pulls/1".to_string(),
            comments_url: "https://api.github.com/repos/owner/repo/pulls/1/comments".to_string(),
            review_comments_url: "https://api.github.com/repos/owner/repo/pulls/1/comments"
                .to_string(),
            review_comment_url: "https://api.github.com/repos/owner/repo/pulls/1/reviews"
                .to_string(),
            statuses_url: "https://api.github.com/repos/owner/repo/statuses/1".to_string(),
            issue_url: "https://api.github.com/repos/owner/repo/issues/1".to_string(),
            commits_url: "https://api.github.com/repos/owner/repo/pulls/1/commits".to_string(),
            review_events_url: "https://api.github.com/repos/owner/repo/pulls/1/review_events"
                .to_string(),
            events_url: "https://api.github.com/repos/owner/repo/pulls/1/events".to_string(),
            assignees_url: "https://api.github.com/repos/owner/repo/pulls/1/assignees".to_string(),
            branches_url: "https://api.github.com/repos/owner/repo/pulls/1/branches".to_string(),
            tags_url: "https://api.github.com/repos/owner/repo/tags".to_string(),
            trees_url: "https://api.github.com/repos/owner/repo/git/trees".to_string(),
            svn_url: "https://github.com/owner/repo".to_string(),
            forks_url: "https://api.github.com/repos/owner/repo/forks".to_string(),
            collaborators_url: "https://api.github.com/repos/owner/repo/collaborators".to_string(),
            subscribers_url: "https://api.github.com/repos/owner/repo/subscribers".to_string(),
            subscription_url: "https://api.github.com/repos/owner/repo/subscription".to_string(),
            commits_count: Some(3),
            additions: Some(50),
            deletions: Some(10),
            changed_files: Some(5),
            created_at: now,
            updated_at: now,
            closed_at: None,
            base: BranchInfo {
                label: "main".to_string(),
                ref_name: "refs/heads/main".to_string(),
                sha: "abc123".to_string(),
                user: User {
                    id: 12345,
                    login: "owner".to_string(),
                    avatar_url: None,
                    html_url: "https://github.com/owner".to_string(),
                    r#type: "User".to_string(),
                    site_admin: None,
                },
                repo: Repository {
                    id: 67890,
                    full_name: "owner/repo".to_string(),
                    owner: User {
                        id: 12345,
                        login: "owner".to_string(),
                        avatar_url: None,
                        html_url: "https://github.com/owner".to_string(),
                        r#type: "User".to_string(),
                        site_admin: None,
                    },
                    description: None,
                    html_url: "https://github.com/owner/repo".to_string(),
                    r#private: false,
                    fork: false,
                    parent: None,
                    template_repository: None,
                    default_branch: "main".to_string(),
                    master_branch: None,
                    permissions: None,
                    is_template: None,
                    network_count: None,
                    subscribers_count: None,
                },
            },
            head: BranchInfo {
                label: "feature".to_string(),
                ref_name: "refs/heads/feature".to_string(),
                sha: "def456".to_string(),
                user: User {
                    id: 12345,
                    login: "testuser".to_string(),
                    avatar_url: None,
                    html_url: "https://github.com/testuser".to_string(),
                    r#type: "User".to_string(),
                    site_admin: None,
                },
                repo: Repository {
                    id: 67890,
                    full_name: "testuser/repo".to_string(),
                    owner: User {
                        id: 12345,
                        login: "testuser".to_string(),
                        avatar_url: None,
                        html_url: "https://github.com/testuser".to_string(),
                        r#type: "User".to_string(),
                        site_admin: None,
                    },
                    description: None,
                    html_url: "https://github.com/testuser/repo".to_string(),
                    r#private: false,
                    fork: false,
                    parent: None,
                    template_repository: None,
                    default_branch: "main".to_string(),
                    master_branch: None,
                    permissions: None,
                    is_template: None,
                    network_count: None,
                    subscribers_count: None,
                },
            },
            labels: None,
            assignees: None,
            draft: Some(false),
            active_lock_reason: None,
            body: Some("This is a test PR".to_string()),
            locked: Some(false),
            author_association: Some("CONTRIBUTOR".to_string()),
            milestone: None,
            reactions: None,
        };

        assert_eq!(pull_request.number, 1);
        assert_eq!(pull_request.title, "Test PR");
        assert_eq!(pull_request.state, "open");
    }

    #[test]
    fn test_notification_reason_display() {
        use crate::github::types::NotificationReason;

        assert_eq!(
            NotificationReason::ReviewRequested.to_string(),
            "review_requested"
        );
        assert_eq!(NotificationReason::Mention.to_string(), "mention");
        assert_eq!(NotificationReason::Comment.to_string(), "comment");
        assert_eq!(NotificationReason::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_webhook_event_type_display() {
        assert_eq!(WebhookEventType::PullRequest.to_string(), "pull_request");
        assert_eq!(WebhookEventType::Issue.to_string(), "issue");
        assert_eq!(WebhookEventType::Push.to_string(), "push");
        assert_eq!(WebhookEventType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_reactions_serialization() {
        let reactions = Reactions {
            total_count: Some(5),
            plus_one: Some(3),
            minus_one: Some(1),
            laugh: Some(1),
            hooray: Some(2),
            confused: Some(0),
            heart: Some(1),
            rocket: Some(0),
            eyes: Some(1),
        };

        let serialized = serde_json::to_string(&reactions).expect("Failed to serialize Reactions");
        assert!(serialized.contains("\"total_count\":5"));
        assert!(serialized.contains("\"+1\":3"));
        assert!(serialized.contains("\"-1\":1"));

        let deserialized: Reactions =
            serde_json::from_str(&serialized).expect("Failed to deserialize Reactions");
        assert_eq!(deserialized.total_count, Some(5));
        assert_eq!(deserialized.plus_one, Some(3));
    }

    #[test]
    fn test_notification_serialization() {
        let now = Utc::now();

        let notification = Notification {
            id: "12345".to_string(),
            repository: Repository {
                id: 67890,
                full_name: "owner/repo".to_string(),
                owner: User {
                    id: 12345,
                    login: "owner".to_string(),
                    avatar_url: None,
                    html_url: "https://github.com/owner".to_string(),
                    r#type: "User".to_string(),
                    site_admin: None,
                },
                description: None,
                html_url: "https://github.com/owner/repo".to_string(),
                r#private: false,
                fork: false,
                parent: None,
                template_repository: None,
                default_branch: "main".to_string(),
                master_branch: None,
                permissions: None,
                is_template: None,
                network_count: None,
                subscribers_count: None,
            },
            subject: NotificationSubject {
                title: "Test Issue".to_string(),
                subject_type: "Issue".to_string(),
                kind: "Issue".to_string(), // Add the missing kind field
                url: Some("https://github.com/owner/repo/issues/1".to_string()),
                latest_comment_url: None,
                html_url: Some("https://github.com/owner/repo/issues/1".to_string()),
            },
            reason: NotificationReason::Assign,
            unread: true,
            updated_at: now,
            last_read_at: None,
            url: "https://github.com/owner/repo/issues/1".to_string(),
            api_url: "https://api.github.com/notifications/threads/12345".to_string(),
            html_url: Some("https://github.com/owner/repo/issues/1".to_string()),
        };

        let serialized =
            serde_json::to_string(&notification).expect("Failed to serialize Notification");
        assert!(serialized.contains("\"id\":\"12345\""));
        assert!(serialized.contains("\"unread\":true"));

        let deserialized: Notification =
            serde_json::from_str(&serialized).expect("Failed to deserialize Notification");
        assert_eq!(deserialized.id, "12345");
        assert_eq!(deserialized.unread, true);
    }
}

impl std::fmt::Display for WebhookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookEventType::PullRequest => write!(f, "pull_request"),
            WebhookEventType::PullRequestReview => write!(f, "pull_request_review"),
            WebhookEventType::PullRequestReviewComment => write!(f, "pull_request_review_comment"),
            WebhookEventType::Issue => write!(f, "issue"),
            WebhookEventType::IssueComment => write!(f, "issue_comment"),
            WebhookEventType::CommitComment => write!(f, "commit_comment"),
            WebhookEventType::Push => write!(f, "push"),
            WebhookEventType::Create => write!(f, "create"),
            WebhookEventType::Delete => write!(f, "delete"),
            WebhookEventType::Release => write!(f, "release"),
            WebhookEventType::Fork => write!(f, "fork"),
            WebhookEventType::Star => write!(f, "star"),
            WebhookEventType::Watch => write!(f, "watch"),
            WebhookEventType::WorkflowRun => write!(f, "workflow_run"),
            WebhookEventType::Schedule => write!(f, "schedule"),
            WebhookEventType::Repository => write!(f, "repository"),
            WebhookEventType::Member => write!(f, "member"),
            WebhookEventType::Membership => write!(f, "membership"),
            WebhookEventType::Organization => write!(f, "organization"),
            WebhookEventType::Team => write!(f, "team"),
            WebhookEventType::TeamAdd => write!(f, "team_add"),
            WebhookEventType::CheckRun => write!(f, "check_run"),
            WebhookEventType::CheckSuite => write!(f, "check_suite"),
            WebhookEventType::Status => write!(f, "status"),
            WebhookEventType::Deployment => write!(f, "deployment"),
            WebhookEventType::DeploymentStatus => write!(f, "deployment_status"),
            WebhookEventType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Pull Request Action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestAction {
    Opened,
    Closed,
    Reopened,
    Edited,
    Assigned,
    Unassigned,
    Labeled,
    Unlabeled,
    ReviewRequested,
    ReviewRequestRemoved,
    ReadyForReview,
    Locked,
    Unlocked,
    AutoMergeEnabled,
    AutoMergeDisabled,
    ConvertToDraft,
    Merged,
    #[serde(other)]
    Unknown,
}

/// Pull Request Review Action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestReviewAction {
    Submitted,
    Edited,
    Dismissed,
    #[serde(other)]
    Unknown,
}

/// Issue Comment Action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCommentAction {
    Created,
    Edited,
    Deleted,
    #[serde(other)]
    Unknown,
}

/// Webhook Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Event Type
    #[serde(rename = "X-GitHub-Event")]
    pub event_type: WebhookEventType,
    /// Event Payload
    pub payload: WebhookPayload,
    /// Repository
    pub repository: Repository,
    /// Sender
    pub sender: User,
    /// Installation
    pub installation: Option<Installation>,
}

/// Webhook Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WebhookPayload {
    #[serde(rename = "opened")]
    PullRequestOpened { pull_request: PullRequest },
    #[serde(rename = "closed")]
    PullRequestClosed { pull_request: PullRequest },
    #[serde(rename = "reopened")]
    PullRequestReopened { pull_request: PullRequest },
    #[serde(rename = "edited")]
    PullRequestEdited { pull_request: PullRequest },
    #[serde(rename = "assigned")]
    PullRequestAssigned { pull_request: PullRequest },
    #[serde(rename = "unassigned")]
    PullRequestUnassigned { pull_request: PullRequest },
    #[serde(rename = "labeled")]
    PullRequestLabeled { pull_request: PullRequest },
    #[serde(rename = "unlabeled")]
    PullRequestUnlabeled { pull_request: PullRequest },
    #[serde(rename = "review_requested")]
    PullRequestReviewRequested { pull_request: PullRequest },
    #[serde(rename = "review_request_removed")]
    PullRequestReviewRequestRemoved { pull_request: PullRequest },
    #[serde(rename = "ready_for_review")]
    PullRequestReadyForReview { pull_request: PullRequest },
    #[serde(rename = "locked")]
    PullRequestLocked { pull_request: PullRequest },
    #[serde(rename = "unlocked")]
    PullRequestUnlocked { pull_request: PullRequest },
    #[serde(rename = "merged")]
    PullRequestMerged { pull_request: PullRequest },
    #[serde(other)]
    Unknown,
}

/// GitHub App Installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    /// Installation ID
    pub id: u64,
    /// Installation Node ID
    pub node_id: String,
    /// Installation Account
    pub account: User,
    /// Installation Created At
    pub created_at: DateTime<Utc>,
    /// Installation Updated At
    pub updated_at: DateTime<Utc>,
}

/// GitHub Rate Limit Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Rate Limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Reset time
    pub reset: u64,
    /// Used requests
    pub used: u32,
    /// Resource type
    pub resource: String,
}

/// GitHub Rate Limit Response Headers
#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    /// Rate Limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Reset time
    pub reset: u64,
}

/// Notification Subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubject {
    /// Subject Title
    pub title: String,
    /// Subject Type
    #[serde(rename = "type")]
    pub subject_type: String,
    /// Subject URL
    pub url: Option<String>,
    /// Subject Latest Comment URL
    pub latest_comment_url: Option<String>,
    /// Subject HTML URL
    pub html_url: Option<String>,

    /// Subject Kind (alias for subject_type)
    #[serde(skip)]
    pub kind: String,
}

impl NotificationSubject {
    /// Kindを取得（subject_typeのエイリアス）
    pub fn kind(&self) -> &str {
        &self.subject_type
    }
}

// NotificationSubjectのコンストラクタを提供
impl NotificationSubject {
    pub fn new(
        title: String,
        subject_type: String,
        url: Option<String>,
        latest_comment_url: Option<String>,
        html_url: Option<String>,
    ) -> Self {
        Self {
            title,
            subject_type: subject_type.clone(),
            url,
            latest_comment_url,
            html_url,
            kind: subject_type,
        }
    }
}

/// Notification Reason
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationReason {
    #[serde(rename = "assign")]
    Assign,
    #[serde(rename = "author")]
    Author,
    #[serde(rename = "comment")]
    Comment,
    #[serde(rename = "invitation")]
    Invitation,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "mention")]
    Mention,
    #[serde(rename = "review_requested")]
    ReviewRequested,
    #[serde(rename = "security_alert")]
    SecurityAlert,
    #[serde(rename = "state_change")]
    StateChange,
    #[serde(rename = "subscribed")]
    Subscribed,
    #[serde(rename = "team_mention")]
    TeamMention,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for NotificationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationReason::Assign => write!(f, "assign"),
            NotificationReason::Author => write!(f, "author"),
            NotificationReason::Comment => write!(f, "comment"),
            NotificationReason::Invitation => write!(f, "invitation"),
            NotificationReason::Manual => write!(f, "manual"),
            NotificationReason::Mention => write!(f, "mention"),
            NotificationReason::ReviewRequested => write!(f, "review_requested"),
            NotificationReason::SecurityAlert => write!(f, "security_alert"),
            NotificationReason::StateChange => write!(f, "state_change"),
            NotificationReason::Subscribed => write!(f, "subscribed"),
            NotificationReason::TeamMention => write!(f, "team_mention"),
            NotificationReason::Unknown => write!(f, "unknown"),
        }
    }
}

/// GitHub Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Notification ID
    pub id: String,
    /// Repository
    pub repository: Repository,
    /// Subject
    pub subject: NotificationSubject,
    /// Notification Reason
    pub reason: NotificationReason,
    /// Notification Unread
    pub unread: bool,
    /// Notification Updated At
    pub updated_at: DateTime<Utc>,
    /// Notification Last Read At
    pub last_read_at: Option<DateTime<Utc>>,
    /// Notification URL
    pub url: String,
    /// Notification API URL
    pub api_url: String,
    /// Notification HTML URL
    pub html_url: Option<String>,
}
