//! OpenAPI のパス+メソッド(operation)を `functus_core::Morphism` に変換する。

use std::collections::{HashMap, HashSet};

use openapiv3::{Content, OpenAPI, Operation, Parameter, PathItem, ReferenceOr, StatusCode};

use functus_core::{Category, Effect, EffectStack, Object, ObjectId};

use crate::error::FrontendError;
use crate::schema::{intern_scalar, resolve_schema};

/// `paths` のすべての operation を効果付き基本射として登録する。
pub(crate) fn register_routes(
    category: &mut Category,
    openapi: &OpenAPI,
) -> Result<(), FrontendError> {
    for (path, path_item_or_ref) in openapi.paths.iter() {
        let path_item = match path_item_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("paths.{path}"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(path_item) => path_item,
        };

        for (method, operation) in operations(path_item) {
            register_operation(category, path, method, path_item, operation)?;
        }
    }
    Ok(())
}

fn operations(path_item: &PathItem) -> Vec<(&'static str, &Operation)> {
    [
        ("GET", path_item.get.as_ref()),
        ("PUT", path_item.put.as_ref()),
        ("POST", path_item.post.as_ref()),
        ("DELETE", path_item.delete.as_ref()),
        ("OPTIONS", path_item.options.as_ref()),
        ("HEAD", path_item.head.as_ref()),
        ("PATCH", path_item.patch.as_ref()),
        ("TRACE", path_item.trace.as_ref()),
    ]
    .into_iter()
    .filter_map(|(method, operation)| operation.map(|operation| (method, operation)))
    .collect()
}

fn register_operation(
    category: &mut Category,
    path: &str,
    method: &str,
    path_item: &PathItem,
    operation: &Operation,
) -> Result<(), FrontendError> {
    let operation_id =
        operation
            .operation_id
            .clone()
            .ok_or_else(|| FrontendError::MissingOperationId {
                method: method.to_string(),
                path: path.to_string(),
            })?;

    let dom = resolve_domain(category, &operation_id, path_item, operation)?;
    let (cod, mut effects) = resolve_success_response(category, &operation_id, operation)?;
    if let Some(error_object) = resolve_error_response(category, &operation_id, operation)? {
        effects.push(Effect::Fallible {
            error: error_object,
        });
    }

    category.add_effectful_primitive_morphism(
        operation_id,
        dom,
        cod,
        EffectStack::wrapping(effects),
    )?;
    Ok(())
}

/// `(名前, 由来, 型)` の3つ組を、名前が衝突しない `(フィールド名, 型)` の列に変換する。
///
/// まず名前だけで衝突する要素に由来(`in`/`body`)を付けて一意化を試みる。
/// それでもなお(たとえば元から `id_path` という名前の要素が別にあった場合など)
/// 衝突するなら、連番を付けて必ず一意になるまで続ける。
fn disambiguate_field_names(
    entries: Vec<(String, &'static str, ObjectId)>,
) -> Vec<(String, ObjectId)> {
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for (name, _, _) in &entries {
        *name_counts.entry(name.clone()).or_insert(0) += 1;
    }

    let mut used_names: HashSet<String> = HashSet::new();
    let mut result = Vec::with_capacity(entries.len());
    for (name, kind, object) in entries {
        let base_name = if name_counts.get(&name).copied().unwrap_or(0) > 1 {
            format!("{name}_{kind}")
        } else {
            name.clone()
        };
        let mut field_name = base_name.clone();
        let mut suffix = 2;
        while used_names.contains(&field_name) {
            field_name = format!("{base_name}{suffix}");
            suffix += 1;
        }
        used_names.insert(field_name.clone());
        result.push((field_name, object));
    }
    result
}

/// operation の domain(入力型)を、path/query 等の parameter と `requestBody` から
/// 組み立てる。要素が0個なら `Unit`、1個ならそのまま、2個以上なら
/// `<operationId>Params` という合成 `Product` にする。`requestBody` がある場合は
/// `body` という名前のフィールドとして扱う。
fn resolve_domain(
    category: &mut Category,
    operation_id: &str,
    path_item: &PathItem,
    operation: &Operation,
) -> Result<ObjectId, FrontendError> {
    let params = merge_parameters(path_item, operation);

    // (パラメータ名, 由来 (path/query/header/cookie/body), 型) の3つ組で集める。
    // `id` (path) と `id` (query) のように name だけが衝突する場合があるため、
    // Product のフィールド名を決める最後の段階で由来を使って一意化する。
    let mut resolved: Vec<(String, &'static str, ObjectId)> = Vec::with_capacity(params.len() + 1);
    for param_or_ref in params {
        let param = match param_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の parameter"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(param) => param,
        };
        let data = param.parameter_data_ref();
        let openapiv3::ParameterSchemaOrContent::Schema(schema_or_ref) = &data.format else {
            return Err(FrontendError::UnsupportedSchema {
                schema: format!("{operation_id}.{}", data.name),
                reason: "content 形式の parameter は Phase1 で未対応".to_string(),
            });
        };
        let context = format!("{operation_id} の parameter {}", data.name);
        let object = resolve_schema(category, &context, schema_or_ref)?;
        let kind = match param {
            Parameter::Query { .. } => "query",
            Parameter::Header { .. } => "header",
            Parameter::Path { .. } => "path",
            Parameter::Cookie { .. } => "cookie",
        };
        resolved.push((data.name.clone(), kind, object));
    }

    if let Some(body_or_ref) = &operation.request_body {
        let body = match body_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の requestBody"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(body) => body,
        };
        let context = format!("{operation_id} の requestBody");
        if let Some(object) = resolve_json_content(category, &context, &body.content)? {
            resolved.push(("body".to_string(), "body", object));
        } else if !body.content.is_empty() {
            return Err(FrontendError::UnsupportedSchema {
                schema: context,
                reason: "application/json 系以外の content type は Phase1 で未対応".to_string(),
            });
        }
    }

    let resolved = disambiguate_field_names(resolved);

    match resolved.as_slice() {
        [] => intern_scalar(category, "Unit"),
        [(_, object)] => Ok(object.clone()),
        _ => {
            let params_object_name = format!("{operation_id}Params");
            if category
                .object(&ObjectId::from(params_object_name.as_str()))
                .is_some()
            {
                return Err(FrontendError::GeneratedNameCollision {
                    operation_id: operation_id.to_string(),
                    name: params_object_name,
                });
            }
            category
                .add_object(Object::product(params_object_name, resolved))
                .map_err(FrontendError::from)
        }
    }
}

/// path item レベルの parameter と operation レベルの parameter をマージする。
///
/// OpenAPI の仕様上、operation レベルの parameter は同じ `(name, in)` の
/// path item レベルの定義を上書きできる(削除はできない)。単純に連結すると
/// 上書きのつもりが二重定義になり、`(name, in)` が同じ scalar パラメータが
/// 2件あるかのように扱われてしまう。
fn merge_parameters<'a>(
    path_item: &'a PathItem,
    operation: &'a Operation,
) -> Vec<&'a ReferenceOr<Parameter>> {
    let mut merged: Vec<&'a ReferenceOr<Parameter>> = Vec::new();
    let mut index_by_key: HashMap<(String, &'static str), usize> = HashMap::new();

    for param_or_ref in path_item
        .parameters
        .iter()
        .chain(operation.parameters.iter())
    {
        let key = parameter_key(param_or_ref);
        if let Some(&index) = index_by_key.get(&key) {
            merged[index] = param_or_ref;
        } else {
            index_by_key.insert(key, merged.len());
            merged.push(param_or_ref);
        }
    }
    merged
}

/// `(name, in)` に相当する重複排除キーを作る。`$ref` は名前を知らないため
/// 参照文字列そのものをキーにし、他の `$ref` や名前付き parameter と
/// 衝突しないようにする。
fn parameter_key(param_or_ref: &ReferenceOr<Parameter>) -> (String, &'static str) {
    match param_or_ref {
        ReferenceOr::Reference { reference } => (reference.clone(), "$ref"),
        ReferenceOr::Item(param) => {
            let kind = match param {
                Parameter::Query { .. } => "query",
                Parameter::Header { .. } => "header",
                Parameter::Path { .. } => "path",
                Parameter::Cookie { .. } => "cookie",
            };
            let name = param.parameter_data_ref().name.clone();
            // HTTPヘッダー名は大文字小文字を区別しないため、path itemレベルの
            // `X-Token` とoperationレベルの`x-token`は同じキーになる必要がある。
            let name = if matches!(param, Parameter::Header { .. }) {
                name.to_lowercase()
            } else {
                name
            };
            (name, kind)
        }
    }
}

/// `content` から JSON 系のメディアタイプ(`application/json`、
/// `application/problem+json` のような `+json` サフィックス、
/// `application/json; charset=utf-8` のようなパラメータ付きも含む)をすべて解決し、
/// 単一の対象 ID にまとめる。
///
/// `application/json` と `application/problem+json` のように複数の JSON 系
/// メディアタイプが同時に宣言されていて、かつ異なるスキーマを指している場合、
/// どちらを採用すべきかは YAML の宣言順ではなく実際のネゴシエーション次第であり
/// 一意に決められないため拒否する。同じスキーマを指している場合は1つにまとめる。
/// マッチする JSON 系メディアタイプが無い場合は `Ok(None)` を返す。
fn resolve_json_content(
    category: &mut Category,
    context: &str,
    content: &Content,
) -> Result<Option<ObjectId>, FrontendError> {
    let mut resolved: Vec<ObjectId> = Vec::new();
    for (media_type, media) in content {
        // メディアタイプは ASCII 大文字小文字を区別しない
        // (RFC 2045 の type/subtype の定義による)。
        let essence = media_type
            .split(';')
            .next()
            .unwrap_or(media_type)
            .trim()
            .to_ascii_lowercase();
        if essence != "application/json" && !essence.ends_with("+json") {
            continue;
        }
        let Some(schema_or_ref) = &media.schema else {
            continue;
        };
        let object = resolve_schema(category, context, schema_or_ref)?;
        if !resolved.contains(&object) {
            resolved.push(object);
        }
    }

    match resolved.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        _ => Err(FrontendError::UnsupportedSchema {
            schema: context.to_string(),
            reason: "複数のJSON系content typeが異なるスキーマを持つ場合はPhase1で未対応"
                .to_string(),
        }),
    }
}

/// operation の 2xx レスポンスから `cod` を選ぶ。
///
/// レンジ定義(`2XX`)は、明示コード(`200` 等)がある場合でもそのコード「以外」の
/// 2xx ステータスに引き続き適用されうる(例: `200` と `2XX` が両方あれば、
/// `201` は `2XX` の定義に従う)。そのため明示コードとレンジ定義のどちらか一方を
/// 無視するのではなく、両方を候補として集めたうえで、異なるスキーマを指す
/// 組み合わせがあれば `cod` を一意に決められないため拒否する。同じスキーマを
/// 指している場合、および本文が無い(`204` 等)レスポンスが `Unit` 1種類だけの
/// 場合は1つにまとめる。
///
/// `responses.default` は明示的な 2xx/レンジが1件も無い場合に限り成功候補として
/// 扱う(その場合、成功時の型を説明しているのは `default` しかないため)。
/// 2xx が既にある場合、`default` は `resolve_error_response` 側でエラー
/// フォールバックとして扱う。`default` は本来 2xx を含む未列挙の任意ステータスを
/// 覆いうるため、この使い分けは仕様上の曖昧性を完全には解消しないが、
/// 実務上ほとんどの API で `default` はエラー用に使われるため、Phase1 では
/// この単純化を採用する(crates/functus-frontend-openapi/CLAUDE.md に記録)。
fn resolve_success_response(
    category: &mut Category,
    operation_id: &str,
    operation: &Operation,
) -> Result<(ObjectId, Vec<Effect>), FrontendError> {
    let responses = &operation.responses.responses;
    if !responses.keys().any(is_success) {
        let Some(default) = &operation.responses.default else {
            return Err(FrontendError::MissingSuccessResponse {
                operation_id: operation_id.to_string(),
            });
        };
        let label = "default".to_string();
        let response = match default {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の {label} レスポンス"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(response) => response,
        };
        let object = resolve_response_body(category, operation_id, &label, response)?;
        return Ok((object, vec![Effect::Async]));
    }

    let mut found: Vec<(String, ObjectId)> = Vec::new();
    for (status, response_or_ref) in responses {
        if !is_success(status) {
            continue;
        }
        let label = status.to_string();
        let response = match response_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の {label} レスポンス"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(response) => response,
        };
        // 本文が無いレスポンス(204等)も Unit という「型」として曖昧性判定に
        // 含める。スキップしてしまうと、`200: User` と本文無しの `204` が
        // 両方ある操作で `User` を無条件に採用してしまい、実際には呼び出し側が
        // 何も受け取らない可能性があることを黙って無視することになる。
        let object = resolve_response_body(category, operation_id, &label, response)?;
        found.push((label, object));
    }

    let mut unique_objects: Vec<ObjectId> = Vec::new();
    for (_, object) in &found {
        if !unique_objects.contains(object) {
            unique_objects.push(object.clone());
        }
    }

    match unique_objects.as_slice() {
        [] => {
            // 冒頭で2xxが1件以上あることを確認しており、上のループは対象と
            // なる2xxを1件も取りこぼさず必ず何か(Unitかスキーマ)を積むため、
            // ここに来ることは無いはずだが、防御的にUnitを返しておく。
            let unit = intern_scalar(category, "Unit")?;
            Ok((unit, vec![Effect::Async]))
        }
        [only] => Ok((only.clone(), vec![Effect::Async])),
        _ => Err(FrontendError::MultipleSuccessResponsesUnsupported {
            operation_id: operation_id.to_string(),
            labels: found.into_iter().map(|(label, _)| label).collect(),
        }),
    }
}

/// `response` の本文を解決する。本文が無ければ `Unit` を返す。
fn resolve_response_body(
    category: &mut Category,
    operation_id: &str,
    label: &str,
    response: &openapiv3::Response,
) -> Result<ObjectId, FrontendError> {
    if response.content.is_empty() {
        return intern_scalar(category, "Unit");
    }
    let context = format!("{operation_id} の {label} レスポンス");
    resolve_json_content(category, &context, &response.content)?.ok_or_else(|| {
        FrontendError::UnsupportedSchema {
            schema: context.clone(),
            reason: "application/json 系以外の content type は Phase1 で未対応".to_string(),
        }
    })
}

/// operation の非2xxレスポンス(`responses.default` を含む)からエラー型を選ぶ。
///
/// `responses.default` は、`resolve_success_response` が2xxの成功候補として
/// 既に消費している場合(2xxが1件も無いoperation)はここでは対象にしない。
/// それ以外の場合は、`EffectStack` の `Fallible` 層は1つしか持てないため、
/// 複数の非2xxレスポンスが異なるスキーマを持つ場合は一意に決められず拒否する。
/// 同じスキーマを指している(例: 404 と 409 がどちらも `ApiError`)場合は
/// 1つにまとめて採用する。
fn resolve_error_response(
    category: &mut Category,
    operation_id: &str,
    operation: &Operation,
) -> Result<Option<ObjectId>, FrontendError> {
    let has_2xx = operation.responses.responses.keys().any(is_success);

    let mut candidates: Vec<(String, &ReferenceOr<openapiv3::Response>)> = operation
        .responses
        .responses
        .iter()
        .filter(|(status, _)| !is_success(status))
        .map(|(status, response)| (status.to_string(), response))
        .collect();
    if has_2xx {
        if let Some(default) = &operation.responses.default {
            candidates.push(("default".to_string(), default));
        }
    }

    let mut found: Vec<(String, ObjectId)> = Vec::new();
    for (label, response_or_ref) in candidates {
        let response = match response_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の response {label}"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(response) => response,
        };
        if response.content.is_empty() {
            // 本文の無いエラーレスポンス(例: 401 のみでボディ無し)は
            // Fallible の対象にしない。
            continue;
        }
        let context = format!("{operation_id} の response {label}");
        let Some(object) = resolve_json_content(category, &context, &response.content)? else {
            return Err(FrontendError::UnsupportedSchema {
                schema: context,
                reason: "application/json 系以外の content type は Phase1 で未対応".to_string(),
            });
        };
        found.push((label, object));
    }

    let mut unique_objects: Vec<ObjectId> = Vec::new();
    for (_, object) in &found {
        if !unique_objects.contains(object) {
            unique_objects.push(object.clone());
        }
    }

    match unique_objects.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        _ => Err(FrontendError::MultipleErrorResponsesUnsupported {
            operation_id: operation_id.to_string(),
            labels: found.into_iter().map(|(label, _)| label).collect(),
        }),
    }
}

fn is_success(status: &StatusCode) -> bool {
    matches!(status, StatusCode::Code(200..=299) | StatusCode::Range(2))
}
