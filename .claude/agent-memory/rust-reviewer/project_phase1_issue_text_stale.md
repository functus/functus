---
name: phase1-issue-text-stale
description: Phase1 の既存 Issue #1-#4 の本文は workspace 設計より前に書かれており DoD が現在の設計と食い違う。レビューで乖離を指摘しない
metadata:
  type: project
---

Phase1 の既存 Issue #1〜#4 の本文は、docs/DESIGN.md と .claude/rules/workspace-layout.md で
Cargo workspace 構成が確定する前に書かれている。そのため受け入れ条件が現在の設計と食い違う。

実例として Issue #1 は次を求めているが、いずれも現在の設計では満たせないか、別クレートの担当になる。

- `cargo new openapi-cq-generator` で単一クレートを作る。現在は crates/functus-* の workspace 構成である
- `cargo run` が通ること。バイナリは functus-cli のみに置く規約のため Phase1 序盤には binary target がない
- ルートの Cargo.toml に `openapiv3` を追加する。OpenAPI 依存は functus-frontend-openapi の担当である

**Why:** Issue 本文の記述を規約違反として指摘すると、設計上正しい実装に不要な修正を求めることになる。
逆に食い違いを黙って放置すると、Issue の未チェック項目が残ったまま close されて後から経緯が追えなくなる。

**How to apply:** Phase1 の Issue に対応する diff をレビューするとき、Issue 本文と設計文書が食い違う箇所は
コードの欠陥として扱わない。かわりに、change description か Issue コメントで
設計文書に置き換わった旨を残すよう `[ask]` で促す。設計文書 (docs/design/*.md と .claude/rules/*.md) を
正とする。関連: [[rust-review-conventions]]
