---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust コーディング規約

## エラーハンドリング

- ライブラリコード(cli 以外)で `unwrap()` / `expect()` / `panic!` 禁止。`Result` で返す
- エラー型はクレートごとに `thiserror` で定義し、呼び出し側が `match` で分岐可能にする
- `Box<dyn Error>` / `anyhow` はライブラリの公開 API では使わない(cli の main 層のみ `anyhow` 可)
- 診断は「どこが・なぜ不正か」を位置情報(span)付きで説明する

## 型設計

- 不正状態を型で排除する: 状態は enum(余積)で表現し、`match` は網羅的に書く(`_` ワイルドカードで潰さない)
- newtype を積極的に使う(`UserId(Uuid)` 等)。プリミティブの裸渡しを避ける
- `bool` 引数を2つ以上取る関数は enum に置き換える
- 公開型には `#[non_exhaustive]` の要否を検討する

## ドキュメント

- 公開 API には `///` ドキュメントコメント必須。`Result` を返すものは `# Errors` セクション必須
- 圏論の概念に対応する型・関数には、対応する数学的構造をコメントで明記する。例:
  ```rust
  /// 射の合成。結合律 compose(compose(f,g),h) == compose(f,compose(g,h)) を満たす。
  ```

## テスト

- 圏論的法則(結合律・単位律・関手法則・モナド則)に関わるコードには法則テスト必須(proptest 推奨)
- テストは実装の詳細ではなく性質(プロパティ)を検証する
- スナップショットテスト(insta)はコード生成の回帰検証に使う

## スタイル

- `cargo fmt` + `cargo clippy --all-targets -- -D warnings` 常時クリーン(hook で自動検査)
- `mod.rs` は使わず `foo.rs` + `foo/` 形式を使う
- import は std / 外部クレート / workspace 内 / self の順にグループ化する
- コメント・ドキュメントは日本語可。識別子は英語

## 依存関係

- 新しい外部クレートの追加は workspace ルートの `[workspace.dependencies]` で一元管理する
- 追加前に既存依存で代替できないか確認する。重い依存(tokio 等)は本当に必要なクレートのみに付ける
