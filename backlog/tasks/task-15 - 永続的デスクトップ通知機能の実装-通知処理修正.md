---
id: task-15
title: 永続的デスクトップ通知機能の実装 - 通知処理修正
status: To Do
assignee:
  - developer
created_date: '2025-11-06 06:32'
updated_date: '2025-11-06 06:34'
labels:
  - feature
  - notifications
  - cross-platform
dependencies:
  - task-14
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
DesktopNotifierの実装を修正し、persistent_notifications設定に応じて通知の永続性を制御する。Linuxのnotify-rust、macOS、Windowsそれぞれの実装を修正。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DesktopNotifierがpersistent_notifications設定を参照して通知の永続性を制御している
- [ ] #2 Linux (notify-rust)でTransientヒントが正しく設定されている
- [ ] #3 macOS (mac-notification-sys)で永続性が制御できる実装になっている
- [ ] #4 Windows (winrt-notification)で永続性が制御できる実装になっている
- [ ] #5 既存の通知機能が破壊されていない
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Poller構造体を更新してConfigへの参照を保持
2. DesktopNotifierのsend_notificationメソッドを修正しpersistent_notifications設定を反映
3. macOS, WindowsのNotifier実装も同様に修正
4. 各プラットフォームで通知永続性が正しく制御されるように実装
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. Notifier traitの更新
Notifier::send_notificationメソッドは既にConfigを直接受け取らないので、引数を更新するのではなく、Poller構造体を介してConfigインスタンスをNotifier実装に渡す必要があります。

### 2. Poller構造体の更新
PollerにConfigへの参照を保持させ、Notifier実装に渡すようにします。

### 3. 各プラットフォーム固有の実装
- Linux (notify-rust): Hint::Transientをfalseに設定
- macOS (mac-notification-sys): 永続性を制御するオプションを確認・実装
- Windows (winrt-notification): 永続性を制御するオプションを確認・実装

### 4. ディレクトリ構造
- src/poller.rs - Notifier実装の更新
- 既存の通知ロジックを修正
<!-- SECTION:NOTES:END -->
