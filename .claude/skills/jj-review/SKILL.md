---
name: jj-review
description: 現在の jj 変更(または指定リビジョン)に対して rust-reviewer SubAgent による品質重視のコードレビューを実行する。
when_to_use: |
  コミット前・PR 作成前に使う。特に:
  - Rust コードを含む jj change の description を付ける直前 (/jj-commit の前段)
  - ユーザーが「レビューして」「rust-reviewer で見て」「品質チェックして」と言った時
allowed-tools:
  - Bash(jj diff:*)
  - Agent(rust-reviewer)
disallowed-tools:
  - AskUserQuestion
# model: sonnet — 深い正しさ判定は rust-reviewer (model: opus) に委譲するため、
#                  このスキル自身は diff 取得・Agent 起動・指摘に基づく修正の
#                  適用/報告という調整役に徹する。opus ほどのコストは不要
model: sonnet
effort: high
# context: 未設定 (インライン実行)。レビュー後の Critical/High 指摘の修正 (Edit) を
#          同一セッションで継続する必要があるため、会話履歴を持たない fork にはしない。
# paths: 未設定。変更されたファイルの種類を問わず、jj change 全体を対象にするため。
---

# jj 変更のコードレビュー

現在の変更を rust-reviewer SubAgent でレビューし、指摘を修正してから description を付ける。

## 手順

1. **レビュー対象の diff を取得**
   ```bash
   jj diff              # 現在の変更 (@)
   jj diff -r <rev>     # 引数でリビジョンが指定された場合
   ```
   変更が空の場合は「レビュー対象がない」と報告して終了する。

2. **rust-reviewer SubAgent を起動**
   Agent ツールで `rust-reviewer` を起動し、以下を伝える:
   - レビュー対象リビジョン(既定は `@`)
   - 変更の目的(対応中の Issue 番号・機能名)

3. **指摘への対応**
   - Critical / High の指摘: 必ず修正する
   - Medium / Low の指摘: 修正するか、見送る場合は理由をユーザーに報告する
   - 修正後、`cargo clippy --all-targets -- -D warnings && cargo test` が通ることを確認する

4. **レビュー結果の報告**
   指摘の要約(重要度別)と対応状況をユーザーに報告する。修正を行った場合は `/jj-commit` の手順で description に経緯を残す。
