//! OpenAPI のスキーマを `functus_core::Object` に変換する。

use std::collections::HashSet;

use openapiv3::{
    ObjectType, OpenAPI, ReferenceOr, Schema, SchemaKind, Type, VariantOrUnknownOrEmpty,
};

use functus_core::{Category, Object, ObjectId};

use crate::error::FrontendError;

const SCHEMA_REF_PREFIX: &str = "#/components/schemas/";

/// `components.schemas` のすべての名前付きスキーマを対象として登録する。
///
/// Phase1 では各スキーマは `type: object` かつプロパティがスカラー型
/// (string/integer/number/boolean)、または他の `components.schemas` エントリへの
/// `$ref` のみをサポートする。ネストしたオブジェクト・配列・`oneOf`/`anyOf`/`allOf`
/// は未対応としてエラーにする。
///
/// `components.schemas` の宣言順はスキーマ間の依存関係(`$ref`)と一致するとは
/// 限らない。`Category::add_object` は前方参照を許可しないため、依存先が
/// まだ登録されていないスキーマは後回しにし、全体が進まなくなった時点で
/// 循環参照または未定義の参照として拒否する。
pub(crate) fn register_named_schemas(
    category: &mut Category,
    openapi: &OpenAPI,
) -> Result<(), FrontendError> {
    let Some(components) = &openapi.components else {
        return Ok(());
    };

    let known_names: HashSet<&str> = components.schemas.keys().map(String::as_str).collect();

    let mut pending = Vec::with_capacity(components.schemas.len());
    for (name, schema_or_ref) in &components.schemas {
        match schema_or_ref {
            ReferenceOr::Reference { reference } => {
                return Err(FrontendError::UnsupportedReference {
                    context: format!("components.schemas.{name}"),
                    reference: reference.clone(),
                });
            }
            ReferenceOr::Item(schema) => pending.push((name, schema)),
        }
    }

    while !pending.is_empty() {
        let mut next_pending = Vec::with_capacity(pending.len());
        let mut progressed = false;

        for (name, schema) in pending {
            let object_type = require_object_type(name, schema)?;

            // 依存先がまだ登録されていない場合は次のパスへ回す。全パスで
            // 進捗が無ければループの外で循環参照として報告する。
            if first_unregistered_dependency(category, &known_names, object_type).is_some() {
                next_pending.push((name, schema));
                continue;
            }

            let mut fields = Vec::with_capacity(object_type.properties.len());
            for (field_name, field_schema) in &object_type.properties {
                let field_object =
                    resolve_property_schema(category, name, field_name, field_schema)?;
                fields.push((field_name.clone(), field_object));
            }
            category.add_object(Object::product(name.clone(), fields))?;
            progressed = true;
        }

        if !progressed {
            let names: Vec<String> = next_pending
                .iter()
                .map(|(name, _)| (*name).clone())
                .collect();
            return Err(FrontendError::UnsupportedSchema {
                schema: names.join(", "),
                reason: "components.schemas 間で循環参照している、または依存先が解決できない"
                    .to_string(),
            });
        }
        pending = next_pending;
    }

    Ok(())
}

fn require_object_type<'a>(
    name: &str,
    schema: &'a Schema,
) -> Result<&'a ObjectType, FrontendError> {
    let SchemaKind::Type(Type::Object(object_type)) = &schema.schema_kind else {
        return Err(FrontendError::UnsupportedSchema {
            schema: name.to_string(),
            reason: "type: object 以外のトップレベルスキーマは Phase1 で未対応".to_string(),
        });
    };
    Ok(object_type)
}

/// `object_type` のプロパティのうち、`components.schemas` 内の別名を参照しているが
/// まだ `category` に登録されていないものがあれば、その名前を返す。
fn first_unregistered_dependency(
    category: &Category,
    known_names: &HashSet<&str>,
    object_type: &ObjectType,
) -> Option<String> {
    object_type.properties.values().find_map(|property| {
        let ReferenceOr::Reference { reference } = property else {
            return None;
        };
        let name = reference.strip_prefix(SCHEMA_REF_PREFIX)?;
        if known_names.contains(name) && category.object(&ObjectId::from(name)).is_none() {
            Some(name.to_string())
        } else {
            None
        }
    })
}

fn resolve_property_schema(
    category: &mut Category,
    schema_name: &str,
    field_name: &str,
    field_schema: &ReferenceOr<Box<Schema>>,
) -> Result<ObjectId, FrontendError> {
    match field_schema {
        ReferenceOr::Reference { reference } => {
            resolve_schema_ref(format!("{schema_name}.{field_name}"), reference)
        }
        ReferenceOr::Item(schema) => {
            resolve_inline_scalar(category, &format!("{schema_name}.{field_name}"), schema)
        }
    }
}

/// `ReferenceOr<Schema>` を対象 ID に解決する。パス parameter やレスポンスの
/// スキーマからも呼ばれる共有ロジック。
pub(crate) fn resolve_schema(
    category: &mut Category,
    context: &str,
    schema_or_ref: &ReferenceOr<Schema>,
) -> Result<ObjectId, FrontendError> {
    match schema_or_ref {
        ReferenceOr::Reference { reference } => resolve_schema_ref(context.to_string(), reference),
        ReferenceOr::Item(schema) => resolve_inline_scalar(category, context, schema),
    }
}

fn resolve_schema_ref(context: String, reference: &str) -> Result<ObjectId, FrontendError> {
    match reference.strip_prefix(SCHEMA_REF_PREFIX) {
        Some(name) if !name.is_empty() => Ok(ObjectId::from(name)),
        _ => Err(FrontendError::UnsupportedReference {
            context,
            reference: reference.to_string(),
        }),
    }
}

fn resolve_inline_scalar(
    category: &mut Category,
    context: &str,
    schema: &Schema,
) -> Result<ObjectId, FrontendError> {
    let SchemaKind::Type(ty) = &schema.schema_kind else {
        return Err(FrontendError::UnsupportedSchema {
            schema: context.to_string(),
            reason: "oneOf/anyOf/allOf/not は Phase1 で未対応".to_string(),
        });
    };

    let scalar_name = match ty {
        Type::String(string_type) => match &string_type.format {
            VariantOrUnknownOrEmpty::Unknown(format) if format == "uuid" => "Uuid",
            _ => "String",
        },
        Type::Integer(_) => "Integer",
        Type::Number(_) => "Number",
        Type::Boolean(_) => "Boolean",
        Type::Object(_) => {
            return Err(FrontendError::UnsupportedSchema {
                schema: context.to_string(),
                reason: "インラインのネストしたオブジェクトは Phase1 で未対応。\
                         components.schemas に名前を付けて $ref で参照すること"
                    .to_string(),
            });
        }
        Type::Array(_) => {
            return Err(FrontendError::UnsupportedSchema {
                schema: context.to_string(),
                reason: "配列型は Phase1 で未対応".to_string(),
            });
        }
    };

    Ok(intern_scalar(category, scalar_name))
}

/// スカラー対象を(未登録なら)登録して ID を返す。同名のスカラーは複数の
/// フィールドから共有される可能性があるため、既存なら登録し直さない。
pub(crate) fn intern_scalar(category: &mut Category, name: &str) -> ObjectId {
    let id = ObjectId::from(name);
    if category.object(&id).is_none() {
        // スカラー対象はフィールドを持たないため add_object は失敗しない。
        let _ = category.add_object(Object::scalar(name));
    }
    id
}
