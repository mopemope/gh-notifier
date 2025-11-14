---
id: task-33
title: モジュール構造の基礎作成
status: To Do
assignee: []
created_date: '2025-11-13 14:22'
labels:
  - refactoring
  - architecture
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
全体のモジュール構造を設計し、基本的なファイル構成を作成する。GitHub APIの厳密な型定義、通知関連の型定義、設定管理モジュールを作成する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/github/types.rs が作成され、GitHub APIの主要な型が定義されている
- [ ] #2 src/notification/types.rs が作成され、通知関連の型が定義されている
- [ ] #3 src/config.rs が作成され、階層化された設定構造が実装されている
- [ ] #4 各型に適切なserdeシリアライズ/デシリアライズが定義されている
- [ ] #5 必要なderive属性が付与されている（Debug, Clone, Serialize, Deserialize）
<!-- AC:END -->
