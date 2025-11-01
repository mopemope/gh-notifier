---
id: task-13
title: メインイベントループと終了処理（Main Event Loop）の実装
status: Done
assignee: []
created_date: '2025-11-01 02:34'
updated_date: '2025-11-01 09:07'
labels:
  - main
  - implementation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
main.rs を更新し、tokioの非同期ループでPollerを起動し、SIGINT/SIGTERMをキャッチして安全に終了する機能を実装する。
終了前に状態を保存する処理も含める。
<!-- SECTION:DESCRIPTION:END -->
