---
id: task-20
title: 通知データ永続化機能 - 通知履歴管理システムの実装
status: Done
assignee:
  - developer
created_date: '2025-11-06 06:35'
updated_date: '2025-11-06 07:15'
labels:
  - feature
  - history
  - persistence
dependencies:
  - task-19
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
受信した通知を永続的に保存し、再起動後も確認可能な履歴管理システムを実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 通知が受信されたときに永続ストレージに保存される
- [ ] #2 重複通知が保存されないよう制御されている
- [ ] #3 過去の通知履歴を取得できるAPIが実装されている
- [ ] #4 特定条件での通知フィルタリングが可能である
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 通知受信時に永続ストレージに保存するロジックを実装
2. 重複防止ロジックの実装
3. 通知履歴取得APIの実装
4. フィルタリング機能の実装
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. 通知保存ロジックの実装
- 通知受信時に永続ストレージに保存する処理を追加
- 保存前に重複チェックを実施

### 2. 重複防止ロジック
- 通知IDを使用した重複検出
- タイムスタンプベースの重複防止

### 3. 履歴取得機能
- 全通知履歴の取得
- 未読通知のみの取得
- 時期指定での取得

### 4. フィルタリング機能
- リポジトリ名、通知理由、通知タイプなどでフィルタリング
- 検索機能の実装

### 5. ディレクトリ構造
- src/history_manager.rs - 通知履歴管理の実装
- src/polling/handler.rs - 通知受信時の保存ロジック
<!-- SECTION:NOTES:END -->
