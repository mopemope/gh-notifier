---
id: task-2
title: GitHub PAT検証APIとの通信機能を実装
status: Done
assignee: []
created_date: '2025-10-31 11:23'
updated_date: '2025-10-31 12:42'
labels:
  - auth
  - api
  - implementation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub の REST API と通信してPATの有効性を検証する機能を実装する。

主な機能:
- PAT が有効かどうかを検証するためのリクエスト
- GitHub API への認証付きリクエスト送信
- トークンの有効性確認処理
- エラーハンドリング（無効なトークン、権限不足、ネットワークエラーなど）
<!-- SECTION:DESCRIPTION:END -->
