---
id: task-19
title: 通知データ永続化と永続的デスクトップ通知機能 - データストレージと通知永続化の設計と実装
status: Done
assignee:
  - developer
created_date: '2025-11-06 06:35'
updated_date: '2025-11-06 06:57'
labels:
  - feature
  - database
  - persistence
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
通知データを永続化するためのデータベーススキーマまたはファイル形式の設計と実装を行う。SQLiteまたはJSONファイルを使用する。さらに、通知が自動消去されない永続的デスクトップ通知機能も実装する。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SQLiteデータベースまたはJSONファイルによる通知データ保存スキーマが設計されている
- [ ] #2 通知データの保存・取得・更新・削除機能が実装されている
- [ ] #3 保存される通知データにはID、タイトル、本文、URL、受信日時、既読状態などが含まれる
- [ ] #4 データベースマイグレーション機能が実装されている（SQLite使用時）
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. データストレージの選択（SQLite or JSONファイル）
2. 通知データモデルの設計
3. 保存/取得/更新/削除機能の実装
4. データベーススキーマの作成（SQLite使用時）
5. データファイルの保存場所・フォーマットの決定
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装計画

### 1. ストレージ技術の選定
- SQLite: 構造化データ保存に適し、検索やフィルタリングに強い
- JSONファイル: 単純な保存に適し、可読性が高い

### 2. 通知データモデルの設計
- 通知の基本情報（ID, タイトル, 本文, URLなど）
- 状態情報（既読/未読, 確認日時など）
- 追跡情報（保存日時, 更新日時など）

### 3. APIの設計
- 保存API: 通知データを永続化
- 取得API: 永続化された通知を取得
- 更新API: 通知の状態を更新（既読にするなど）
- 削除API: 不要な通知データを削除

### 4. ディレクトリ構造
- src/storage.rs - データストレージ関連の実装
- src/models.rs - 永続化用のデータモデル
<!-- SECTION:NOTES:END -->
