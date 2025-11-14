---
id: task-39
title: 統合テストの導入
status: To Do
assignee: []
created_date: '2025-11-13 14:24'
labels:
  - testing
  - integration
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
モジュール間の連携をテストする統合テストを導入する。外部API呼び出しのモック化、エンドツーエンドの通知フローのテストを作成する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/integration_test.rsが作成され、主要なアプリケーションフローがテストされている
- [ ] #2 GitHub API呼び出しのモックが実装され、外部依存なしでテストが実行可能である
- [ ] #3 通知のエンドツーエンドフローがテストされている
- [ ] #4 設定ファイルの読み込みと適用がテストされている
- [ ] #5 テストがCI環境でも正常に実行可能である
<!-- AC:END -->
