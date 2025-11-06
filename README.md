# GitHubリアルタイムデスクトップ通知デーモン (gh-notifier)

Rustで構築された軽量デーモンアプリケーションで、GitHub通知のリアルタイム（または準リアルタイム）デスクトップ通知を提供します。GitHub Personal Access Token (PAT) を使用した安全な認証方式を採用しています。

## 機能

- **リアルタイムGitHub通知**: Issue、プルリクエスト、メンションなどに関する通知をリアルタイムで受信します。
- **ネイティブデスクトップ通知**: Linux（D-Bus）、macOS（NSUserNotification）、Windows（ToastNotification）のネイティブ通知システムと統合します。
- **PAT (Personal Access Token) 認証**: GitHub Personal Access Token を使用した安全な認証。
- **安全なトークン保存**: アクセストークンはOSキーチェーン（macOS Keychain、Windowsクレデンシャルボルト、Linux libsecret）に安全に保存されます。
- **低リソース使用**: CPUやメモリ消費が最小限の軽量デーモンです。
- **クロスプラットフォーム**: Linux（GNOME/KDE）、macOS、Windows 10/11で動作。
- **構成可能なポーリング**: 通知ポーリング間隔やフィルタリング設定が可能です。
- **安全なシャットダウン**: SIGINT/SIGTERMシグナルによる安全な終了処理。

## アーキテクチャ

アプリケーションは以下の主要コンポーネントで構成されています：

- `Config` - TOMLファイルからの設定読み込みと管理
- `AuthManager` - PAT認証の処理、トークンの保存と検証、OSキーチェーン操作
- `GitHubClient` - 認証済みHTTPクライアント（自動トークン付与）
- `Poller` - 通知ポーリングと通知送信
- `Notifier` - OS依存の通知送信
- `StateManager` - ETagと最終取得時刻の管理
- `Logger` - `tracing`ベースの構造化ログ
- `Polling Module` - 通知のフィルタリング、処理、ポーリングループの実行をそれぞれ別々に管理（filter.rs, handler.rs, runner.rs）
- `InitializationService` - アプリケーションの初期化処理
- `Shutdown` - シグナルハンドリングによる安全なシャットダウン処理

## インストール

### 前提条件

- Rust 1.75以上
- ネイティブ通知およびキーチェーンアクセス用のシステム依存パッケージ
  - Linux: libsecret-devel, libdbus-1-dev など
  - macOS: システム通知が有効
  - Windows: WinRT通知APIが利用可能

### ソースからのビルド

```bash
git clone https://github.com/mopemope/gh-notifier.git
cd gh-notifier
cargo build --release
```

実行可能ファイルは `target/release/gh-notifier` で利用可能です。

## 使用方法

## 使用方法

### 初期設定と認証

1. 設定ファイル `~/.config/gh-notifier/config.toml` を作成するか編集します
2. GitHubのPersonal Access Tokenを設定ファイルの `pat` フィールドに追加します：

```toml
pat = "your_personal_access_token_here"
```

### Personal Access Token の作成方法

1. GitHubの [Personal Access Token](https://github.com/settings/tokens) ページにアクセス
2. 「Generate new token」または「Fine-grained personal access token」を選択
3. 以下の権限を付与:
   - `notifications` - 通知の読み取り
   - `repo` (オプション) - 通知を既読にする場合
4. 生成されたトークンをコピーし、設定ファイルの `pat` フィールドに設定

### 実行

```bash
./target/release/gh-notifier
```

プログラムは認証が完了している場合、自動的にバックグラウンドで実行され、定期的にGitHub通知をポーリングします。

### シャットダウン

プログラムを終了するには `Ctrl+C` (SIGINT) または `SIGTERM` シグナルを送信します：

```bash
# Ctrl+C を使用して終了
# または
pkill -TERM gh-notifier
```

プログラムは受信したシグナルに応じて安全に終了し、状態を保存して終了します。

## 設定

設定ファイルは以下の場所に保存されます：
- Linux/macOS: `~/.config/gh-notifier/config.toml`
- Windows: `%APPDATA%\gh-notifier\config.toml`

デフォルト設定：
```toml
pat = "your_personal_access_token_here"  # GitHub Personal Access Token
poll_interval_sec = 30                   # 通知ポーリング間隔（秒）
mark_as_read_on_notify = false           # 通知表示時に既読にするか
log_level = "info"                       # ログレベル（info, debug, warn, error）
log_file_path = ""                       # ログファイルの保存パス（省略可能）

# 通知フィルタリング設定（デフォルトでは自分宛てのPRレビュー依頼のみ通知）
[notification_filters]
include_reasons = ["review_requested"]     # 受け取る通知理由（レビュー依頼のみ）
include_subject_types = ["PullRequest"]    # 受け取る通知タイプ（プルリクエストのみ）
exclude_repositories = []                  # 除外するリポジトリのリスト
exclude_reasons = []                       # 除外する通知理由のリスト
exclude_draft_prs = false                  # ドラフトPRの通知を除外するかどうか
exclude_private_repos = false              # プライベートリポジトリの通知を除外するかどうか
exclude_fork_repos = false                 # フォークリポジトリの通知を除外するかどうか
exclude_participating = false              # 参加しているスレッドの通知を除外するかどうか

## 自分宛てのPRレビューのみを通知する設定方法

自分宛てのプルリクエストレビュー依頼のみを通知するように設定するには、以下の2つの方法があります：

### 方法1: デフォルト設定を使用（推奨）
アプリケーションはデフォルトで自分宛てのPRレビュー依頼のみを通知するように設定されています。特別な設定をしなくても、`include_reasons = ["review_requested"]` と `include_subject_types = ["PullRequest"]` がデフォルトで適用されるため、特に設定ファイルを変更する必要はありません。

### 方法2: 設定ファイルで明示的に指定
設定ファイルで明示的に指定する場合、以下のように設定します：

```toml
[notification_filters]
# 自分宛てのPRレビュー依頼のみを通知する設定
include_reasons = ["review_requested"]     # レビュー依頼のみを通知
include_subject_types = ["PullRequest"]    # プルリクエストのみを対象

# オプションで追加のフィルタを設定可能
exclude_draft_prs = false                  # ドラフトPRを除外しない（trueにするとドラフトPRは通知されません）
exclude_private_repos = false              # プライベートリポジトリを除外しない
```

この設定により、GitHub APIから届くすべての通知の中で、自分にレビューが依頼されたプルリクエストのみがデスクトップ通知として表示されます。

# 通知バッチ処理設定（バッチ処理を無効にするにはbatch_size = 0）
[notification_batch_config]
batch_size = 0                           # 通知バッチの最大数（0で無効）
batch_interval_sec = 30                  # バッチ処理の間隔（秒）

# ポーリングエラーハンドリング設定
[polling_error_handling_config]
retry_count = 3                          # エラー発生時の再試行回数
retry_interval_sec = 5                   # 再試行間隔（秒）


```

## 設定例

### 自分宛てのPRレビュー依頼のみを通知（デフォルト）
アプリケーションはデフォルトで自分宛てのPRレビュー依頼のみを通知するように設定されています。この設定は以下のように明示的に設定することもできます：

```toml
[notification_filters]
include_reasons = ["review_requested"]
include_subject_types = ["PullRequest"]
```

この設定により、自分にレビューが依頼されたプルリクエストのみが通知されます。他のすべての通知（メンション、コメント、Issueの更新など）はフィルタリングされます。

### すべてのメンションとPRレビュー依頼を通知
```toml
[notification_filters]
include_reasons = ["mention", "review_requested"]
include_subject_types = ["Issue", "PullRequest"]
```

### PRレビュー依頼のみを通知（より明示的な例）
特定の通知理由とタイプのみを受信する場合、以下の設定が最も明確です：

```toml
[notification_filters]
# 自分にレビューが依頼されたプルリクエストのみを通知
include_reasons = ["review_requested"]     # レビュー依頼のみ
include_subject_types = ["PullRequest"]    # プルリクエストのみ
# 他のすべての通知は除外されます
```

この設定では、GitHubから届くすべての通知がフィルタリングされ、自分にレビューが依頼されたプルリクエストの通知だけがデスクトップ通知として表示されます。

### すべてのメンションとPRレビュー依頼を通知
```toml
[notification_filters]
include_reasons = ["mention", "review_requested"]
include_subject_types = ["Issue", "PullRequest"]
```

### 特定のリポジトリからの通知のみを受け取る
```toml
[notification_filters]
include_repositories = [
  "your-org/important-project",
  "your-org/another-project"
]
```

### 自分が所属する組織からの通知のみを受け取る
```toml
[notification_filters]
include_organizations = ["your-org"]
```

### 特定キーワードを含む通知のみを受け取る
```toml
[notification_filters]
title_contains = ["urgent", "critical", "bug"]
```

### プライベートリポジトリの通知を除外
```toml
[notification_filters]
exclude_private_repos = true
```

### 最近24時間以内の通知のみを受け取る
```toml
[notification_filters]
minimum_updated_time = "24h"
```

### 複合フィルター例（レビュー依頼 + 重要なキーワード + 指定リポジトリのみ）
```toml
[notification_filters]
include_reasons = ["review_requested"]
include_subject_types = ["PullRequest"]
title_contains = ["important", "critical"]
include_repositories = ["important-org/main-repo"]
exclude_private_repos = false
```

### ドラフトPRの通知を除外
```toml
[notification_filters]
exclude_draft_prs = true  # ドラフト状態のプルリクエストの通知を除外
```

### フォークリポジトリの通知を除外
```toml
[notification_filters]
exclude_fork_repos = true  # フォークリポジトリからの通知を除外
```

### 参加しているスレッドの通知を除外
```toml
[notification_filters]
exclude_participating = true  # 自分が参加しているスレッドの通知を除外（現在のところ完全には実装されていません）
```

## 設定オプションの詳細

- `poll_interval_sec`: GitHub APIから通知をポーリングする間隔（秒単位）。デフォルトは30秒。
- `mark_as_read_on_notify`: trueにすると、通知表示時に自動的にGitHub上で通知を既読に設定します。
- `log_level`: ログの詳細度（info, debug, warn, error）。デフォルトはinfo。
- `log_file_path`: ログファイルの保存パス（省略可能、デフォルト: データディレクトリ下の logs/gh-notifier.log）

### 通知フィルタリングオプション

#### リポジトリベースのフィルタリング
- `include_repositories`: 通知を受け取りたいリポジトリのリスト（指定されたリポジトリからのみ通知を受信）
- `exclude_repositories`: 通知を受け取りたくないリポジトリのリスト
- `include_organizations`: 通知を受け取りたい組織のリスト（指定された組織のリポジトリからのみ通知を受信）
- `exclude_organizations`: 通知を受け取りたくない組織のリスト
- `exclude_private_repos`: trueにすると、プライベートリポジトリからの通知を除外します
- `exclude_fork_repos`: trueにすると、フォークリポジトリからの通知を除外します

#### 通知タイプベースのフィルタリング
- `include_subject_types`: 通知を受け取りたい通知タイプのリスト（例: "Issue", "PullRequest", "Commit", "Release"）
- `exclude_subject_types`: 通知を受け取りたくない通知タイプのリスト
- `include_reasons`: 通知を受け取りたい通知理由のリスト（指定された理由のみ通知を受信）
- `exclude_reasons`: 通知を受け取りたくない通知理由のリスト（例: "mention", "comment", "subscribed" など）

#### コンテンツベースのフィルタリング
- `title_contains`: 通知タイトルに含まれるべきキーワードのリスト（指定されたキーワードを含むタイトルのみ通知）
- `title_not_contains`: 通知タイトルに含まれてはいけないキーワードのリスト
- `repository_contains`: 通知を受け取りたいリポジトリ名に含まれるべきキーワードのリスト

#### 高度なフィルタリング
- `minimum_updated_time`: 通知の最小更新時間（例: "1h", "30m", "2d"）。この時間より古い通知は除外されます
- `exclude_draft_prs`: ドラフト状態のプルリクエストの通知を除外するかどうか（trueにするとドラフトPRの通知が表示されません）
- `exclude_participating`: 参加しているスレッドの通知を除外するかどうか（現在のところ完全には実装されていません。GitHub APIの通知レスポンスにはparticipatingフィールドが含まれないため、機能は定義されていますが実際には動作しません）

#### 通知理由の種類 (Reasons)
通知のフィルタリングで使用できる理由の種類:

- `assign`: 自分にアサインされた場合
- `author`: 自分が作成したリソースに関する通知（例: 自分が作成したIssueの更新）
- `comment`: 自分の投稿に対するコメント
- `invitation`: リポジトリへの招待
- `manual`: 手動でメンションされた（例: `@username`）
- `mention`: 自分へのメンション（`manual`と同義）
- `review_requested`: 自分にレビューが依頼された
- `security_alert`: セキュリティアラート
- `state_change`: 自分が関連するIssue/Pull Requestの状態変更（オープン、クローズなど）
- `subscribed`: 購読しているリポジトリでのアクティビティ

## セキュリティ

- トークンはOSキーチェーンに安全に保存されます
- トークンはログに出力されたり公開されたりすることはありません
- ファイルパーミッションは安全に設定されています
- Personal Access Token 認証により、安全なアクセスを実現

## 技術的詳細

- **ポーリング方式**: ETagおよびIf-Modified-Sinceヘッダーを使用した効率的なポーリングを行うGitHub REST API v3 `/notifications`エンドポイント
- **認証方式**: PAT (Personal Access Token) 認証、`notifications` スコープが必要
- **ログ**: `tracing`クレートによる構造化ログ（ファイル出力もサポート）
- **非同期ランタイム**: 非同期操作のためのTokioベース
- **シャットダウン処理**: SIGINT/SIGTERMシグナルをキャッチして安全に終了するイベントループ
- **タスク管理**: `tokio::spawn`を使用した非同期タスクの実行
- **フィルタリング**: 様々なフィルタリング条件に基づいた高度な通知フィルタリング
- **バッチ処理**: 通知のバッチ処理機能（設定可能なバッチサイズと間隔）
- **エラーハンドリング**: ポーリングエラーに対する再試行ロジック（設定可能な回数と間隔）

## トラブルシューティング

### 認証に関する問題
- 初回認証時にPersonal Access Tokenが無効な場合、再入力を求められます
- トークンの権限が不足している場合、特定のAPI機能が利用できません

### 通知に関する問題
- 通知が表示されない場合は、OSの通知設定が有効であることを確認してください
- 通知の頻度を調整するには、設定ファイルで`poll_interval_sec`を変更してください

### ログの確認
- `log_level`を`debug`に設定すると、より詳細なログを確認できます
- ログは標準出力に構造化形式で表示されます

## ライセンス

このプロジェクトのライセンスはLICENSEファイルに記載されています。

## 貢献

貢献は歓迎します！プルリクエストを気軽に送ってください。

## 謝辞

- パフォーマンスとメモリ安全性のためのRust構築
- 様々な機能に使用されるcrates.ioの最新クレート
- ネイティブ通知システムを使用したクロスプラットフォーム対応