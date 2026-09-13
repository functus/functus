# functus-frontend-openapi

OpenAPI v3.0 の YAML を `functus_core::Category` に変換するクレート。

## 依存方向

`.claude/rules/workspace-layout.md` に従い `functus-core` にのみ依存する。`functus-gen` や
`functus-dsl` など他のフロントエンド・バックエンドには依存しない。

## Phase1 のスコープ決定

- **OpenAPI v3.1 は非対応。** `openapiv3` クレート自体が v3.0.x 専用で、v3.1(JSON Schema
  2020-12 ベース、`nullable` 廃止等)は読めない。`docs/design/02-architecture.md` の
  「OpenAPI v3.0/3.1 → IR 変換 (`openapiv3` クレート利用)」という記述はこの制約を反映
  できておらず、設計文書側の既知の課題として扱う。v3.1 対応が必要になったら `oas3` 等
  別クレードの併用を検討する
- **YAML パーサーは `serde_yaml` ではなく `serde_norway` を使う。** `serde_yaml` は
  deprecated(上流アーカイブ済み)なため、保守されているフォークを採用した
- **`$ref` は `#/components/schemas/<name>` の形のみ解決する。** これは Issue #2 が
  「ファーストフェーズとしては Item のみをサポートする割り切りを行う」と書いている
  ことの解釈だが、レスポンス・プロパティのスキーマは実務上ほぼ必ず名前付きスキーマへの
  `$ref` になるため、完全に無視すると Issue #2 のゴール(型とルートの抽出)自体が
  達成できない。外部ファイル参照・`responses`/`parameters` コンポーネントへの参照は未対応
- **スキーマはスカラー(string/integer/number/boolean)プロパティを持つ `type: object` の
  トップレベル定義のみサポートする。** ネストしたインラインオブジェクト・配列・
  `oneOf`/`anyOf`/`allOf`/`not` は Phase1 では扱わない
- **「独自の内部モデル(`ApiSchema`/`ApiRoute`)」は作らない。** Issue #2 の本文は
  functus-core の圏論的 IR(#14)が確定する前に書かれている。スキーマ→対象・
  パス+メソッド→射という docs/design/04-components.md 4.2 の対応表に従い、
  `functus_core::Category` を直接組み立てる
- **レスポンスの効果マッピング。** 2xx レスポンスのスキーマを `cod` にし、常に
  `Effect::Async` を付与する(API呼び出しは必ず非同期)。2xx 以外でスキーマ付きの
  レスポンスが最初に見つかったものを `Effect::Fallible` のエラー型にする
- **path parameter が複数ある operation は `<operationId>Params` という名前の
  合成 Product 対象を作る。** 0個なら `Unit` スカラー、1個ならそのスカラーを
  そのまま domain にする
- **`components.schemas` は宣言順に依存しない。** 前方参照(先に宣言したスキーマが
  後で宣言されるスキーマを `$ref` する)を許すため、`schema.rs::register_named_schemas`
  は「依存先が揃うまで後回しにする」を繰り返し、全体が進まなくなった時点で
  循環参照または未定義参照として拒否する
- **path item レベルと operation レベルの parameter は `(name, in)` でマージする。**
  OpenAPI の仕様上 operation レベルが同名の path item レベルを上書きできるため、
  単純な連結ではなく `route.rs::merge_parameters` で重複排除し、後勝ち(operation優先)
  で解決する
