---
id: task-10
title: デスクトップ通知機能（Notifier）の実装
status: To Do
assignee: []
created_date: '2025-11-01 02:34'
labels:
  - notifier
  - implementation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`notifier.rs` を作成し、OSに応じたデスクトップ通知機能を実装する。
- Linux: notify-rust
- macOS: mac-notification-sys
- Windows: winrt-notification
通知クリックでブラウザで通知ページを開く機能も実装する。
<!-- SECTION:DESCRIPTION:END -->
