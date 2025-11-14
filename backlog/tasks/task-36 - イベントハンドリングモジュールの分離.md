---
id: task-36
title: イベントハンドリングモジュールの分離
status: To Do
assignee: []
created_date: '2025-11-13 14:23'
labels:
  - refactoring
  - events
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHubイベントの処理と通知生成ロジックを担当するモジュールを独立させる。Webhookイベントの解析、通知への変換、イベントフィルタリングをevents/配下に実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/events/processor.rs が作成され、イベント処理ロジックが実装されている
- [ ] #2 GitHub Webhookイベントの解析ロジックが実装されている
- [ ] #3 イベントから通知への変換ロジックが実装されている
- [ ] #4 イベントフィルタリング機能が実装されている
- [ ] #5 src/events/mod.rs が作成され、モジュールの公開インターフェースが定義されている
<!-- AC:END -->
