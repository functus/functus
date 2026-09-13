# 4. コンポーネント設計

[← 目次に戻る](../DESIGN.md)

## 4.1 functus-core (Phase 0–1)
- `Object`, `Morphism`, `Category` の Rust 型定義
- 効果アノテーション (`Effect::Async`, `Effect::Fallible`) をモナドスタックとして表現
- `LawChecker`: 合成可能性・網羅性・到達可能性の検証

## 4.2 フロントエンド (Phase 1)
- OpenAPI: `ReferenceOr<T>` の解決は初期段階では `Item` のみサポート (#2 の方針を踏襲)
- スキーマ → 対象、パス+メソッド → 射、レスポンス分岐 → 余積

## 4.3 コード生成 (Phase 1–2)
- `CodeGenerator` トレイト (#5 の設計を踏襲・拡張)
- 生成器は「関手の実装」であり、関手法則テストを義務付ける
- テンプレートエンジンは初期は `format!`、必要に応じて `askama`/`tera`

## 4.4 functus DSL (Phase 3)
- OpenAPI に依存しない圏論モデル直接記述言語
- 例:
  ```
  object User { id: Uuid, name: String }
  morphism getUser : UserId -> Async<Result<User, ApiError>>
  state UserFetch = Idle | Loading | Success(User) | Failure(ApiError)
  ```
- パーサー: `chumsky` または `nom`、型検査 + 圏論的整合性チェック

## 4.5 functus-verify (Phase 3)
- 生成コードと同時にプロパティベーステスト (TS: fast-check / Rust: proptest) を出力
- 状態遷移の網羅性 (すべての `status` 分岐が処理されるか) の検証

## 4.6 functus-cli (Phase 4)
- サブコマンド: `generate` / `check` (モデル検証のみ) / `verify` (テスト生成+実行) / `explain` (圏論構造の可視化)
- 設定ファイル `functus.toml`、watch モード
