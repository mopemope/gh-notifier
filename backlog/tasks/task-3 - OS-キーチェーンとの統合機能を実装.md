---
id: task-3
title: OS キーチェーンとの統合機能を実装
status: To Do
assignee: []
created_date: '2025-10-31 11:23'
labels:
  - auth
  - security
  - implementation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
keyring クレートを使用して、OS のキーチェーンにトークンを安全に保存・読み込みする機能を実装する。

対応プラットフォーム:
- macOS: Keychain
- Windows: Credential Vault
- Linux: libsecret（GNOME Keyring など）

主な機能:
- アクセストークン・リフレッシュトークンの保存
- トークンの読み込み
- トークン削除（再認証時など）
<!-- SECTION:DESCRIPTION:END -->
