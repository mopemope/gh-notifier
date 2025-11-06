---
id: task-21
title: 通知データ永続化機能 - 未読通知トラッキング機能の実装
status: Done
assignee:
  - developer
created_date: '2025-11-06 06:35'
updated_date: '2025-11-06 07:41'
labels:
  - feature
  - tracking
  - persistence
dependencies:
  - task-19
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
通知の既読/未読状態を永続化し、システム再起動後も状態が保持されるように実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 通知の既読/未読状態が永続的に保存されている
- [ ] #2 ユーザー操作で既読状態を更新できる
- [ ] #3 再起動後も既読状態が維持されている
- [ ] #4 既読状態の更新が高速に反映される
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 通知モデルに既読状態フィールドを追加
2. 既読状態を永続化するロジックを実装
3. 既読状態更新APIを実装
4. 通知表示時に既読状態を反映するロジックを実装
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. 通知モデルへの既読状態追加
- 既読/未読状態を表すフィールドを通知モデルに追加
- 状態更新日時を記録するフィールドも追加

### 2. 既読状態の永続化
- 既読状態変更時にDBまたはファイルに保存
- トランザクション的安全性を確保

### 3. 既読状態更新API
- 通知IDを指定して既読状態を更新するAPI
- 複数通知の一括更新機能も検討

### 4. 通知表示ロジック
- 既読状態に応じた表示方法の変更（UI）

### 5. ディレクトリ構造
- src/models.rs - 通知モデルの更新
- src/storage.rs - 既読状態管理の実装
<!-- SECTION:NOTES:END -->
