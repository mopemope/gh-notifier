---
id: task-7
title: 設定ファイル読み込み機能（ConfigLoader）の実装
status: Done
assignee: []
created_date: '2025-11-01 02:33'
updated_date: '2025-11-01 02:42'
labels:
  - config
  - implementation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`config.rs` を作成し、設定ファイル `~/.config/gh-notifier/config.toml` を読み込む機能を実装する。
- pat (省略可、GitHub Personal Access Token)
- poll_interval_sec (デフォルト: 30)
- mark_as_read_on_notify (デフォルト: false)
を読み込めるようにする。
<!-- SECTION:DESCRIPTION:END -->
