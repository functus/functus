---
name: project-issue-text-predates-ir
description: Phase1 の Issue 本文(#2 等)は圏論的IR確定(#14)より前に書かれており、独自内部モデル前提の記述は設計乖離として扱わない
metadata:
  type: project
---

Phase0〜Phase1 前半に立てた Issue の本文は、functus-core の圏論的 IR
(`Category` / `Object` / `Morphism`、#14) が確定する前に書かれている。そのため
「独自の内部モデル(`ApiSchema` / `ApiRoute`)を作る」のような、IR を経由しない
前提の記述が残っている。実装が Issue 本文どおりでないこと自体は指摘対象にしない。

**Why:** #2 (OpenAPI パース) の実装は `docs/design/04-components.md` 4.2 の対応表
(スキーマ→対象、パス+メソッド→射)に従い `functus_core::Category` を直接組み立てる
設計を採った。これは Issue 本文より後の設計判断であり、`workspace-layout.md` の
依存方向(frontend → core のみ)にも合致する。Issue 本文を根拠に「独自モデルが無い」
と指摘すると、確定済みの設計を巻き戻す方向に働く。

**How to apply:** Issue 本文と実装の乖離を見つけたら、まず `docs/DESIGN.md` /
`docs/design/` と該当クレートの `CLAUDE.md` を正とする。乖離が
クレート `CLAUDE.md` の「スコープ決定」に明記されていれば `[ask]` 以下に留め、
明記されていない場合のみ「設計判断が記録されていない」ことを指摘する
(乖離そのものではなく、記録の欠落が指摘対象)。関連: [[review-scope-doc-consistency]]
