---
id: doc-1
title: GitHub通知リアルタイムデスクトップ通知デーモン仕様書
type: other
created_date: '2025-10-31 11:17'
updated_date: '2025-10-31 11:18'
---
# GitHub通知リアルタイムデスクトップ通知デーモン仕様書（PAT Config版）

## 1. 概要

本ドキュメントは、GitHub の通知をリアルタイム（または準リアルタイム）で受信し、デスクトップ通知としてユーザーに表示するための軽量なデーモンプログラムの仕様を定義します。  
認証方式として **設定ファイルに記載されたGitHub Personal Access Token (PAT)** を使用し、安全な認証を実現します。  
開発言語は **Rust** を使用し、最新の非同期アーキテクチャとモダンなライブラリ群を活用します。

---

## 2. 目的

- GitHub の通知（Issues, Pull Requests, Mentions など）をリアルタイムに受信
- ユーザーのデスクトップ環境（Linux/macOS/Windows）にネイティブ通知を表示
- **PAT の設定ファイル読み込みによるシンプルかつ安全な認証を実現**
- 軽量・低リソース消費・長時間のバックグラウンド実行を可能にする
- 認証情報（アクセストークン）を OS キーチェーンに安全に保存（オプション）

---

## 3. 動作環境

- **OS**: Linux (GNOME/KDE), macOS, Windows 10/11
- **Rust バージョン**: 1.75 以上
- **依存ライブラリ**: crates.io の最新安定版を使用

---

## 4. 機能要件

### 4.1 GitHub通知の取得

- GitHub REST API v3 の `/notifications` エンドポイントを定期的にポーリング
- 最終確認時刻以降の未読通知を取得
- ETag / If-Modified-Since ヘッダーによる効率的なポーリング
- ポーリング間隔：デフォルト 30 秒（設定可能）

> ※ Webhook はパーソナル用途では設定が困難なため、ポーリング方式を採用

### 4.2 デスクトップ通知の表示

- OSネイティブ通知システムを使用：
  - Linux: D-Bus 経由（`notify-rust`）
  - macOS: `NSUserNotification`（`mac-notification-sys`）
  - Windows: `ToastNotification`（`winrt-notification`）
- 通知内容：
  - タイトル：リポジトリ名 / 通知の種類
  - 本文：通知の概要
  - クリックでブラウザで通知詳細ページを開く

### 4.3 認証：PAT 設定ファイル

#### 起動時：
1. アプリが設定ファイル `~/.config/gh-notifier/config.toml` を読み込み
2. 設定ファイル内の `pat` フィールドから GitHub Personal Access Token を取得
3. トークンの有効性を検証

#### 認証スコープ（Scope）
- 必須スコープ：`notifications`（通知の読み取り）
- オプション：`repo`（既読マークを付ける場合）

#### トークンの保存
- トークンは設定ファイルに保存されるため、OS キーチェーンへの保存はオプション
- Rust では [`keyring`](https://crates.io/crates/keyring) クレートを使用（必要に応じて）

#### トークンの更新
- PAT は手動で更新されるため、アプリケーションでの自動更新機能は不要
- トークンが無効な場合 → エラーメッセージを表示し、設定ファイルの更新を促す

### 4.4 設定ファイル

- 設定ファイル（TOML）で以下を管理：
  - `pat`: GitHub Personal Access Token
  - `poll_interval_sec`: ポーリング間隔（秒）
  - `mark_as_read_on_notify`: 通知表示時に自動で「既読」にするか（デフォルト: false）
- 設定ファイルの保存場所：
  - Linux/macOS: `~/.config/gh-notifier/config.toml`
  - Windows: `%APPDATA%\gh-notifier\config.toml`

### 4.5 ログ出力

- `tracing` による構造化ログ
- ログレベル：`info`, `warn`, `error`, `debug`
- 認証コードやトークンは **絶対にログに出力しない**

### 4.6 バックグラウンド実行

- 常駐プロセスとして実行
- シグナル（`SIGINT`, `SIGTERM`）で安全に終了
- 初回認証時は対話モード（stdout にコード表示）、その後はバックグラウンド移行

---

## 5. 非機能要件

| 項目 | 要件 |
|------|------|
| パフォーマンス | CPU 使用率 < 1%、メモリ使用量 < 25MB |
| 信頼性 | トークン更新失敗時に再認証を自動開始 |
| セキュリティ | トークンはキーチェーン保存、設定ファイルパーミッション 600 |
| 拡張性 | 将来的に通知フィルタや Webhook 対応を可能に |
| 依存管理 | `Cargo.lock` 付き、再現可能なビルド |

---

## 6. アーキテクチャ設計

### 6.1 全体構成

+---------------------+
| Main Event Loop | ← Tokio ランタイム
+----------+----------+
|
+----------v----------+ +------------------+
| Auth Manager |<--->| GitHub REST API |
| (PAT Auth + | | (Token Validation) |
| Token Management) | +------------------+
+----------+----------+
|
+----------v----------+ +------------------+
| GitHub Notification |<--->| GitHub REST API |
| Poller Task | | (w/ Bearer Auth) |
+----------+----------+ +------------------+
|
+----------v----------+
| Desktop Notifier | → OS ネイティブ通知
+---------------------+

### 6.2 主要コンポーネント

| コンポーネント | 説明 |
|----------------|------|
| `ConfigLoader` | 設定ファイル読み込み |
| `AuthManager` | PAT 読み込み、トークン検証・管理、キーチェーン操作（オプション） |
| `GitHubClient` | 認証済み HTTP クライアント（アクセストークン自動付与） |
| `Poller` | 通知ポーリング + 新規通知検出 |
| `Notifier` | OS 依存の通知送信 |
| `StateManager` | ETag / 最終取得時刻管理 |
| `Logger` | `tracing` ベースのログ |

### 6.3 使用する主要クレート

| クレート | 用途 |
|--------|------|
| `tokio` | 非同期ランタイム |
| `reqwest` | HTTP クライアント |
| `serde` + `serde_json` | JSON 処理 |
| `tracing` + `tracing-subscriber` | ログ |
| `keyring` | OS キーチェーン統合（オプション） |
| `clap` | CLI（例: `--reauth`） |
| `notify-rust` / `winrt-notification` | 通知 |

---

## 8. 初回認証フロー（ユーザー体験）

1. ユーザーが `gh-notify-daemon` を初回起動
2. 以下がターミナルに表示される：
