---
id: task-23
title: 通知データ永続化機能 - システム再起動時の通知回復機能の実装
status: To Do
assignee:
  - developer
created_date: '2025-11-06 06:35'
updated_date: '2025-11-06 06:36'
labels:
  - feature
  - recovery
  - notifications
dependencies:
  - task-19
  - task-20
  - task-21
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
システム再起動時に未読通知を検出し、適切にデスクトップ通知として再表示する機能を実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 アプリケーション起動時に未読通知が検出される
- [ ] #2 未読通知がデスクトップ通知として表示される
- [ ] #3 再起動直後の重複通知が表示されない
- [ ] #4 システム再起動時の通知状態が正しく復元される
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. アプリ起動時の未読通知チェック機能を実装
2. 未読通知のデスクトップ通知再表示機能を実装
3. 重複防止ロジックを実装
4. 再起動復元処理をアプリ初期化時に追加
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. 起動時の未読通知検出
- アプリケーション起動時に未読通知をDBから取得
- 重要な通知（PRレビューなど）を優先的に表示

### 2. 通知再表示機能
- 未読通知をデスクトップ通知として再表示
- 通知の重要度に応じた表示方法を検討

### 3. 重複防止ロジック
- 最後に確認した時刻以降に保存された通知のみ再通知
- 一定期間より古い未読通知は再通知しない（設定可能）

### 4. 初期化処理の更新
- アプリケーション初期化時に通知復元処理を追加

### 5. ディレクトリ構造
- src/initialization_service.rs - 再起動時の初期化処理
- src/app.rs - アプリケーション起動時の処理
<!-- SECTION:NOTES:END -->
