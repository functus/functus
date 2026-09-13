//! 対象と射の集まりである圏 (Category)。射の合成と型整合チェックを提供する。

use std::collections::HashMap;

use crate::morphism::{Morphism, MorphismId, MorphismKind};
use crate::object::{Object, ObjectId};

/// 圏の操作が失敗したときのエラー。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CategoryError {
    #[error("対象 `{0}` は登録されていない")]
    UnknownObject(String),

    #[error("射 `{0}` は登録されていない")]
    UnknownMorphism(String),

    #[error("合成できない: `{f}` の余域 `{f_cod}` と `{g}` の域 `{g_dom}` が一致しない")]
    DomCodMismatch {
        f: String,
        f_cod: String,
        g: String,
        g_dom: String,
    },
}

/// 対象と射の集まり。合成は既存の射から新しい射を導出し、圏に登録する。
#[derive(Debug, Default)]
pub struct Category {
    objects: HashMap<ObjectId, Object>,
    morphisms: HashMap<MorphismId, Morphism>,
}

impl Category {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_object(&mut self, object: Object) -> ObjectId {
        let id = object.id.clone();
        self.objects.insert(id.clone(), object);
        id
    }

    pub fn object(&self, id: &ObjectId) -> Option<&Object> {
        self.objects.get(id)
    }

    pub fn morphism(&self, id: &MorphismId) -> Option<&Morphism> {
        self.morphisms.get(id)
    }

    /// フロントエンド・DSL 由来の基本射を登録する。`dom` / `cod` は事前に `add_object` 済みであること。
    pub fn add_primitive_morphism(
        &mut self,
        id: impl Into<String>,
        dom: ObjectId,
        cod: ObjectId,
    ) -> Result<MorphismId, CategoryError> {
        self.require_object(&dom)?;
        self.require_object(&cod)?;
        let id = MorphismId::new(id);
        self.morphisms.insert(
            id.clone(),
            Morphism {
                id: id.clone(),
                dom,
                cod,
                kind: MorphismKind::Primitive,
            },
        );
        Ok(id)
    }

    /// 対象 `obj` 上の恒等射を返す。既に生成済みなら同じ ID を再利用する。
    pub fn identity(&mut self, obj: &ObjectId) -> Result<MorphismId, CategoryError> {
        self.require_object(obj)?;
        let id = MorphismId::new(format!("id[{}]", obj.as_str()));
        self.morphisms
            .entry(id.clone())
            .or_insert_with(|| Morphism {
                id: id.clone(),
                dom: obj.clone(),
                cod: obj.clone(),
                kind: MorphismKind::Identity,
            });
        Ok(id)
    }

    /// `f: A -> B` と `g: B -> C` を合成し `A -> C` の射を返す。
    ///
    /// 恒等射との合成は単位律により相手の射をそのまま返す。それ以外は
    /// 構成要素を平坦化した `MorphismKind::Composed` として登録するため、
    /// 結合律 `compose(compose(f, g), h) == compose(f, compose(g, h))` が
    /// 構造的に成り立つ。
    pub fn compose(&mut self, f: &MorphismId, g: &MorphismId) -> Result<MorphismId, CategoryError> {
        let f_morphism = self.require_morphism(f)?;
        let g_morphism = self.require_morphism(g)?;

        if f_morphism.cod != g_morphism.dom {
            return Err(CategoryError::DomCodMismatch {
                f: f.as_str().to_string(),
                f_cod: f_morphism.cod.as_str().to_string(),
                g: g.as_str().to_string(),
                g_dom: g_morphism.dom.as_str().to_string(),
            });
        }

        if matches!(f_morphism.kind, MorphismKind::Identity) {
            return Ok(g.clone());
        }
        if matches!(g_morphism.kind, MorphismKind::Identity) {
            return Ok(f.clone());
        }

        let dom = f_morphism.dom.clone();
        let cod = g_morphism.cod.clone();

        let mut parts = Vec::new();
        extend_with_parts(&mut parts, f, f_morphism);
        extend_with_parts(&mut parts, g, g_morphism);

        let id = MorphismId::new(
            parts
                .iter()
                .map(MorphismId::as_str)
                .collect::<Vec<_>>()
                .join(";"),
        );
        self.morphisms
            .entry(id.clone())
            .or_insert_with(|| Morphism {
                id: id.clone(),
                dom,
                cod,
                kind: MorphismKind::Composed(parts),
            });
        Ok(id)
    }

    fn require_object(&self, id: &ObjectId) -> Result<(), CategoryError> {
        if self.objects.contains_key(id) {
            Ok(())
        } else {
            Err(CategoryError::UnknownObject(id.as_str().to_string()))
        }
    }

    fn require_morphism(&self, id: &MorphismId) -> Result<&Morphism, CategoryError> {
        self.morphisms
            .get(id)
            .ok_or_else(|| CategoryError::UnknownMorphism(id.as_str().to_string()))
    }
}

fn extend_with_parts(parts: &mut Vec<MorphismId>, id: &MorphismId, morphism: &Morphism) {
    match &morphism.kind {
        MorphismKind::Composed(existing) => parts.extend(existing.iter().cloned()),
        _ => parts.push(id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_category() -> (Category, ObjectId, ObjectId) {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A"));
        let b = category.add_object(Object::scalar("B"));
        (category, a, b)
    }

    #[test]
    fn compose_rejects_unknown_morphism() {
        let (mut category, _a, _b) = sample_category();
        let err = category
            .compose(
                &MorphismId::from("missing"),
                &MorphismId::from("also-missing"),
            )
            .unwrap_err();
        assert_eq!(err, CategoryError::UnknownMorphism("missing".to_string()));
    }

    #[test]
    fn compose_rejects_dom_cod_mismatch() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A"));
        let b = category.add_object(Object::scalar("B"));
        let c = category.add_object(Object::scalar("C"));
        let f = category.add_primitive_morphism("f", a, b).unwrap();
        let h = category.add_primitive_morphism("h", c.clone(), c).unwrap();

        let err = category.compose(&f, &h).unwrap_err();
        assert!(matches!(err, CategoryError::DomCodMismatch { .. }));
    }

    #[test]
    fn identity_is_cached_per_object() {
        let (mut category, a, _b) = sample_category();
        let id1 = category.identity(&a).unwrap();
        let id2 = category.identity(&a).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn add_primitive_morphism_rejects_unknown_object() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A"));
        let err = category
            .add_primitive_morphism("f", a, ObjectId::from("Missing"))
            .unwrap_err();
        assert_eq!(err, CategoryError::UnknownObject("Missing".to_string()));
    }
}
