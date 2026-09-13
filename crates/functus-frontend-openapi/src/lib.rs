//! OpenAPI v3.0 の YAML を `functus_core::Category` (圏論的 IR) に変換するクレート。
//!
//! Phase1 のスコープ:
//! - OpenAPI v3.0.x のみ対応する(`openapiv3` クレート自体が v3.0.x 専用のため)。
//!   `openapi` フィールドが `3.0` で始まらない文書は明示的に拒否する。v3.1 対応は
//!   別クレート・別実装が必要で、`docs/design/02-architecture.md` の
//!   「OpenAPI v3.0/3.1」という記述はこの制約を反映できていない。設計文書側の
//!   既知の課題として扱う
//! - `$ref` は `#/components/schemas/<name>` の形のみ解決する。それ以外の参照
//!   (外部ファイル、`responses`/`parameters` コンポーネントへの参照等)は未対応
//! - スキーマはスカラー(string/integer/number/boolean)を持つ `type: object` の
//!   トップレベル定義のみサポートする。ネストしたインラインオブジェクト・配列・
//!   `oneOf`/`anyOf`/`allOf`/`not` は未対応
//! - `nullable: true`、および `additionalProperties` に型付きスキーマを持つ
//!   辞書型は未対応(IRに null 許容・辞書の表現が無いため)
//! - `format` は `string` の `uuid` のみ区別する。`date`/`date-time`/`password`/
//!   `byte`/`binary` を含むそれ以外の `format` はすべて `String` に収束させる
//! - `required` は読まない。すべてのプロパティ・parameter・`requestBody` を
//!   必須として扱うため、実際の API が任意にしているフィールドや
//!   `requestBody.required: false` も生成される型では必須になる
//! - `requestBody` の `application/json` 系スキーマは domain の一部として扱う
//!   (parameter と合わせて2要素以上あれば `body` フィールドを持つ合成 `Product` にする)
//! - レスポンスの content type は `application/json` または `+json` サフィックス
//!   (`application/problem+json` 等)のみ扱う。`; charset=...` のようなパラメータ
//!   付き content type は許容するが、複数のJSON系content typeが異なるスキーマを
//!   指す場合は一意に決められないため拒否する
//! - 明示コードの2xx、非2xx(`responses.default` を含む)は、レンジ定義(`2XX`)を
//!   含めて候補として集め、異なるスキーマを指す場合は一意な型に決められないため
//!   拒否する。同じスキーマを指している場合は1つにまとめる。本文の無いレスポンス
//!   (`204` 等)は `Unit` という型として同じ判定に含める
//! - `responses.default` は、明示的な2xx/レンジが1件も無い場合に限り成功候補として
//!   扱う。それ以外の場合はエラーのフォールバックとして扱う(仕様上の完全な
//!   曖昧性解消ではない簡略化)
#![deny(missing_docs)]

mod error;
mod route;
mod schema;

pub use error::FrontendError;

use functus_core::Category;

/// OpenAPI v3.0 の YAML 文字列を圏論的 IR (`Category`) に変換する。
///
/// # Errors
///
/// YAML のパースに失敗した場合、Phase1 で未対応の `$ref`・スキーマ形状・
/// content type が含まれる場合、`operationId` が無い場合、2xx レスポンスが
/// 無い場合、非2xxレスポンスが複数の異なるスキーマを持つ場合、対象名が
/// 予約スカラー名と衝突する場合、`openapi` フィールドが v3.0.x 以外を示している場合、
/// または圏への登録が失敗した場合に失敗する。
pub fn parse_openapi_yaml(yaml: &str) -> Result<Category, FrontendError> {
    let openapi: openapiv3::OpenAPI = serde_norway::from_str(yaml)?;
    if !is_supported_openapi_version(&openapi.openapi) {
        return Err(FrontendError::UnsupportedOpenApiVersion {
            version: openapi.openapi.clone(),
        });
    }
    let mut category = Category::new();
    schema::register_named_schemas(&mut category, &openapi)?;
    route::register_routes(&mut category, &openapi)?;
    Ok(category)
}

/// `openapi` フィールドの値が `3.0.<patch>`(`<patch>` は非負整数)の形かを検証する。
/// `3.0`・`3.0foo`・`3.01.0` のような形式違反は許容しない。
fn is_supported_openapi_version(version: &str) -> bool {
    match version.split('.').collect::<Vec<_>>().as_slice() {
        [major, minor, patch] => *major == "3" && *minor == "0" && patch.parse::<u32>().is_ok(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use functus_core::{Effect, MorphismId, ObjectId, ObjectKind};

    const SIMPLE_USER_YAML: &str = include_str!("../../../tests/fixtures/simple_user.yaml");

    /// issue #2 のゴール: サンプルの YAML を読み込んだ際、抽出された型と
    /// ルートの情報が正しく `Category` に反映されること。
    #[test]
    fn parses_simple_user_fixture_into_expected_category() {
        let category = parse_openapi_yaml(SIMPLE_USER_YAML).unwrap();

        let user = category.object(&ObjectId::from("User")).unwrap();
        match &user.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(fields[0], ("id".to_string(), ObjectId::from("Uuid")));
                assert_eq!(fields[1], ("name".to_string(), ObjectId::from("String")));
            }
            other => panic!("User は Product を期待したが {other:?} だった"),
        }

        let api_error = category.object(&ObjectId::from("ApiError")).unwrap();
        match &api_error.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(fields[0], ("code".to_string(), ObjectId::from("String")));
                assert_eq!(fields[1], ("message".to_string(), ObjectId::from("String")));
            }
            other => panic!("ApiError は Product を期待したが {other:?} だった"),
        }

        let get_user = category
            .morphism(&MorphismId::from("getUser"))
            .expect("getUser 射が登録されているべき");
        assert_eq!(get_user.dom, ObjectId::from("Uuid"));
        assert_eq!(get_user.cod, ObjectId::from("User"));
        assert_eq!(
            get_user.effects.layers(),
            &[
                Effect::Async,
                Effect::Fallible {
                    error: ObjectId::from("ApiError")
                }
            ]
        );
    }

    #[test]
    fn rejects_invalid_yaml() {
        let err = parse_openapi_yaml("not: [valid, openapi").unwrap_err();
        assert!(matches!(err, FrontendError::Yaml(_)));
    }

    #[test]
    fn rejects_operation_without_operation_id() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      responses:
        "200":
          description: ok
"#;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::MissingOperationId { .. }));
    }

    /// components/parameters への `$ref` は `#/components/schemas/...` の形ではないため未対応。
    #[test]
    fn rejects_unsupported_reference_shapes() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      parameters:
        - $ref: "#/components/parameters/Shared"
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedReference { .. }));
    }

    /// path parameter が複数あるときは `<operationId>Params` という合成 Product を作る。
    #[test]
    fn builds_a_synthetic_params_object_for_multiple_parameters() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /posts/{postId}/comments/{commentId}:
    get:
      operationId: getComment
      parameters:
        - name: postId
          in: path
          required: true
          schema:
            type: string
        - name: commentId
          in: path
          required: true
          schema:
            type: string
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"#;
        let category = parse_openapi_yaml(yaml).unwrap();
        let get_comment = category.morphism(&MorphismId::from("getComment")).unwrap();
        assert_eq!(get_comment.dom, ObjectId::from("getCommentParams"));
        let params = category
            .object(&ObjectId::from("getCommentParams"))
            .unwrap();
        match &params.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(fields[0], ("postId".to_string(), ObjectId::from("String")));
                assert_eq!(
                    fields[1],
                    ("commentId".to_string(), ObjectId::from("String"))
                );
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    /// Codex が指摘した回帰: `components.schemas` の宣言順が依存関係の順と
    /// 逆でも(前方参照でも)登録できなければならない。
    #[test]
    fn registers_schemas_regardless_of_forward_reference_declaration_order() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        address:
          $ref: "#/components/schemas/Address"
    Address:
      type: object
      properties:
        city:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let user = category.object(&ObjectId::from("User")).unwrap();
        match &user.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(
                    fields[1],
                    ("address".to_string(), ObjectId::from("Address"))
                );
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
        assert!(category.object(&ObjectId::from("Address")).is_some());
    }

    /// Codex が指摘した回帰: operation レベルの parameter は同名の path item
    /// レベルの parameter を上書きする(二重定義として扱わない)。
    #[test]
    fn operation_level_parameter_overrides_path_item_level_parameter() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items/{id}:
    parameters:
      - name: id
        in: path
        required: true
        schema:
          type: string
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"#;
        let category = parse_openapi_yaml(yaml).unwrap();
        let get_item = category.morphism(&MorphismId::from("getItem")).unwrap();
        // path item 側は string、operation 側は uuid で上書きしている。
        // 上書きが効いていれば重複フィールドエラーにならず、Uuid が採用される。
        assert_eq!(get_item.dom, ObjectId::from("Uuid"));
    }

    /// Codex が指摘した回帰: エラーレスポンスの `$ref` を黙って読み飛ばさず、
    /// 未対応として拒否しなければならない。読み飛ばすと `Fallible` 効果が
    /// 静かに欠落した不完全な IR ができてしまう。
    #[test]
    fn rejects_referenced_error_response_instead_of_dropping_it() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items/{id}:
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
        "404":
          $ref: "#/components/responses/NotFound"
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedReference { .. }));
    }

    /// Critical回帰: requestBodyを黙って無視せず、domainの一部として扱う。
    #[test]
    fn request_body_becomes_the_sole_domain_when_there_are_no_parameters() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /users:
    post:
      operationId: createUser
      requestBody:
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/User"
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
components:
  schemas:
    User:
      type: object
      properties:
        name:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let create_user = category.morphism(&MorphismId::from("createUser")).unwrap();
        assert_eq!(create_user.dom, ObjectId::from("User"));
    }

    /// Critical回帰: parameterとrequestBodyが両方あるoperationは、
    /// bodyフィールドを持つ合成Productをdomainにする。
    #[test]
    fn request_body_and_parameters_combine_into_a_synthetic_product() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /users/{id}:
    put:
      operationId: updateUser
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      requestBody:
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/User"
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
components:
  schemas:
    User:
      type: object
      properties:
        name:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let update_user = category.morphism(&MorphismId::from("updateUser")).unwrap();
        let params = category
            .object(&update_user.dom)
            .expect("合成Productが登録されているべき");
        match &params.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(fields[0], ("id".to_string(), ObjectId::from("String")));
                assert_eq!(fields[1], ("body".to_string(), ObjectId::from("User")));
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    /// Critical回帰: responses.defaultを黙って無視せず、非2xxが無い場合の
    /// エラー型として使う。
    #[test]
    fn responses_default_is_used_as_the_error_type_when_no_explicit_non_2xx_exists() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
        default:
          description: error
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ApiError"
components:
  schemas:
    ApiError:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(
            ping.effects.layers(),
            &[
                Effect::Async,
                Effect::Fallible {
                    error: ObjectId::from("ApiError")
                }
            ]
        );
    }

    /// Critical回帰: 複数の非2xxレスポンスが異なるスキーマを持つ場合は、
    /// 黙って最初の1件を採用せず拒否する。
    #[test]
    fn rejects_multiple_distinct_error_response_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
        "404":
          description: not found
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/NotFoundError"
        "409":
          description: conflict
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ConflictError"
components:
  schemas:
    NotFoundError:
      type: object
      properties:
        message:
          type: string
    ConflictError:
      type: object
      properties:
        message:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::MultipleErrorResponsesUnsupported { .. }
        ));
    }

    /// 複数の非2xxレスポンスが同じスキーマを指している場合は、1つにまとめて
    /// 採用する(異なるスキーマの場合とは違い拒否しない)。
    #[test]
    fn multiple_error_responses_with_the_same_schema_are_merged() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
        "404":
          description: not found
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ApiError"
        "409":
          description: conflict
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ApiError"
components:
  schemas:
    ApiError:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(
            ping.effects.layers(),
            &[
                Effect::Async,
                Effect::Fallible {
                    error: ObjectId::from("ApiError")
                }
            ]
        );
    }

    /// Critical回帰: 明示コード(200)はレンジ定義(2XX)より優先される。
    #[test]
    fn rejects_distinct_schemas_across_explicit_code_and_range() {
        // "2XX" は "200" 以外の2xx(201等)にも適用されうるため、異なるスキーマを
        // 指す場合は "200" が黙って優先されるべきではなく、あいまいとして拒否する。
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "2XX":
          description: generic success
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Generic"
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Specific"
components:
  schemas:
    Generic:
      type: object
      properties:
        g:
          type: string
    Specific:
      type: object
      properties:
        s:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::MultipleSuccessResponsesUnsupported { .. }
        ));
    }

    /// "2XX" と "200" が同じスキーマを指している場合は、あいまいではないため
    /// 1つにまとめて受理する。
    #[test]
    fn merges_explicit_code_and_range_when_they_share_the_same_schema() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "2XX":
          description: generic success
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pong"
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pong"
components:
  schemas:
    Pong:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(ping.cod, ObjectId::from("Pong"));
    }

    /// Critical回帰: 本文の無い2xx(204等)はエラーにせずUnitをcodにする。
    #[test]
    fn no_content_success_response_maps_to_unit() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /users/{id}:
    delete:
      operationId: deleteUser
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        "204":
          description: no content
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let delete_user = category.morphism(&MorphismId::from("deleteUser")).unwrap();
        assert_eq!(delete_user.cod, ObjectId::from("Unit"));
    }

    /// Critical回帰: application/problem+json や charset付きのcontent typeも
    /// json系として扱う。
    #[test]
    fn accepts_json_suffixed_and_parameterized_content_types() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json; charset=utf-8:
              schema:
                type: string
        "404":
          description: not found
          content:
            application/problem+json:
              schema:
                $ref: "#/components/schemas/ApiError"
components:
  schemas:
    ApiError:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(
            ping.effects.layers(),
            &[
                Effect::Async,
                Effect::Fallible {
                    error: ObjectId::from("ApiError")
                }
            ]
        );
    }

    /// High回帰: 組み込みスカラー名(Uuid等)と同名のスキーマ定義は拒否する。
    #[test]
    fn rejects_a_component_schema_named_like_a_reserved_scalar() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    Uuid:
      type: object
      properties:
        raw:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::ScalarNameCollision { .. }));
    }

    /// High回帰: date-time 等の未対応 format は(拒否ではなく)String に収束する。
    /// この挙動は crates/functus-frontend-openapi/CLAUDE.md に明記している。
    #[test]
    fn unsupported_string_formats_collapse_to_string() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    Event:
      type: object
      properties:
        occurredAt:
          type: string
          format: date-time
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let event = category.object(&ObjectId::from("Event")).unwrap();
        match &event.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(
                    fields[0],
                    ("occurredAt".to_string(), ObjectId::from("String"))
                );
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    /// path parameter と query parameter は `(name, in)` の組で区別されるため、
    /// 名前が同じでも `in` が違えばマージされず両方残る。
    #[test]
    fn parameters_with_the_same_name_but_different_in_are_not_merged() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items/{id}:
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
        - name: id
          in: query
          required: false
          schema:
            type: integer
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let get_item = category.morphism(&MorphismId::from("getItem")).unwrap();
        let params = category.object(&get_item.dom).unwrap();
        match &params.kind {
            ObjectKind::Product(fields) => {
                // 名前だけでは両方とも "id" になり Product のフィールド名が
                // 衝突するため、由来(in)を付けて一意化する。
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0], ("id_path".to_string(), ObjectId::from("String")));
                assert_eq!(
                    fields[1],
                    ("id_query".to_string(), ObjectId::from("Integer"))
                );
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    /// schema.rs の fixpoint ループが自己参照を循環参照として拒否することを確認する。
    #[test]
    fn rejects_self_referencing_schema() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    Node:
      type: object
      properties:
        next:
          $ref: "#/components/schemas/Node"
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// schema.rs の fixpoint ループが相互参照を循環参照として拒否することを確認する。
    #[test]
    fn rejects_mutually_referencing_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    A:
      type: object
      properties:
        b:
          $ref: "#/components/schemas/B"
    B:
      type: object
      properties:
        a:
          $ref: "#/components/schemas/A"
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// 3段の依存チェーンを逆順(C, B, A の順に依存される A, B, C を A, B, C の
    /// 宣言順のまま定義しつつ実際には C→B→A の逆順参照)で解決できることを確認し、
    /// fixpoint ループが複数パスにわたって正しく進むことを検証する。
    #[test]
    fn resolves_a_three_level_forward_reference_chain() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    A:
      type: object
      properties:
        b:
          $ref: "#/components/schemas/B"
    B:
      type: object
      properties:
        c:
          $ref: "#/components/schemas/C"
    C:
      type: object
      properties:
        value:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        assert!(category.object(&ObjectId::from("A")).is_some());
        assert!(category.object(&ObjectId::from("B")).is_some());
        assert!(category.object(&ObjectId::from("C")).is_some());
    }

    /// Codex回帰: openapiフィールドがv3.1を示していても構文的に読めてしまうため、
    /// 意味的な互換性が無いことを明示的に検証してエラーにする。
    #[test]
    fn rejects_openapi_3_1_documents() {
        let yaml = r##"
openapi: 3.1.0
info:
  title: t
  version: "1.0.0"
paths: {}
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::UnsupportedOpenApiVersion { .. }
        ));
    }

    /// Codex回帰: `3.0`で始まるだけの不正な形式(セマンティックバージョンでない)は
    /// prefix一致では通ってしまっていたため拒否する。
    #[test]
    fn rejects_malformed_openapi_version_strings() {
        for version in ["3.0", "3.0foo", "3.01.0"] {
            let yaml = format!(
                "openapi: \"{version}\"\ninfo:\n  title: t\n  version: \"1.0.0\"\npaths: {{}}\n"
            );
            let err = parse_openapi_yaml(&yaml).unwrap_err();
            assert!(
                matches!(err, FrontendError::UnsupportedOpenApiVersion { .. }),
                "version {version:?} should be rejected"
            );
        }
    }

    /// Codex回帰: `id_path`のような由来を付けた一意化後の名前が、たまたま
    /// 別の実在するparameter名と衝突する場合でも最終的に一意になる。
    #[test]
    fn disambiguated_field_names_do_not_collide_with_pre_existing_names() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items/{id}:
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
        - name: id
          in: query
          required: false
          schema:
            type: integer
        - name: id_path
          in: header
          required: false
          schema:
            type: boolean
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let get_item = category.morphism(&MorphismId::from("getItem")).unwrap();
        let params = category.object(&get_item.dom).unwrap();
        match &params.kind {
            ObjectKind::Product(fields) => {
                let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
                let mut unique = names.clone();
                unique.sort_unstable();
                unique.dedup();
                assert_eq!(
                    names.len(),
                    unique.len(),
                    "フィールド名が重複してはいけない: {names:?}"
                );
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    /// Codex回帰: 明示コードの2xxレスポンスが2種類以上の異なるスキーマを
    /// 持つ場合、片方を黙って選ばず拒否する。
    #[test]
    fn rejects_multiple_distinct_explicit_success_response_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /users:
    post:
      operationId: createUser
      responses:
        "200":
          description: existing user
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
        "201":
          description: newly created user
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/CreatedUser"
components:
  schemas:
    User:
      type: object
      properties:
        name:
          type: string
    CreatedUser:
      type: object
      properties:
        name:
          type: string
        createdAt:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::MultipleSuccessResponsesUnsupported { .. }
        ));
    }

    /// Codex回帰: 2XXレンジと非2xxの明示コード(404)が両方あっても、
    /// レンジの成功スキーマを黙って捨ててはいけない。明示コード優先の判定は
    /// 「2xxの」明示コードにだけ適用されるべきで、非2xxの明示コードの存在は
    /// レンジの採用を妨げない。
    #[test]
    fn range_success_response_is_kept_when_only_non_2xx_explicit_codes_exist() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "2XX":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pong"
        "404":
          description: not found
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ApiError"
components:
  schemas:
    Pong:
      type: object
      properties:
        message:
          type: string
    ApiError:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(ping.cod, ObjectId::from("Pong"));
    }

    /// Codex回帰: 本文付きの2xx(200: User)と本文無しの2xx(204)が両方ある操作は、
    /// 呼び出し側が受け取る型が状況によって変わる(User または Unit)ため、
    /// 片方を黙って採用せず拒否する。
    #[test]
    fn rejects_a_mix_of_schema_and_no_content_success_responses() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /users:
    post:
      operationId: createUser
      responses:
        "200":
          description: existing user returned
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
        "204":
          description: accepted with no content
components:
  schemas:
    User:
      type: object
      properties:
        name:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::MultipleSuccessResponsesUnsupported { .. }
        ));
    }

    /// Codex回帰: `nullable: true` を黙って非null型として扱わず拒否する。
    /// IRにnull許容の表現が無いため、黙って扱うとAPIが実際にnullを返す
    /// ケースを取りこぼす。
    #[test]
    fn rejects_nullable_properties() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    User:
      type: object
      properties:
        nickname:
          type: string
          nullable: true
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// Codex回帰: `additionalProperties` に型付きスキーマを持つ辞書型を
    /// propertiesだけから登録して辞書の側面を黙って失わず拒否する。
    #[test]
    fn rejects_typed_dictionary_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    Tags:
      type: object
      additionalProperties:
        type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// Codex回帰: components.schemasのトップレベルスキーマ自体がnullable: trueの
    /// 場合も(プロパティのnullableと同様に)拒否する。
    #[test]
    fn rejects_nullable_top_level_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths: {}
components:
  schemas:
    User:
      type: object
      nullable: true
      properties:
        name:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// Codex回帰: 合成Params対象名(<operationId>Params)が既存のcomponents.schemas
    /// の対象名と衝突する場合、意味不明なDuplicateObjectではなく専用の
    /// エラーで拒否する。
    #[test]
    fn rejects_generated_params_name_colliding_with_existing_schema() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items/{id}:
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
        - name: filter
          in: query
          required: false
          schema:
            type: string
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
components:
  schemas:
    getItemParams:
      type: object
      properties:
        note:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::GeneratedNameCollision { .. }));
    }

    /// Codex回帰: 明示的な2xx/レンジが1件も無くdefaultだけがある操作は、
    /// defaultを唯一の成功候補として使う(以前はMissingSuccessResponseに
    /// なっていた)。
    #[test]
    fn default_only_response_is_used_as_the_success_type() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        default:
          description: the only documented response
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pong"
components:
  schemas:
    Pong:
      type: object
      properties:
        message:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(ping.cod, ObjectId::from("Pong"));
        // 2xxが無くdefaultを成功として消費したため、Fallible効果は付かない。
        assert_eq!(ping.effects.layers(), &[Effect::Async]);
    }

    /// Codex回帰: application/json と application/problem+json のように
    /// 複数のJSON系content typeが同時に宣言され、かつ異なるスキーマを
    /// 指している場合、YAMLの宣言順で片方を黙って採用せず拒否する。
    #[test]
    fn rejects_multiple_json_media_types_with_different_schemas() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pong"
            application/problem+json:
              schema:
                $ref: "#/components/schemas/ApiError"
components:
  schemas:
    Pong:
      type: object
      properties:
        message:
          type: string
    ApiError:
      type: object
      properties:
        message:
          type: string
"##;
        let err = parse_openapi_yaml(yaml).unwrap_err();
        assert!(matches!(err, FrontendError::UnsupportedSchema { .. }));
    }

    /// Codex回帰: `/`を含む名前のスキーマはJSON Pointerエスケープ(`~1`)で
    /// 参照されるため、デコードせずに未登録扱いにしてはいけない。
    #[test]
    fn resolves_json_pointer_escaped_schema_references() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Foo~1Bar"
components:
  schemas:
    Foo/Bar:
      type: object
      properties:
        value:
          type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        assert!(category.object(&ObjectId::from("Foo/Bar")).is_some());
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(ping.cod, ObjectId::from("Foo/Bar"));
    }

    /// Codex回帰: HTTPヘッダー名は大文字小文字を区別しないため、path itemレベルの
    /// `X-Token` はoperationレベルの`x-token`で正しく上書きされなければならない
    /// (別々のフィールドとして残ってはいけない)。
    #[test]
    fn header_parameter_override_is_case_insensitive() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /items:
    parameters:
      - name: X-Token
        in: header
        required: true
        schema:
          type: string
    get:
      operationId: getItems
      parameters:
        - name: x-token
          in: header
          required: true
          schema:
            type: integer
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let get_items = category.morphism(&MorphismId::from("getItems")).unwrap();
        // 上書きが効いていれば単一のparameter(operation側のinteger)になり、
        // 合成Productではなくスカラー1つがdomainになる。
        assert_eq!(get_items.dom, ObjectId::from("Integer"));
    }

    /// Codex回帰: メディアタイプはASCII大文字小文字を区別しないため、
    /// `Application/JSON`のような大文字混じりのcontent typeも
    /// `application/json`として扱わなければならない。
    #[test]
    fn matches_json_media_types_case_insensitively() {
        let yaml = r##"
openapi: 3.0.3
info:
  title: t
  version: "1.0.0"
paths:
  /ping:
    get:
      operationId: ping
      responses:
        "200":
          description: ok
          content:
            Application/JSON:
              schema:
                type: string
"##;
        let category = parse_openapi_yaml(yaml).unwrap();
        let ping = category.morphism(&MorphismId::from("ping")).unwrap();
        assert_eq!(ping.cod, ObjectId::from("String"));
    }
}
