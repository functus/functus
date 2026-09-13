//! 対象と射の集まりである圏 (Category)。射の合成と型整合チェックを提供する。

use std::collections::{HashMap, HashSet};

use crate::effect::{Effect, EffectStack};
use crate::morphism::{Morphism, MorphismId};
use crate::object::{Object, ObjectId, ObjectKind};

/// 圏の操作が失敗したときのエラー。
///
/// 新しい失敗種別を今後追加する見込みが高いため、下流に網羅的な `match` を
/// 強制しないよう `#[non_exhaustive]` を付けている。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CategoryError {
    /// 参照された対象が `add_object` で登録されていない。
    #[error("対象 `{0}` は登録されていない")]
    UnknownObject(ObjectId),

    /// 参照された射が登録されていない。
    #[error("射 `{0}` は登録されていない")]
    UnknownMorphism(MorphismId),

    /// `compose(f, g)` で `f` の余域と `g` の域が一致しない。
    #[error("合成できない: `{f}` の余域 `{f_cod}` と `{g}` の域 `{g_dom}` が一致しない")]
    DomCodMismatch {
        /// 合成の左側の射。
        f: MorphismId,
        /// `f` の余域。
        f_cod: ObjectId,
        /// 合成の右側の射。
        g: MorphismId,
        /// `g` の域。
        g_dom: ObjectId,
    },

    /// 同じ ID の射がすでに登録されている。
    #[error("射 `{0}` は既に登録されている")]
    DuplicateMorphism(MorphismId),

    /// 同じ ID の対象がすでに登録されている。
    #[error("対象 `{0}` は既に登録されている")]
    DuplicateObject(ObjectId),

    /// 積・余積のフィールド名が対象内で重複している。
    #[error("対象 `{object}` のフィールド名 `{field}` が重複している")]
    DuplicateField {
        /// フィールド重複が起きた対象。
        object: ObjectId,
        /// 重複しているフィールド名。
        field: String,
    },

    /// 積・余積が、まだ登録されていない対象を参照している。
    #[error("対象 `{object}` はまだ登録されていない対象 `{missing}` を参照している")]
    DanglingFieldReference {
        /// 参照元の対象。
        object: ObjectId,
        /// まだ登録されていない参照先。
        missing: ObjectId,
    },

    /// `compose(f, g)` で `f` と `g` の双方が異なる効果を持つ。Kleisli 圏の bind は
    /// 同一モナドの下でのみ定義されるため、異なるモナドを跨ぐ合成(lift)なしには
    /// 一意な効果を決定できない。
    #[error("合成できない: `{f}` の効果 `{f_effects}` と `{g}` の効果 `{g_effects}` が異なる")]
    IncompatibleEffects {
        /// 合成の左側の射。
        f: MorphismId,
        /// `f` の効果。
        f_effects: EffectStack,
        /// 合成の右側の射。
        g: MorphismId,
        /// `g` の効果。
        g_effects: EffectStack,
    },
}

/// 対象と射の集まり。合成は既存の射から新しい射を導出し、圏に登録する。
#[derive(Debug, Default)]
pub struct Category {
    objects: HashMap<ObjectId, Object>,
    morphisms: HashMap<MorphismId, Morphism>,
}

impl Category {
    /// 空の圏を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 対象を登録する。積・余積が参照する対象は先に登録しておく必要がある
    /// (前方参照は許可しない)。
    ///
    /// # Errors
    ///
    /// 同じ ID の対象がすでに登録されている場合、フィールド名が重複している場合、
    /// またはフィールドが未登録の対象を参照している場合に失敗する。
    pub fn add_object(&mut self, object: Object) -> Result<ObjectId, CategoryError> {
        if self.objects.contains_key(&object.id) {
            return Err(CategoryError::DuplicateObject(object.id));
        }
        if let ObjectKind::Product(fields) | ObjectKind::Coproduct(fields) = &object.kind {
            let mut seen = HashSet::new();
            for (field, target) in fields {
                if !seen.insert(field) {
                    return Err(CategoryError::DuplicateField {
                        object: object.id,
                        field: field.clone(),
                    });
                }
                if !self.objects.contains_key(target) {
                    return Err(CategoryError::DanglingFieldReference {
                        object: object.id,
                        missing: target.clone(),
                    });
                }
            }
        }
        let id = object.id.clone();
        self.objects.insert(id.clone(), object);
        Ok(id)
    }

    /// 登録済みの対象を参照する。
    pub fn object(&self, id: &ObjectId) -> Option<&Object> {
        self.objects.get(id)
    }

    /// 登録済みの射を参照する。
    pub fn morphism(&self, id: &MorphismId) -> Option<&Morphism> {
        self.morphisms.get(id)
    }

    /// フロントエンド・DSL 由来の、効果を持たない基本射を登録する。
    /// `dom` / `cod` は事前に `add_object` 済みであること。
    ///
    /// # Errors
    ///
    /// `dom` / `cod` が未登録の場合、または同じ ID の射がすでに登録されている場合に失敗する。
    pub fn add_primitive_morphism(
        &mut self,
        id: impl Into<String>,
        dom: ObjectId,
        cod: ObjectId,
    ) -> Result<MorphismId, CategoryError> {
        self.add_effectful_primitive_morphism(id, dom, cod, EffectStack::pure())
    }

    /// フロントエンド・DSL 由来の基本射を、`cod` に付与する効果とともに登録する。
    /// `dom` / `cod` は事前に `add_object` 済みであること。
    ///
    /// # Errors
    ///
    /// `dom` / `cod` が未登録の場合、または同じ ID の射がすでに登録されている場合に失敗する。
    pub fn add_effectful_primitive_morphism(
        &mut self,
        id: impl Into<String>,
        dom: ObjectId,
        cod: ObjectId,
        effects: EffectStack,
    ) -> Result<MorphismId, CategoryError> {
        self.require_object(&dom)?;
        self.require_object(&cod)?;
        for effect in effects.layers() {
            if let Effect::Fallible { error } = effect {
                self.require_object(error)?;
            }
        }
        let id = MorphismId::named(id);
        if self.morphisms.contains_key(&id) {
            return Err(CategoryError::DuplicateMorphism(id));
        }
        self.morphisms.insert(
            id.clone(),
            Morphism {
                id: id.clone(),
                dom,
                cod,
                effects,
            },
        );
        Ok(id)
    }

    /// 対象 `obj` 上の恒等射を返す。既に生成済みなら同じ ID を再利用する。
    ///
    /// # Errors
    ///
    /// `obj` が未登録の場合に失敗する。
    pub fn identity(&mut self, obj: &ObjectId) -> Result<MorphismId, CategoryError> {
        self.require_object(obj)?;
        let id = MorphismId::Identity(obj.clone());
        self.morphisms
            .entry(id.clone())
            .or_insert_with(|| Morphism {
                id: id.clone(),
                dom: obj.clone(),
                cod: obj.clone(),
                effects: EffectStack::pure(),
            });
        Ok(id)
    }

    /// `f: A -> B` と `g: B -> C` を図式順(`f` を先に適用)で合成し `A -> C` の射を返す。
    /// 数学的な `f∘g` ではなく `g∘f` に相当する。
    ///
    /// 恒等射との合成は単位律により相手の射をそのまま返す。それ以外は
    /// 構成要素を平坦化した `MorphismId::Composed` として登録するため、
    /// 結合律 `compose(compose(f, g), h) == compose(f, compose(g, h))` が
    /// 構造的に成り立つ。
    ///
    /// # Errors
    ///
    /// `f` / `g` が未登録の場合、または `f` の余域と `g` の域が一致しない場合に失敗する。
    pub fn compose(&mut self, f: &MorphismId, g: &MorphismId) -> Result<MorphismId, CategoryError> {
        let f_morphism = self.require_morphism(f)?;
        let g_morphism = self.require_morphism(g)?;

        if f_morphism.cod != g_morphism.dom {
            return Err(CategoryError::DomCodMismatch {
                f: f.clone(),
                f_cod: f_morphism.cod.clone(),
                g: g.clone(),
                g_dom: g_morphism.dom.clone(),
            });
        }

        if matches!(f, MorphismId::Identity(_)) {
            return Ok(g.clone());
        }
        if matches!(g, MorphismId::Identity(_)) {
            return Ok(f.clone());
        }

        let effects = merge_effects(f, &f_morphism.effects, g, &g_morphism.effects)?;
        let dom = f_morphism.dom.clone();
        let cod = g_morphism.cod.clone();

        let mut parts = Vec::new();
        extend_with_parts(&mut parts, f);
        extend_with_parts(&mut parts, g);

        let id = MorphismId::Composed(parts);
        self.morphisms
            .entry(id.clone())
            .or_insert_with(|| Morphism {
                id: id.clone(),
                dom,
                cod,
                effects,
            });
        Ok(id)
    }

    fn require_object(&self, id: &ObjectId) -> Result<(), CategoryError> {
        if self.objects.contains_key(id) {
            Ok(())
        } else {
            Err(CategoryError::UnknownObject(id.clone()))
        }
    }

    fn require_morphism(&self, id: &MorphismId) -> Result<&Morphism, CategoryError> {
        self.morphisms
            .get(id)
            .ok_or_else(|| CategoryError::UnknownMorphism(id.clone()))
    }
}

/// `compose(f, g)` の効果の合成規則。
///
/// 純粋な射(効果なし)は相手の効果をそのまま通す。両方が効果を持つ場合は、
/// 同じモナドスタックであれば(bind によって単一の層に潰れるはずなので)
/// そのまま採用し、異なるスタックを持つ場合は自動では決定できないため
/// エラーにする。
fn merge_effects(
    f: &MorphismId,
    f_effects: &EffectStack,
    g: &MorphismId,
    g_effects: &EffectStack,
) -> Result<EffectStack, CategoryError> {
    if f_effects.is_pure() {
        Ok(g_effects.clone())
    } else if g_effects.is_pure() || f_effects == g_effects {
        Ok(f_effects.clone())
    } else {
        Err(CategoryError::IncompatibleEffects {
            f: f.clone(),
            f_effects: f_effects.clone(),
            g: g.clone(),
            g_effects: g_effects.clone(),
        })
    }
}

fn extend_with_parts(parts: &mut Vec<MorphismId>, id: &MorphismId) {
    match id {
        MorphismId::Composed(existing) => parts.extend(existing.iter().cloned()),
        MorphismId::Named(_) | MorphismId::Identity(_) => parts.push(id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;

    fn sample_category() -> (Category, ObjectId, ObjectId) {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
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
        assert_eq!(
            err,
            CategoryError::UnknownMorphism(MorphismId::from("missing"))
        );
    }

    #[test]
    fn compose_rejects_dom_cod_mismatch() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
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
        let a = category.add_object(Object::scalar("A")).unwrap();
        let err = category
            .add_primitive_morphism("f", a, ObjectId::from("Missing"))
            .unwrap_err();
        assert_eq!(err, CategoryError::UnknownObject(ObjectId::from("Missing")));
    }

    #[test]
    fn add_primitive_morphism_rejects_duplicate_id() {
        let (mut category, a, b) = sample_category();
        category
            .add_primitive_morphism("f", a.clone(), b.clone())
            .unwrap();
        let err = category.add_primitive_morphism("f", a, b).unwrap_err();
        assert_eq!(err, CategoryError::DuplicateMorphism(MorphismId::from("f")));
    }

    #[test]
    fn add_object_rejects_duplicate_id() {
        let mut category = Category::new();
        category.add_object(Object::scalar("A")).unwrap();
        let err = category.add_object(Object::scalar("A")).unwrap_err();
        assert_eq!(err, CategoryError::DuplicateObject(ObjectId::from("A")));
    }

    #[test]
    fn add_object_rejects_dangling_field_reference() {
        let mut category = Category::new();
        let err = category
            .add_object(Object::product(
                "User",
                vec![("id".to_string(), ObjectId::from("UserId"))],
            ))
            .unwrap_err();
        assert_eq!(
            err,
            CategoryError::DanglingFieldReference {
                object: ObjectId::from("User"),
                missing: ObjectId::from("UserId"),
            }
        );
    }

    #[test]
    fn add_object_rejects_duplicate_field_name() {
        let mut category = Category::new();
        category.add_object(Object::scalar("UserId")).unwrap();
        let err = category
            .add_object(Object::product(
                "User",
                vec![
                    ("id".to_string(), ObjectId::from("UserId")),
                    ("id".to_string(), ObjectId::from("UserId")),
                ],
            ))
            .unwrap_err();
        assert_eq!(
            err,
            CategoryError::DuplicateField {
                object: ObjectId::from("User"),
                field: "id".to_string(),
            }
        );
    }

    /// Codex / rust-reviewer が指摘した回帰: 合成規則が生成する ID (`f;g` 相当) と
    /// 同じ見た目の名前を持つ基本射を登録しても、別の射として扱われなければならない。
    #[test]
    fn compose_does_not_alias_a_primitive_named_like_a_composed_id() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
        let z = category.add_object(Object::scalar("Z")).unwrap();
        let f = category.add_primitive_morphism("f", a, b.clone()).unwrap();
        let g = category.add_primitive_morphism("g", b, c).unwrap();
        category
            .add_primitive_morphism("f;g", z.clone(), z)
            .unwrap();

        let fg = category.compose(&f, &g).unwrap();
        let composed = category.morphism(&fg).unwrap();
        assert_eq!(composed.dom.as_str(), "A");
        assert_eq!(composed.cod.as_str(), "C");
    }

    /// Codex / rust-reviewer が指摘した回帰: 恒等射の内部 ID (`id[P]` 相当) と
    /// 同じ見た目の名前を持つ基本射を登録しても、単位律の判定に影響してはならない。
    #[test]
    fn identity_does_not_alias_a_primitive_named_like_an_identity_id() {
        let mut category = Category::new();
        let p = category.add_object(Object::scalar("P")).unwrap();
        let q = category.add_object(Object::scalar("Q")).unwrap();
        let z = category.add_object(Object::scalar("Z")).unwrap();
        // "id[P]" という名前の無関係な基本射 (Z -> Z) を先に登録しておく。
        // 文字列ベースの命名規則に依存した実装なら、これが identity(P) と
        // 衝突して単位律を壊す。
        category
            .add_primitive_morphism("id[P]", z.clone(), z)
            .unwrap();
        let f = category.add_primitive_morphism("f", p.clone(), q).unwrap();
        let id_p = category.identity(&p).unwrap();
        assert_eq!(category.compose(&id_p, &f).unwrap(), f);
    }

    /// issue #15 のゴール: `GET /users/{id}` が
    /// `UserId -> Async<Result<User, ApiError>>` として IR 上で表現できること。
    #[test]
    fn effectful_primitive_morphism_renders_as_async_result() {
        let mut category = Category::new();
        let user_id = category.add_object(Object::scalar("UserId")).unwrap();
        let user = category.add_object(Object::scalar("User")).unwrap();
        let api_error = category.add_object(Object::scalar("ApiError")).unwrap();

        let get_user = category
            .add_effectful_primitive_morphism(
                "getUser",
                user_id.clone(),
                user.clone(),
                EffectStack::wrapping(vec![
                    Effect::Async,
                    Effect::Fallible {
                        error: api_error.clone(),
                    },
                ]),
            )
            .unwrap();

        let morphism = category.morphism(&get_user).unwrap();
        // IR の構造そのもの(域・余域・効果の層)を検証する。書式(render の出力文字列)は
        // effect.rs の render_nests_outer_to_inner が別途担当する。
        assert_eq!(morphism.dom, user_id);
        assert_eq!(morphism.cod, user);
        assert_eq!(
            morphism.effects.layers(),
            &[Effect::Async, Effect::Fallible { error: api_error }]
        );
        assert_eq!(
            morphism.effects.render(&morphism.cod),
            "Async<Result<User, ApiError>>"
        );
    }

    /// Codex が指摘した回帰: `Fallible` のエラー対象は `dom`/`cod` と同様に
    /// 未登録なら拒否されなければならない。放置すると、後段のジェネレータが
    /// 解決できない対象を参照する不正な IR が組み立てられてしまう。
    #[test]
    fn add_effectful_primitive_morphism_rejects_unregistered_error_object() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let err = category
            .add_effectful_primitive_morphism(
                "f",
                a,
                b,
                EffectStack::wrapping(vec![Effect::Fallible {
                    error: ObjectId::from("Missing"),
                }]),
            )
            .unwrap_err();
        assert_eq!(err, CategoryError::UnknownObject(ObjectId::from("Missing")));
    }

    #[test]
    fn compose_with_a_pure_morphism_preserves_the_other_sides_effects() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();

        let effects = EffectStack::wrapping(vec![Effect::Async]);
        let f = category
            .add_effectful_primitive_morphism("f", a, b.clone(), effects.clone())
            .unwrap();
        let g = category.add_primitive_morphism("g", b, c).unwrap();

        let fg = category.compose(&f, &g).unwrap();
        assert_eq!(category.morphism(&fg).unwrap().effects, effects);
    }

    #[test]
    fn compose_rejects_incompatible_effects() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
        let err = category.add_object(Object::scalar("Err")).unwrap();

        let f = category
            .add_effectful_primitive_morphism(
                "f",
                a,
                b.clone(),
                EffectStack::wrapping(vec![Effect::Async]),
            )
            .unwrap();
        let g = category
            .add_effectful_primitive_morphism(
                "g",
                b,
                c,
                EffectStack::wrapping(vec![Effect::Fallible { error: err }]),
            )
            .unwrap();

        let err = category.compose(&f, &g).unwrap_err();
        assert!(matches!(err, CategoryError::IncompatibleEffects { .. }));
    }
}
