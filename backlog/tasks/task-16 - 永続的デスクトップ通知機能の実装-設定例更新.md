---
id: task-16
title: 永続的デスクトップ通知機能の実装 - 設定例更新
status: To Do
assignee:
  - developer
created_date: '2025-11-06 06:32'
updated_date: '2025-11-06 06:34'
labels:
  - documentation
  - configuration
dependencies:
  - task-14
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
config_example.tomlにpersistent_notifications設定オプションを追加し、使用方法を説明する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 config_example.tomlにpersistent_notificationsオプションが追加されている
- [ ] #2 persistent_notificationsの使用例と説明が記載されている
- [ ] #3 trueに設定すると通知が永続的になることが説明されている
- [ ] #4 false（デフォルト）の場合は現行通り自動消去されることを説明
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. config_example.tomlにpersistent_notificationsオプションを追加
2. 設定値の説明コメントを記載
3. true/falseの効果について説明を追加
4. 他の設定例と同様のフォーマットで配置
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. config_example.tomlの更新
persistent_notifications = false を例として追加（デフォルト動作を示すため）

### 2. コメントの追加
設定オプションの意味と効果について説明を追加

### 3. 使用例の記述
trueに設定した場合とfalseに設定した場合の動作の違いを説明

### 4. 位置
config_example.tomlの基本的な設定オプションセクション付近に追加
<!-- SECTION:NOTES:END -->
