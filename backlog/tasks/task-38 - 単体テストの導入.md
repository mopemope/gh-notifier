---
id: task-38
title: 単体テストの導入
status: To Do
assignee: []
created_date: '2025-11-13 14:23'
labels:
  - testing
  - quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
各モジュールごとの単体テストを導入する。GitHub API型のテスト、設定管理のテスト、通知ロジックのテスト、イベント処理のテストを作成する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/github/types.rsに対する単体テストが作成され、型のシリアライズ/デシリアライズがテストされている
- [ ] #2 src/config.rsに対する単体テストが作成され、設定の読み込みとバリデーションがテストされている
- [ ] #3 src/notification/manager.rsに対する単体テストが作成され、通知ロジックがテストされている
- [ ] #4 src/events/processor.rsに対する単体テストが作成され、イベント処理ロジックがテストされている
- [ ] #5 各テストがcargo testで正常に実行可能である
<!-- AC:END -->
