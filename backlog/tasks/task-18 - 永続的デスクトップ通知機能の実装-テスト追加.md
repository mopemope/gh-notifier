---
id: task-18
title: 永続的デスクトップ通知機能の実装 - テスト追加
status: To Do
assignee:
  - developer
created_date: '2025-11-06 06:33'
updated_date: '2025-11-06 06:34'
labels:
  - testing
  - feature
dependencies:
  - task-14
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
persistent_notifications設定のためのユニットテストと統合テストを追加する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 persistent_notificationsの有効/無効時の通知動作を検証するテストが追加されている
- [ ] #2 各プラットフォームでの通知永続性を検証するユニットテストがある
- [ ] #3 設定値の読み込みと適用を検証するテストが実装されている
- [ ] #4 既存テストがすべて通る状態を維持している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. persistent_notifications設定のユニットテストを追加
2. 設定値に基づく通知動作を検証するテストを実装
3. 各プラットフォーム向けのテストコードを追加
4. 既存テストが破壊されていないことを確認
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. ユニットテストの追加
persistent_notifications設定が通知の永続性に影響することを検証するテストを追加

### 2. モックNotifierの作成
通知の永続性を検証するためのモックNotifierを実装

### 3. 設定読み込みテスト
persistent_notificationsの設定値が正しく読み込まれることを検証

### 4. 各プラットフォーム用テスト
Linux/macOS/WindowsそれぞれのNotifier実装について、永続性設定が正しく反映されることを検証
<!-- SECTION:NOTES:END -->
