//! 圏論的法則の静的検証 (`LawChecker`)。
//!
//! `Category` の変更系メソッド(`compose` 等)は不正な操作をその場で拒否するが、
//! `LawChecker` は読み取り専用の検証をひとまとまりの API として公開し、
//! 生成前の事前チェック(design/04-components.md 4.6 の `check` サブコマンド)や、
//! `Category` の変更系メソッドの内部からも共通のルールとして再利用できるようにする。

use crate::category::{Category, CategoryError};
use crate::morphism::MorphismId;
use crate::object::{ObjectId, ObjectKind};

/// 圏論的法則の静的検証をまとめた名前空間。状態を持たない。
pub struct LawChecker;

impl LawChecker {
    /// `f: A -> B` と `g: B -> C` が合成可能か(`f` の余域と `g` の域が一致するか)を検証する。
    ///
    /// # Errors
    ///
    /// `f` / `g` が未登録の場合、または `f` の余域と `g` の域が一致しない場合に失敗する。
    pub fn check_composable(
        category: &Category,
        f: &MorphismId,
        g: &MorphismId,
    ) -> Result<(), CategoryError> {
        let f_morphism = category
            .morphism(f)
            .ok_or_else(|| CategoryError::UnknownMorphism(f.clone()))?;
        let g_morphism = category
            .morphism(g)
            .ok_or_else(|| CategoryError::UnknownMorphism(g.clone()))?;
        if f_morphism.cod != g_morphism.dom {
            return Err(CategoryError::DomCodMismatch {
                f: f.clone(),
                f_cod: f_morphism.cod.clone(),
                g: g.clone(),
                g_dom: g_morphism.dom.clone(),
            });
        }
        Ok(())
    }

    /// 余積 `coproduct` のすべてのバリアントが、それを消費する射(`dom` がそのバリアントの
    /// 対象である射)を少なくとも1つ持つかを検証する。
    ///
    /// docs/design/03-categorical-ir.md 3.3 の「許可される遷移のみを射として定義」に対応し、
    /// 状態集合(余積)の一部の分岐が誰にも処理されないまま放置されるモデルを検出する。
    ///
    /// # Errors
    ///
    /// `coproduct` が未登録、または余積でない場合、あるいは処理する射を持たない
    /// バリアントが存在する場合に失敗する。
    pub fn check_coproduct_coverage(
        category: &Category,
        coproduct: &ObjectId,
    ) -> Result<(), CategoryError> {
        let object = category
            .object(coproduct)
            .ok_or_else(|| CategoryError::UnknownObject(coproduct.clone()))?;
        let ObjectKind::Coproduct(variants) = &object.kind else {
            return Err(CategoryError::NotACoproduct(coproduct.clone()));
        };

        let missing_variants: Vec<String> = variants
            .iter()
            .filter(|(_, variant_object)| {
                !category
                    .morphisms()
                    .any(|morphism| morphism.dom == *variant_object)
            })
            .map(|(name, _)| name.clone())
            .collect();

        if missing_variants.is_empty() {
            Ok(())
        } else {
            Err(CategoryError::UnhandledCoproductVariants {
                coproduct: coproduct.clone(),
                missing_variants,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Object;

    /// docs/design/03-categorical-ir.md 3.3 の `UserFetch = Idle | Loading | Success(User) | Failure(ApiError)`
    /// を組み立てるヘルパー。
    fn user_fetch_category() -> (Category, ObjectId) {
        let mut category = Category::new();
        let idle = category.add_object(Object::scalar("Idle")).unwrap();
        let loading = category.add_object(Object::scalar("Loading")).unwrap();
        let user = category.add_object(Object::scalar("User")).unwrap();
        let api_error = category.add_object(Object::scalar("ApiError")).unwrap();
        let user_fetch = category
            .add_object(Object::coproduct(
                "UserFetch",
                vec![
                    ("Idle".to_string(), idle),
                    ("Loading".to_string(), loading),
                    ("Success".to_string(), user),
                    ("Failure".to_string(), api_error),
                ],
            ))
            .unwrap();
        (category, user_fetch)
    }

    #[test]
    fn check_composable_rejects_dom_cod_mismatch() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
        let f = category.add_primitive_morphism("f", a, b).unwrap();
        let h = category.add_primitive_morphism("h", c.clone(), c).unwrap();

        let err = LawChecker::check_composable(&category, &f, &h).unwrap_err();
        assert!(matches!(err, CategoryError::DomCodMismatch { .. }));
    }

    #[test]
    fn check_composable_accepts_a_matching_pair_without_registering_a_composed_morphism() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
        let f = category.add_primitive_morphism("f", a, b.clone()).unwrap();
        let g = category.add_primitive_morphism("g", b, c).unwrap();

        LawChecker::check_composable(&category, &f, &g).unwrap();
        // 読み取り専用のチェックなので、実際の合成射 (f;g) は登録されない。
        assert_eq!(category.morphisms().count(), 2);
    }

    #[test]
    fn check_coproduct_coverage_rejects_non_coproduct() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let err = LawChecker::check_coproduct_coverage(&category, &a).unwrap_err();
        assert_eq!(err, CategoryError::NotACoproduct(ObjectId::from("A")));
    }

    #[test]
    fn check_coproduct_coverage_reports_all_missing_variants() {
        let (category, user_fetch) = user_fetch_category();
        let err = LawChecker::check_coproduct_coverage(&category, &user_fetch).unwrap_err();
        match err {
            CategoryError::UnhandledCoproductVariants {
                coproduct,
                mut missing_variants,
            } => {
                assert_eq!(coproduct, user_fetch);
                missing_variants.sort();
                assert_eq!(
                    missing_variants,
                    vec!["Failure", "Idle", "Loading", "Success"]
                );
            }
            other => panic!("UnhandledCoproductVariants を期待したが {other:?} だった"),
        }
    }

    #[test]
    fn check_coproduct_coverage_passes_once_every_variant_has_a_handler() {
        let (mut category, user_fetch) = user_fetch_category();
        let idle = ObjectId::from("Idle");
        let loading = ObjectId::from("Loading");
        let user = ObjectId::from("User");
        let api_error = ObjectId::from("ApiError");

        category
            .add_primitive_morphism("startLoading", idle, loading.clone())
            .unwrap();
        category
            .add_primitive_morphism("onSuccess", user, loading.clone())
            .unwrap();
        category
            .add_primitive_morphism("onFailure", api_error, loading)
            .unwrap();

        // Loading だけがまだ処理されていない。
        let err = LawChecker::check_coproduct_coverage(&category, &user_fetch).unwrap_err();
        assert_eq!(
            err,
            CategoryError::UnhandledCoproductVariants {
                coproduct: user_fetch.clone(),
                missing_variants: vec!["Loading".to_string()],
            }
        );

        category
            .add_primitive_morphism("retry", ObjectId::from("Loading"), ObjectId::from("Idle"))
            .unwrap();

        LawChecker::check_coproduct_coverage(&category, &user_fetch).unwrap();
    }
}
