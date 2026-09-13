# functus 開発ハーネス

圏論の概念でコード品質を担保するコード生成ツール群。全体設計は `docs/DESIGN.md` を参照。

詳細な規約は `.claude/rules/` 配下に分割してある(workspace-layout / coding-standards / tdd は
`paths` フロントマターにより該当ファイルを操作する時だけ読み込まれる。code-review-response /
git-notes はファイル種別に紐付かないため無条件読み込み)。`@import` はしない — `.claude/rules/`
は Claude Code が自動的に検出するため、CLAUDE.md から明示的に読み込む必要はない。

## バージョン管理: jj (Jujutsu)

このリポジトリは **jj の colocated リポジトリ**。git の履歴操作(`git commit` / `checkout` / `rebase` 等)は使わない(hook でブロックされる)。閲覧系の `git log` 等は可。

### マイクロコミット運用

1つの論理的変更 = 1つの jj change。作業の区切りごとに:

1. `jj diff` で変更を確認(複数の論理単位が混ざっていたら `jj split`)
2. Rust コードを含む変更では品質ゲートを通す: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
3. `/jj-commit` スキルに従って `jj desc` で description を記述
4. `jj new` で次の変更を開始

### Change Description の形式

Conventional Commits + 経緯説明(詳細は `/jj-commit` スキル):

```
<type>(<scope>): <日本語の要約>

## 経緯
<なぜ必要か。関連 Issue (#N)、設計判断、発生していた問題>

## 実装内容
<何をどう実装したか。採用アプローチと却下した代替案>
```

type: feat / fix / refactor / docs / test / build / chore
scope: core / frontend-openapi / dsl / gen / verify / cli / product-design 等

## Rust 品質基準

- `cargo fmt` + `cargo clippy --all-targets -- -D warnings` は常時クリーン(編集時に hook が自動検査)
- ライブラリコードで `unwrap()` / `expect()` 禁止。`Result` + `thiserror` を使う
- 不正状態を型で排除する(状態遷移は enum = 余積、網羅的 match を強制)
- 公開 API には `///` ドキュメントコメントと `# Errors` セクション
- 圏論的法則(結合律・単位律・関手法則)に関わるコードには法則テストを必ず付ける

## コードレビュー

コミット前・PR 前には `/jj-review` スキルで `rust-reviewer` SubAgent によるレビューを実行し、Critical / High の指摘を解消してから description を付ける。

## Issue 運用

- マイルストーン: Phase0(設計) → Phase1(Core) → Phase2(多言語) → Phase3(DSL/検証) → Phase4(CLI/エコシステム)
- 実装に着手する際は対応する Issue / Sub-Issue を確認し、description の「経緯」で `#番号` を参照する
