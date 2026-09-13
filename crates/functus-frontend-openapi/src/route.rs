//! OpenAPI のパス+メソッド(operation)を `functus_core::Morphism` に変換する。

use std::collections::HashMap;

use openapiv3::{OpenAPI, Operation, Parameter, PathItem, ReferenceOr, StatusCode};

use functus_core::{Category, Effect, EffectStack, Object, ObjectId};

use crate::error::FrontendError;
use crate::schema::resolve_schema;

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

fn resolve_domain(
    category: &mut Category,
    operation_id: &str,
    path_item: &PathItem,
    operation: &Operation,
) -> Result<ObjectId, FrontendError> {
    let params = merge_parameters(path_item, operation);

    let mut resolved = Vec::with_capacity(params.len());
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
        resolved.push((data.name.clone(), object));
    }

    match resolved.as_slice() {
        [] => Ok(crate::schema::intern_scalar(category, "Unit")),
        [(_, object)] => Ok(object.clone()),
        _ => {
            let params_object_name = format!("{operation_id}Params");
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
            (param.parameter_data_ref().name.clone(), kind)
        }
    }
}

fn resolve_success_response(
    category: &mut Category,
    operation_id: &str,
    operation: &Operation,
) -> Result<(ObjectId, Vec<Effect>), FrontendError> {
    for (status, response_or_ref) in &operation.responses.responses {
        if !is_success(status) {
            continue;
        }
        let response = match response_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の response {status}"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(response) => response,
        };
        if let Some(schema_or_ref) = response
            .content
            .get("application/json")
            .and_then(|media| media.schema.as_ref())
        {
            let context = format!("{operation_id} の {status} レスポンス");
            let object = resolve_schema(category, &context, schema_or_ref)?;
            return Ok((object, vec![Effect::Async]));
        }
    }
    Err(FrontendError::MissingSuccessResponse {
        operation_id: operation_id.to_string(),
    })
}

fn resolve_error_response(
    category: &mut Category,
    operation_id: &str,
    operation: &Operation,
) -> Result<Option<ObjectId>, FrontendError> {
    for (status, response_or_ref) in &operation.responses.responses {
        if is_success(status) {
            continue;
        }
        let response = match response_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("{operation_id} の response {status}"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(response) => response,
        };
        if let Some(schema_or_ref) = response
            .content
            .get("application/json")
            .and_then(|media| media.schema.as_ref())
        {
            let context = format!("error response {status}");
            let object = resolve_schema(category, &context, schema_or_ref)?;
            return Ok(Some(object));
        }
    }
    Ok(None)
}

fn is_success(status: &StatusCode) -> bool {
    matches!(status, StatusCode::Code(200..=299) | StatusCode::Range(2))
}
