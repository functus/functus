---
name: jj-commit
description: jj (Jujutsu) によるマイクロコミットを行う。現在の変更に Conventional Commits 形式 + 実装経緯を含む description を付け、jj new で次の変更を開始する。
when_to_use: |
  作業の区切りごと、1つの論理的変更が完成した時に使う。特に:
  - TDD の Refactor が完了し cargo test が Green になった直後
  - ユーザーが「コミットして」「jj でまとめて」「区切って」「description つけて」と言った時
  - /jj-review でのレビュー対応が完了した直後
allowed-tools:
  - Bash(jj st:*)
  - Bash(jj diff:*)
  - Bash(jj split:*)
  - Bash(jj desc:*)
  - Bash(jj describe:*)
  - Bash(jj new:*)
  - Bash(cargo fmt:*)
  - Bash(cargo clippy:*)
  - Bash(cargo test:*)
disallowed-tools:
  - AskUserQuestion
model: inherit
effort: medium
# context: 未設定 (インライン実行)。直前のターンで何を実装したかという会話履歴を
#          参照して「経緯」を書く必要があるため、履歴を持たない fork サブエージェントにはしない。
# paths: 未設定。特定のファイル種別に紐付かない全リポジトリ横断のワークフローのため、
#        Glob によるファイル種別限定は行わない(常にどの変更にも適用されうる)。
---

# jj マイクロコミット

このリポジトリは git ではなく **jj (Jujutsu)** で履歴管理する。1つの論理的変更 = 1つの jj change を徹底する(マイクロコミット)。

## 手順

1. **変更内容の確認**
   ```bash
   jj st
   jj diff
   ```
   変更が複数の論理単位を含む場合は、`jj split` で分割してから各変更に description を付けること。

2. **品質ゲート**(Rust コードを含む変更のみ)
   ```bash
   cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
   ```
   失敗したら修正してから次へ進む。

3. **description の記述**
   ```bash
   jj desc -m "$(cat <<'EOF'
   <type>(<scope>): <要約(50字以内、命令形)>

   ## 経緯
   <なぜこの変更が必要になったか。背景となる Issue・設計判断・発生していた問題>

   ## 実装内容
   <何をどう実装したか。採用したアプローチと、検討して却下した代替案があればその理由>
   EOF
   )"
   ```

4. **次の変更を開始**
   ```bash
   jj new
   ```

## Conventional Commits の type

| type | 用途 |
|---|---|
| `feat` | 新機能(IR、ジェネレーター、CLI 等の機能追加) |
| `fix` | バグ修正 |
| `refactor` | 挙動を変えない構造改善 |
| `docs` | ドキュメントのみの変更 |
| `test` | テストの追加・修正 |
| `build` | Cargo.toml・依存関係・CI |
| `chore` | 上記以外の雑務 |

## scope の例

`core` / `frontend-openapi` / `dsl` / `gen` / `verify` / `cli` / `product-design` など、クレート名・領域名を使う。

## ルール

- 要約は日本語で書く(既存履歴の慣例に従う: 例 `docs(product-design): 全体的な構成についてドキュメント作成`)
- 「経緯」セクションは必須。関連 Issue があれば `#番号` で参照する
- 変更が空(`jj st` で no changes)なら何もしない
- `git commit` は使用禁止(hook でブロックされる)
