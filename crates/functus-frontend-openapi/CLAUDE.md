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
- **`required` は読まない。** すべてのプロパティ・parameter・`requestBody` を
  必須として扱う(`requestBody.required: false` や省略時の既定値 `false` も含む)。
  `ObjectKind` に任意フィールドの表現が無い Phase1 では、実装で解くよりまず
  この制約を明示するほうが安全だと判断した。生成コードの型が実 API より
  厳しくなる(本来 optional なフィールドが必須になる)方向にしか外れない
- **`format` は `uuid` のみ区別し、それ以外(`date`/`date-time`/`password`/`byte`/
  `binary`)はすべて `String` に収束させる。** 専用の対象を作ると組み合わせが
  爆発するため、Phase1 は日時等を独自型として扱わない
- **`requestBody` は domain の一部として扱う。** `application/json` 系スキーマを
  `body` という名前の要素として、path/query parameter と同じ「domain を構成する
  要素」のリストに加える。要素数の 0/1/2+ の分岐ロジックは parameter のみの
  場合と共有する
- **合成 Product のフィールド名が衝突する場合は由来(`in`)で一意化する。**
  `id` (path) と `id` (query) のように name だけが同じ parameter が両方
  存在する場合、`id_path` / `id_query` のようにリネームする。衝突していない
  名前はそのまま使う(不要な接尾辞を付けない)
- **レスポンスの content type は `application/json` または `+json` サフィックスを
  許容する。** `application/problem+json` (RFC 7807) は実務で頻出するため、
  完全一致ではなく essence(`;` より前)の末尾一致で判定する
- **2xx は明示コードがレンジより優先され、本文の無い成功レスポンスは `Unit` に
  なる。** `openapiv3::Responses` の仕様どおり、`200` のような明示コードが
  `2XX` のようなレンジ定義より優先される。`204` 等スキーマの無い 2xx は
  エラーにせず `Unit` を `cod` にする
- **非2xx(`responses.default` を含む)が複数の異なるスキーマを持つ場合は拒否する。**
  `EffectStack` の `Fallible` 層は1つしか持てないため、一意に決められない
  構成は `MultipleErrorResponsesUnsupported` で明示的に拒否する。将来
  複数のエラー型を余積(`Object::coproduct`)として統合する拡張の余地はあるが、
  Phase1 のスコープ外とする
- **スカラー名(`String`/`Uuid`/`Integer`/`Number`/`Boolean`/`Unit`)は予約語。**
  `components.schemas` に同名の対象を定義すると `ScalarNameCollision` で拒否する。
  黙って型を取り違えたり、意味不明な `DuplicateObject` になったりするのを防ぐ
- **`openapi` フィールドの値を検証する。** `3.0` で始まらない場合は
  `UnsupportedOpenApiVersion` で拒否する。`openapiv3` クレートは構文的には
  v3.1 の文書もそれなりに読めてしまうが、`nullable` の扱い等が意味的に異なるため
  黙って処理を続けさせない
- **明示コードの2xxレスポンスが複数の異なるスキーマを持つ場合は拒否する。**
  `200: User` と `201: CreatedUser` のように成功時の型が分岐しうる操作は、
  `cod` を一意に決められないため `MultipleSuccessResponsesUnsupported` で拒否する。
  同じスキーマを指している場合は1つにまとめる(非2xxの扱いと対称)。
  本文の無いレスポンス(`204` 等)も `Unit` という1つの型として同じ曖昧性判定に
  含める。`200: User` と本文無しの `204` が両方ある操作は、呼び出し側が
  受け取る型が状況によって変わるため、`User` を無条件に採用せず拒否する
- **「明示コードが2xxを優先する」判定は2xxの明示コードにのみ適用する。**
  `2XX`(レンジ)と `404`(非2xx の明示コード)が両方ある操作で、`404` の存在を
  理由にレンジの採用を妨げてはならない。判定には `StatusCode::Code(200..=299)`
  のみを数える
- **明示コードとレンジ定義は「どちらか一方を無視する」のではなく両方を候補にする。**
  `2XX` は `200` のような明示コードがあっても、それ以外の2xx(`201` 等)には
  引き続き適用されうる。異なるスキーマを指す場合は `cod` を一意に決められないため
  拒否し、同じスキーマを指す場合のみ1つにまとめる
- **`nullable: true` は拒否する。** `ObjectKind` に null 許容の表現が無いため、
  黙って非null型として扱うと実際に null が来るケースを取りこぼす。
  `components.schemas` のトップレベルスキーマ自体と、プロパティ・parameter・
  レスポンスのインラインスキーマの両方でチェックする
- **自動生成する対象名(`<operationId>Params` 等)が既存の `components.schemas`
  と衝突する場合は専用のエラーで拒否する。** 黙って上書き・別名採用せず、
  `GeneratedNameCollision` で operationId かスキーマ名の変更を促す
- **`responses.default` は、明示的な2xx/レンジが1件も無い場合に限り成功候補
  として扱う。** それ以外の場合はエラーのフォールバックとして扱う(既存の
  `resolve_error_response` の挙動)。OpenAPI の仕様上 `default` は未列挙の
  任意ステータス(2xx を含みうる)を覆うため、明示的な2xxと `default` が
  共存する場合にこの使い分けは仕様上の曖昧性を完全には解消していないが、
  実務上ほとんどの API で `default` はエラー用に使われるため、Phase1 は
  この単純化を採用する
- **複数のJSON系content type(`application/json` と `application/problem+json`
  等)が同時に宣言され、異なるスキーマを指している場合は拒否する。** YAMLの
  宣言順でどちらかを黙って採用しない。同じスキーマを指している場合は
  1つにまとめる
- **`$ref` は JSON Pointer のエスケープ(`~1` → `/`、`~0` → `~`)を復号する。**
  `Foo/Bar` という名前のスキーマは `$ref: "#/components/schemas/Foo~1Bar"` の
  ように参照されるため、復号せずに文字列として扱うと実在するスキーマが
  未登録として扱われてしまう
- **HTTPヘッダー parameter の名前は大文字小文字を区別せずマージする。**
  path item レベルの `X-Token` と operation レベルの `x-token` は同じ
  ヘッダーを指すため、`merge_parameters` の重複排除キーではヘッダー名のみ
  小文字化して比較する(path/query/cookie の名前は大文字小文字を区別する
  ためそのまま使う)
- **`additionalProperties` に型付きスキーマを持つ辞書型は拒否する。**
  `properties` だけを読んで辞書の側面を黙って無視すると、任意キーの値の型情報が
  失われる。`additionalProperties: true/false`(真偽値)は許容する
- **合成 Product のフィールド名一意化は、由来を付けた後の名前がさらに衝突しても
  必ず一意になるまで連番を付け続ける。** `id_path` という名前の parameter が
  別に実在する場合など、由来による一意化だけでは足りないケースへの対応
