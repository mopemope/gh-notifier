---
id: task-2
title: GitHub OAuth Device Flow API との通信機能を実装
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
GitHub の OAuth Device Flow エンドポイントと通信する機能を実装する。

主な機能:
- Device Code と User Code を取得するためのリクエスト
- Verification URI と User Code をユーザーに表示
- トークン取得のためのポーリング処理
- エラーハンドリング（認証タイムアウト、ユーザーによるキャンセルなど）
<!-- SECTION:DESCRIPTION:END -->
