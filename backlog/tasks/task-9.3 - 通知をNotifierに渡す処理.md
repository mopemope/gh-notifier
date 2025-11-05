---
id: task-9.3
title: 通知をNotifierに渡す処理
status: Done
assignee: []
created_date: '2025-11-01 02:38'
labels:
  - poller
  - notifier
dependencies: []
parent_task_id: task-9
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
新規通知を検出した際、その通知データをNotifierに渡してデスクトップ通知を表示する処理を実装する。

## 拡張内容 (2025-11-05)
通知の内容をより具体的で分かりやすいものに改善しました。

### 改善点:
1. タイトルフォーマットの改善
   - 以前: `{repository.full_name} / {subject.kind}` (例: "user/repo / PullRequest")
   - 以後: `{repository.full_name} - {reason_display_text}` (例: "user/repo - _Review Requested_")

2. ボディ内容の充実
   - 以前: 通知タイトルのみ
   - 以後: 通知タイトル + リポジトリ名 + 通知種別 (例: "Fix bug in authentication\nRepository: myrepo\nType: Pull Request")

3. 理由表示の改善
   - 各種通知理由("mention", "review_requested", "assign"など)に分かりやすい日本語表示をマッピング
   - 例: "review_requested" → "_Review Requested_", "mention" → "mentioned you"

4. 通知種別表示の改善  
   - 技術的な表記から分かりやすい表記に変更
   - 例: "PullRequest" → "Pull Request", "Issue" → "Issue"

これらの改善により、ユーザーは通知を見ただけですばやく何の通知かを理解できるようになりました。
<!-- SECTION:DESCRIPTION:END -->
