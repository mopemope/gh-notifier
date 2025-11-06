---
id: task-14
title: 永続的デスクトップ通知機能の実装 - 設定項目追加
status: To Do
assignee:
  - developer
created_date: '2025-11-06 06:32'
updated_date: '2025-11-06 06:33'
labels:
  - feature
  - configuration
  - notifications
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Config構造体にpersistent_notificationsフラグを追加する。デフォルト値はfalse（現行の自動消去挙動を維持）。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Config構造体にpersistent_notifications(bool)フィールドが追加されている
- [ ] #2 persistent_notificationsのデフォルト値がfalseになっている
- [ ] #3 設定ファイルからpersistent_notifications値を読み込める
- [ ] #4 Cargo.tomlの依存関係に変更がない
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/config.rsにpersistent_notifications: boolフィールドを追加
2. default()関数でデフォルト値をfalseに設定
3. serde属性でTOMLからの読み込みを有効化
4. Config構造体のSerialize, Deserializeを維持
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. Config構造体の変更
src/config.rsにpersistent_notificationsフィールドを追加します。

### 2. デフォルト値の設定
default()メソッドでpersistent_notificationsのデフォルト値をfalseに設定します。

### 3. TOMLファイルからの読み込み
serdeの属性を使用してTOMLファイルからの値読み込みを実現します。

### 4. ディレクトリ構造
- src/config.rs - Config構造体の定義
<!-- SECTION:NOTES:END -->
