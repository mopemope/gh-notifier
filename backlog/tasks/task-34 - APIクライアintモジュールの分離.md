---
id: task-34
title: APIクライアintモジュールの分離
status: To Do
assignee: []
created_date: '2025-11-13 14:22'
labels:
  - refactoring
  - github-api
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub APIとの通信を担当するクライアintモジュールを独立させる。HTTPクライアintの設定、認証、API呼び出しロジックをgithub/client.rsに実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/github/client.rs が作成され、GitHub APIへのHTTP通信ロジックが実装されている
- [ ] #2 認証トークンの管理と設定が実装されている
- [ ] #3 主要なGitHub APIエンドポイント（notifications, repositories等）の呼び出しが実装されている
- [ ] #4 APIエラーハンドリングが適切に実装されている
- [ ] #5 src/github/mod.rs が作成され、モジュールの公開インターフェースが定義されている
<!-- AC:END -->
