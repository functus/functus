# functus 全体設計ドキュメント

> 圏論の概念を用いてコードの品質を担保するツールエコシステムの設計

## 1. ビジョン

「モデルが正しければ、生成されるコードは必然的に正しい (Correct-by-Construction)」を実現する。
圏論の構造(対象・射・関手・モナド・自然変換)を中間表現(IR)として採用し、
入力仕様(OpenAPI 等)から各言語のコードを **数学的法則を満たす形で** 自動生成する。

## 2. アーキテクチャ概要

```
入力フロントエンド          コア                      バックエンド
┌──────────────┐   ┌───────────────────┐   ┌─────────────────┐
│ OpenAPI v3.x │──▶│                   │──▶│ TypeScript/React │
├──────────────┤   │  圏論的 IR         │   ├─────────────────┤
│ functus DSL  │──▶│  (functus-core)    │──▶│ Go               │
├──────────────┤   │  + 法則検証        │   ├─────────────────┤
│ (将来) GraphQL│──▶│  (functus-verify) │──▶│ Rust             │
└──────────────┘   └───────────────────┘   └─────────────────┘
                            ▲
                       functus-cli
```

### クレート構成 (Cargo workspace)

| クレート | 役割 |
|---|---|
| `functus-core` | 圏論的 IR の定義。対象(型)・射(操作)・関手(言語への写像)・モナド(効果)・自然変換(表現間変換)と、その法則チェッカー |
| `functus-frontend-openapi` | OpenAPI v3.0/3.1 → IR 変換 (`openapiv3` クレート利用) |
| `functus-dsl` | 圏論モデルを直接記述する DSL のパーサー・型検査器 |
| `functus-gen` | `CodeGenerator` トレイトと各言語バックエンド (TS/React, Go, Rust) |
| `functus-verify` | 法則検証。生成コードに対するプロパティベーステスト生成 |
| `functus-cli` | `functus generate / check / verify / explain` |

## 3. 圏論的 IR の設計

### 3.1 対応関係

| 圏論の概念 | functus での意味 |
|---|---|
| 対象 (Object) | データ型・API スキーマ・UI 状態 |
| 射 (Morphism) | API 操作・状態遷移・純粋変換 |
| 恒等射・合成 | no-op / パイプライン合成 (結合律を IR レベルで検証) |
| 関手 (Functor) | IR → ターゲット言語型システムへの構造保存写像 |
| モナド (Monad) | 効果のモデル化: 非同期(Promise/Future)、失敗(Result/Either)、状態(useState) |
| 自然変換 | 言語間・表現間の一貫した変換 (例: TS の型 ⇔ Go の型) |
| 余積 (Coproduct) | タグ付きユニオン (`IDLE \| LOADING \| SUCCESS \| FAILURE`) |
| 積 (Product) | 構造体・レコード型 |

### 3.2 法則による品質担保

- **圏の法則**: 射の合成の結合律、恒等射の単位律 → IR 構築時に検証
- **関手法則**: `F(id) = id`, `F(g∘f) = F(g)∘F(f)` → バックエンドが構造を保存することの検証
- **モナド則**: 単位律・結合律 → 生成される非同期/エラー処理コードの正しさ
- 検証層は 3 段階: ① IR 構築時の静的検証 ② 生成時のスナップショット検証 ③ 生成コードへのプロパティベーステスト出力

### 3.3 状態遷移の定式化

React フック生成の核心は「API 呼び出し = 状態圏上の射」と見なすこと。
状態集合を余積 `Idle + Loading + Success(T) + Failure(E)` とし、
許可される遷移のみを射として定義 → 不正遷移はコンパイル時に排除される。

## 4. コンポーネント設計

### 4.1 functus-core (Phase 0–1)
- `Object`, `Morphism`, `Category` の Rust 型定義
- 効果アノテーション (`Effect::Async`, `Effect::Fallible`) をモナドスタックとして表現
- `LawChecker`: 合成可能性・網羅性・到達可能性の検証

### 4.2 フロントエンド (Phase 1)
- OpenAPI: `ReferenceOr<T>` の解決は初期段階では `Item` のみサポート (#2 の方針を踏襲)
- スキーマ → 対象、パス+メソッド → 射、レスポンス分岐 → 余積

### 4.3 コード生成 (Phase 1–2)
- `CodeGenerator` トレイト (#5 の設計を踏襲・拡張)
- 生成器は「関手の実装」であり、関手法則テストを義務付ける
- テンプレートエンジンは初期は `format!`、必要に応じて `askama`/`tera`

### 4.4 functus DSL (Phase 3)
- OpenAPI に依存しない圏論モデル直接記述言語
- 例:
  ```
  object User { id: Uuid, name: String }
  morphism getUser : UserId -> Async<Result<User, ApiError>>
  state UserFetch = Idle | Loading | Success(User) | Failure(ApiError)
  ```
- パーサー: `chumsky` または `nom`、型検査 + 圏論的整合性チェック

### 4.5 functus-verify (Phase 3)
- 生成コードと同時にプロパティベーステスト (TS: fast-check / Rust: proptest) を出力
- 状態遷移の網羅性 (すべての `status` 分岐が処理されるか) の検証

### 4.6 functus-cli (Phase 4)
- サブコマンド: `generate` / `check` (モデル検証のみ) / `verify` (テスト生成+実行) / `explain` (圏論構造の可視化)
- 設定ファイル `functus.toml`、watch モード

## 5. ロードマップ (マイルストーン)

| Phase | 内容 | 状態 |
|---|---|---|
| Phase 0 | 設計・アーキテクチャ確定 (本ドキュメント + 設計 Issue) | 本作業 |
| Phase 1 | Core: OpenAPI → IR → TS/React 生成の E2E | 既存 #1–#4 |
| Phase 2 | 多言語対応: CodeGenerator トレイト、Go/Rust バックエンド | 既存 #5 + 追加 |
| Phase 3 | DSL と法則検証 (functus-dsl / functus-verify) | 新規 |
| Phase 4 | CLI・ドキュメント・v0.1.0 リリース | 新規 |

## 6. 品質担保の原則 (まとめ)

1. **法則がテストである**: 関手法則・モナド則を CI で機械的に検証する
2. **型が仕様である**: 生成コードはタグ付きユニオン + 網羅的パターンマッチを強制する
3. **モデルが唯一の真実**: 手書きコードとの乖離は `functus check` で検出する
