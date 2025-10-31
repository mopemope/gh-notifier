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

### ソースからのビルド

```bash
git clone https://github.com/mopemope/gh-notifier.git
cd gh-notifier
cargo build --release
```

実行可能ファイルは `target/release/gh-notifier` で利用可能です。

## 使用方法

初回実行時に、GitHubのOAuth Device Flowを使用した認証プロンプトが表示されます：

1. アプリケーションが認証コードとURLを表示します
2. URLにアクセスし、コードを入力してGitHubアカウントで認証します
3. アプリケーションを承認します
4. アプリケーションがトークンを受信し、安全に保存します

認証後、デーモンはバックグラウンドで実行され、30秒ごとに（設定可能）GitHubから新規通知をポーリングします。

## 設定

設定ファイルは以下の場所に保存されます：
- Linux/macOS: `~/.config/gh-notify-daemon/config.toml`
- Windows: `%APPDATA%\gh-notify-daemon\config.toml`

デフォルト設定：
```toml
poll_interval_sec = 30  # ポーリング間隔（秒）
mark_as_read_on_notify = false  # 通知表示時に既読にするか
client_id = "Iv1.xxxxxxxxxxxxxxxx"  # GitHub OAuth App Client ID
```

## セキュリティ

- トークンはOSキーチェーンに安全に保存されます
- アクセストークンは期限切れ時に自動的に更新されます
- トークンはログに出力されたり公開されたりすることはありません
- ファイルパーミッションは安全に設定されています（600）

## 技術的詳細

- **ポーリング方式**: ETagおよびIf-Modified-Sinceヘッダーを使用した効率的なポーリングを行うGitHub REST API v3 `/notifications`エンドポイント
- **認証スコープ**: 通知の読み取りに`notifications`スコープが必要、既読に設定する場合はオプションで`repo`スコープ
- **ログ**: `tracing`クレートによる構造化ログ
- **非同期ランタイム**: 非同期操作のためのTokioベース

## ライセンス

このプロジェクトのライセンスはLICENSEファイルに記載されています。

## 貢献

貢献は歓迎します！プルリクエストを気軽に送ってください。

## 謝辞

- パフォーマンスとメモリ安全性のためのRust構築
- 様々な機能に使用されるcrates.ioの最新クレート
- ネイティブ通知システムを使用したクロスプラットフォーム対応