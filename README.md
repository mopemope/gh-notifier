# GitHubリアルタイムデスクトップ通知デーモン (gh-notifier)

Rustで構築された軽量デーモンアプリケーションで、GitHub通知のリアルタイム（または準リアルタイム）デスクトップ通知を提供します。個人用アクセストークン（PAT）を手動で生成する必要をなくす、安全なGitHub OAuth Device Flow認証方式を採用しています。

## 機能

- **リアルタイムGitHub通知**: Issue、プルリクエスト、メンションなどに関する通知をリアルタイムで受信します。
- **ネイティブデスクトップ通知**: Linux（D-Bus）、macOS（NSUserNotification）、Windows（ToastNotification）のネイティブ通知システムと統合します。
- **OAuth Device Flow認証**: 手動での個人用アクセストークン生成を必要としない安全な認証。
- **安全なトークン保存**: アクセストークンはOSキーチェーン（macOS Keychain、Windowsクレデンシャルボルト、Linux libsecret）に安全に保存されます。
- **自動トークン更新**: トークンの期限切れ時に自動的に更新します。
- **低リソース使用**: CPUやメモリ消費が最小限の軽量デーモンです。
- **クロスプラットフォーム**: Linux（GNOME/KDE）、macOS、Windows 10/11で動作。
- **構成可能なポーリング**: 通知ポーリング間隔やフィルタリング設定が可能です。
- **安全なシャットダウン**: SIGINT/SIGTERMシグナルによる安全な終了処理。

## アーキテクチャ

アプリケーションは以下の主要コンポーネントで構成されています：

- `ConfigLoader` - TOMLファイルからの設定読み込み
- `AuthManager` - OAuth Device Flowの処理、トークンの保存と更新、OSキーチェーン操作
- `GitHubClient` - 認証済みHTTPクライアント（自動トークン付与）
- `Poller` - 通知ポーリングと新規通知の検出
- `Notifier` - OS依存の通知送信
- `StateManager` - ETagと最終取得時刻の管理
- `Logger` - `tracing`ベースの構造化ログ

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

### 初期設定と認証

初回実行時に、GitHubのOAuth Device Flowを使用した認証プロンプトが表示されます：

1. ターミナル上で `./target/release/gh-notifier` を実行
2. アプリケーションが以下のように表示します：
   ```
   GitHub Notifier starting...
   No existing token found, starting OAuth Device Flow...
   No authentication token found. Starting authentication process...
   ```
3. GitHubから認証コードとURLが提供されます
4. 指示に従ってWebブラウザでURLにアクセスし、コードを入力してGitHubアカウントで認証し、アプリケーションを承認します
5. アプリケーションがトークンを受信し、OSのキーチェーンに安全に保存します

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
poll_interval_sec = 30                    # 通知ポーリング間隔（秒）
mark_as_read_on_notify = false           # 通知表示時に既読にするか
client_id = "Iv1.898a6d2a86c3f7aa"      # GitHub OAuth App Client ID
log_level = "info"                       # ログレベル（info, debug, warn, error）

# 通知フィルタリング設定の例
[notification_filters]
exclude_repositories = []                # 除外するリポジトリのリスト
exclude_reasons = []                     # 除外する通知理由のリスト

# 通知バッチ処理設定（バッチ処理を無効にするにはbatch_size = 0）
[notification_batch_config]
batch_size = 0                           # 通知バッチの最大数（0で無効）
batch_interval_sec = 30                  # バッチ処理の間隔（秒）

# ポーリングエラーハンドリング設定
[polling_error_handling_config]
retry_count = 3                          # エラー発生時の再試行回数
retry_interval_sec = 5                   # 再試行間隔（秒）
```

## 設定オプションの詳細

- `poll_interval_sec`: GitHub APIから通知をポーリングする間隔（秒単位）。デフォルトは30秒。
- `mark_as_read_on_notify`: trueにすると、通知表示時に自動的にGitHub上で通知を既読に設定します。
- `log_level`: ログの詳細度（info, debug, warn, error）。デフォルトはinfo。
- `client_id`: GitHub OAuthアプリケーションのクライアントID。デフォルトは組み込みのID。

### 通知フィルタリングオプション

#### リポジトリベースのフィルタリング
- `include_repositories`: 通知を受け取りたいリポジトリのリスト（指定されたリポジトリからのみ通知を受信）
- `exclude_repositories`: 通知を受け取りたくないリポジトリのリスト
- `include_organizations`: 通知を受け取りたい組織のリスト（指定された組織のリポジトリからのみ通知を受信）
- `exclude_organizations`: 通知を受け取りたくない組織のリスト
- `exclude_private_repos`: trueにすると、プライベートリポジトリからの通知を除外します

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

## セキュリティ

- トークンはOSキーチェーンに安全に保存されます
- アクセストークンは期限切れ時に自動的に更新されます
- トークンはログに出力されたり公開されたりすることはありません
- ファイルパーミッションは安全に設定されています
- OAuth Device Flow認証により、個人用アクセストークンの手動作成が不要です

## 技術的詳細

- **ポーリング方式**: ETagおよびIf-Modified-Sinceヘッダーを使用した効率的なポーリングを行うGitHub REST API v3 `/notifications`エンドポイント
- **認証スコープ**: 通知の読み取りに`notifications`スコープが必要、既読に設定する場合はオプションで`repo`スコープ
- **ログ**: `tracing`クレートによる構造化ログ
- **非同期ランタイム**: 非同期操作のためのTokioベース
- **シャットダウン処理**: SIGINT/SIGTERMシグナルをキャッチして安全に終了するイベントループ
- **タスク管理**: `tokio::spawn`を使用した非同期タスクの実行

## トラブルシューティング

### 認証に関する問題
- 最初の認証時にGitHubで承認を拒否すると、再度OAuthフローを実行する必要があります
- トークンの有効期限切れ時には、自動的に再認証プロセスが開始されます

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