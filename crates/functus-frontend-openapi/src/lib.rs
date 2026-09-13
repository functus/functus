//! OpenAPI v3.0 の YAML を `functus_core::Category` (圏論的 IR) に変換するクレート。
//!
//! Phase1 のスコープ:
//! - OpenAPI v3.0.x のみ対応する(`openapiv3` クレート自体が v3.0.x 専用のため)。
//!   v3.1 対応は別クレート・別実装が必要で、`docs/design/02-architecture.md` の
//!   「OpenAPI v3.0/3.1」という記述はこの制約を反映できていない。設計文書側の
//!   既知の課題として扱う
//! - `$ref` は `#/components/schemas/<name>` の形のみ解決する。それ以外の参照
//!   (外部ファイル、`responses`/`parameters` コンポーネントへの参照等)は未対応
//! - スキーマはスカラー(string/integer/number/boolean)を持つ `type: object` の
//!   トップレベル定義のみサポートする。ネストしたインラインオブジェクト・配列・
//!   `oneOf`/`anyOf`/`allOf`/`not` は未対応
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
/// YAML のパースに失敗した場合、Phase1 で未対応の `$ref`・スキーマ形状が
/// 含まれる場合、`operationId` が無い場合、2xx のスキーマ付きレスポンスが
/// 無い場合、または圏への登録が失敗した場合に失敗する。
pub fn parse_openapi_yaml(yaml: &str) -> Result<Category, FrontendError> {
    let openapi: openapiv3::OpenAPI = serde_norway::from_str(yaml)?;
    let mut category = Category::new();
    schema::register_named_schemas(&mut category, &openapi)?;
    route::register_routes(&mut category, &openapi)?;
    Ok(category)
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
}
