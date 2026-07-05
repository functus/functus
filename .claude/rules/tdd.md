# テスト駆動開発 (TDD) — 必須

このリポジトリでは **Red → Green → Refactor** のサイクルを強制する。hook がこれを機械的に検査する。

## サイクル

1. **Red**: 実装より先に、期待する振る舞いを表す失敗するテストを書く
   - `cargo test <対象>` を実行し、**意図した理由で失敗する**ことを確認してから実装に進む
2. **Green**: テストを通す最小限の実装を書く
   - `cargo test` が通ることを確認する
3. **Refactor**: テストが通る状態を維持したまま設計を改善する
4. `/jj-commit` でマイクロコミット(1サイクル completing = 1 change が理想)

## Hook による強制

- **Stop 時 (tdd-guard)**: 作業コピー (@) で `src/` の Rust コードが変更されているのに
  テストの追加・変更が一切ない場合、停止がブロックされる
- **Stop 時**: `cargo test` が失敗している(Red のまま)場合も停止がブロックされる。
  Green まで完了させてから停止すること

## 例外 (escape hatch)

テストが原理的に不要な変更(再エクスポートのみ、doc コメントのみ、derive 追加のみ等)は、
change description に `[skip-tdd]` を含めることで tdd-guard をスキップできる。
ただし乱用しない — レビューで `[skip-tdd]` の妥当性を必ず確認する。

## テストの置き場所

- 単体テスト: 同ファイル内の `#[cfg(test)] mod tests`
- 法則テスト(proptest): `crates/<crate>/tests/laws.rs`
- 統合テスト: `crates/<crate>/tests/`
- E2E(生成コードのコンパイル検証): `examples/` + CI
