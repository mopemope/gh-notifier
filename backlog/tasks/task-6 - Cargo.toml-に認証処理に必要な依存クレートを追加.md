---
id: task-6
title: Cargo.toml に認証処理に必要な依存クレートを追加
status: Done
assignee: []
created_date: '2025-10-31 11:24'
updated_date: '2025-11-01 02:01'
labels:
  - dependencies
  - setup
  - configuration
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
認証処理に必要な依存クレートを Cargo.toml に追加する。

必要なクレート:
- reqwest: HTTP クライアント
- serde: JSON シリアライズ
- serde_json: JSON パース
- keyring: OS キーチェーン統合
- tokio: 非同期処理
- url: URL 操作
- base64: 認証フロー補助
<!-- SECTION:DESCRIPTION:END -->
