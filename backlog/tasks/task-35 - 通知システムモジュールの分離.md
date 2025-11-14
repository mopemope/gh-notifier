---
id: task-35
title: 通知システムモジュールの分離
status: To Do
assignee: []
created_date: '2025-11-13 14:23'
labels:
  - refactoring
  - notification
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
通知の管理と配信を担当するモジュールを独立させる。通知の作成、フィルタリング、配信ロジックをnotification/配下に実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/notification/manager.rs が作成され、通知管理ロジックが実装されている
- [ ] #2 通知のフィルタリング機能が実装されている
- [ ] #3 システム通知への配信ロジックが実装されている
- [ ] #4 通知アクション（クリック時の処理等）が実装されている
- [ ] #5 src/notification/mod.rs が作成され、モジュールの公開インターフェースが定義されている
<!-- AC:END -->
