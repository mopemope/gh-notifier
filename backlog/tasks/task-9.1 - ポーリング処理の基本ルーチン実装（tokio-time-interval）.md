---
id: task-9.1
title: 'ポーリング処理の基本ルーチン実装（tokio::time::interval）'
status: Done
assignee: []
created_date: '2025-11-01 02:37'
updated_date: '2025-11-01 08:11'
labels:
  - poller
  - implementation
dependencies: []
parent_task_id: task-9
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`poller.rs` に `tokio::time::interval` を使用して定期的に通知取得を行う基本ループを実装する。
設定ファイルから `poll_interval_sec` を読み込み、ポーリング間隔を設定する。
<!-- SECTION:DESCRIPTION:END -->
