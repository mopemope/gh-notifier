---
id: task-9
title: 通知ポーリング処理（Poller）の実装
status: Done
assignee: []
created_date: '2025-11-01 02:33'
updated_date: '2025-11-01 05:35'
labels:
  - poller
  - implementation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`poller.rs` を作成し、tokioのintervalを使用して定期的にGitHub APIから通知を取得し、新規通知を検出する機能を実装する。
- 設定ファイルのpoll_interval_secに従う
- StateManagerから最終確認日時を取得し、それ以降の通知を取得
- 通知をNotifierに渡す
- mark_as_read_on_notifyオプションに対応
<!-- SECTION:DESCRIPTION:END -->
