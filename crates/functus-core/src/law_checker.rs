//! 圏論的法則の静的検証 (`LawChecker`)。
//!
//! `Category` の変更系メソッド(`compose` 等)は不正な操作をその場で拒否するが、
//! `LawChecker` は読み取り専用の検証をひとまとまりの API として公開し、
//! 生成前の事前チェック(design/04-components.md 4.6 の `check` サブコマンド)から
//! `Category` の変更を伴わずに同じ判定を呼べるようにする。
//! `LawChecker` は `Category` の公開 API(および `resolve_composition`)のみに
//! 依存し、`Category` 側は `LawChecker` に依存しない一方向の関係を保つ。

use std::collections::{HashMap, HashSet};

use crate::category::{Category, CategoryError};
use crate::morphism::MorphismId;
use crate::object::{ObjectId, ObjectKind};

/// 圏論的法則の静的検証をまとめた名前空間。状態を持たない。
pub struct LawChecker;

impl LawChecker {
    /// `f: A -> B` と `g: B -> C` が合成可能かを、実際には合成射を登録せずに検証する。
    ///
    /// `Category::compose` が使う判定基準(域・余域の一致に加えて効果の互換性)と
    /// 完全に同一のロジックを共有するため、このチェックが `Ok` を返したのに
    /// `compose` が失敗する、という事前チェックと実処理の乖離は起きない。
    ///
    /// # Errors
    ///
    /// `f` / `g` が未登録の場合、`f` の余域と `g` の域が一致しない場合、
    /// または `f` と `g` の効果が非互換な場合に失敗する。
    pub fn check_composable(
        category: &Category,
        f: &MorphismId,
        g: &MorphismId,
    ) -> Result<(), CategoryError> {
        category.resolve_composition(f, g).map(|_| ())
    }

    /// 余積 `coproduct` のすべてのバリアントが、それを消費する基本射
    /// (`dom` がそのバリアントの対象である `MorphismId::Named` の射)を
    /// 少なくとも1つ持つかを検証する。
    ///
    /// docs/design/03-categorical-ir.md 3.3 の「許可される遷移のみを射として定義」に対応し、
    /// 状態集合(余積)の一部の分岐が誰にも処理されないまま放置されるモデルを検出する。
    ///
    /// `identity` / `compose` が生成する派生射(`MorphismId::Identity` /
    /// `MorphismId::Composed`)は候補から除外する。恒等射は no-op であり
    /// 状態遷移を処理したことにならないため、これを候補に含めると
    /// 「対象を作っただけで何も処理していない」モデルを誤って合格させてしまう。
    ///
    /// この判定は「バリアントのペイロード対象を `dom` に取る射があるか」という
    /// ヒューリスティックであり、その射が本当にそのバリアントを処理する意図で
    /// 書かれたかまでは検証できない。余積と無関係な射がたまたま同じペイロード
    /// 対象を `dom` に取っている場合、誤って「処理済み」と判定される既知の限界が
    /// ある。射がどの余積のどのバリアントを消費するかを IR が明示的に表現できる
    /// ようになるまでの暫定実装であり、crates/functus-core/CLAUDE.md に記録している。
    ///
    /// # Errors
    ///
    /// `coproduct` が未登録の場合、余積でない場合、複数のバリアントが同じ
    /// ペイロード対象を共有していて判定不能な場合、または処理する射を
    /// 持たないバリアントが存在する場合に失敗する。
    pub fn check_coproduct_variants_are_consumed(
        category: &Category,
        coproduct: &ObjectId,
    ) -> Result<(), CategoryError> {
        let object = category
            .object(coproduct)
            .ok_or_else(|| CategoryError::UnknownObject(coproduct.clone()))?;
        let ObjectKind::Coproduct(variants) = &object.kind else {
            return Err(CategoryError::NotACoproduct(coproduct.clone()));
        };

        // 同じペイロード対象を複数のバリアントが共有していると、どのバリアントが
        // 処理されたか一意に決められない。
        let mut payload_owners: HashMap<&ObjectId, &str> = HashMap::new();
        for (name, payload) in variants {
            if payload_owners.insert(payload, name.as_str()).is_some() {
                return Err(CategoryError::AmbiguousCoproductVariants {
                    coproduct: coproduct.clone(),
                    payload: payload.clone(),
                });
            }
        }

        let consumed_by_named_morphism: HashSet<&ObjectId> = category
            .morphisms()
            .filter(|morphism| matches!(morphism.id, MorphismId::Named(_)))
            .map(|morphism| &morphism.dom)
            .collect();

        let missing_variants: Vec<String> = variants
            .iter()
            .filter(|(_, payload)| !consumed_by_named_morphism.contains(payload))
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
    use crate::effect::{Effect, EffectStack};
    use crate::object::Object;

    /// docs/design/03-categorical-ir.md 3.3 の `UserFetch = Idle | Loading | Success(User) | Failure(ApiError)`
    /// を組み立てるヘルパー。バリアントのペイロード対象はすべて異なる
    /// (Idle/Loading 用のダミー対象を分けている)ため `AmbiguousCoproductVariants` にはならない。
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

    /// rust-reviewer が指摘した回帰: 事前チェックと `compose` は同じ理由で
    /// 失敗しなければならない。ドムコッドは一致するが効果が非互換な組で、
    /// 事前チェックが `Ok` を返すのに `compose` が失敗する、という乖離を防ぐ。
    #[test]
    fn check_composable_and_compose_agree_on_incompatible_effects() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let b = category.add_object(Object::scalar("B")).unwrap();
        let c = category.add_object(Object::scalar("C")).unwrap();
        let err_obj = category.add_object(Object::scalar("Err")).unwrap();
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
                EffectStack::wrapping(vec![Effect::Fallible { error: err_obj }]),
            )
            .unwrap();

        let check_result = LawChecker::check_composable(&category, &f, &g);
        let compose_result = category.compose(&f, &g);
        assert!(check_result.is_err());
        assert_eq!(check_result.unwrap_err(), compose_result.unwrap_err());
    }

    #[test]
    fn check_coproduct_coverage_rejects_non_coproduct() {
        let mut category = Category::new();
        let a = category.add_object(Object::scalar("A")).unwrap();
        let err = LawChecker::check_coproduct_variants_are_consumed(&category, &a).unwrap_err();
        assert_eq!(err, CategoryError::NotACoproduct(ObjectId::from("A")));
    }

    #[test]
    fn check_coproduct_coverage_reports_all_missing_variants_in_declared_order() {
        let (category, user_fetch) = user_fetch_category();
        let err =
            LawChecker::check_coproduct_variants_are_consumed(&category, &user_fetch).unwrap_err();
        assert_eq!(
            err,
            CategoryError::UnhandledCoproductVariants {
                coproduct: user_fetch,
                missing_variants: vec![
                    "Idle".to_string(),
                    "Loading".to_string(),
                    "Success".to_string(),
                    "Failure".to_string(),
                ],
            }
        );
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
        let err =
            LawChecker::check_coproduct_variants_are_consumed(&category, &user_fetch).unwrap_err();
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

        LawChecker::check_coproduct_variants_are_consumed(&category, &user_fetch).unwrap();
    }

    /// Critical回帰: 恒等射しか無い状態は「処理済み」とみなしてはならない。
    /// 恒等射は no-op であり、状態遷移を何も処理していない。
    #[test]
    fn identity_morphism_alone_does_not_count_as_handling_a_variant() {
        let (mut category, user_fetch) = user_fetch_category();
        // バリアント名ではなく、各バリアントのペイロード対象 (Idle/Loading/User/ApiError)
        // に対して恒等射を作る。
        for payload in ["Idle", "Loading", "User", "ApiError"] {
            category.identity(&ObjectId::from(payload)).unwrap();
        }
        let err =
            LawChecker::check_coproduct_variants_are_consumed(&category, &user_fetch).unwrap_err();
        assert!(matches!(
            err,
            CategoryError::UnhandledCoproductVariants { .. }
        ));
    }

    /// Critical回帰: 複数のバリアントが同じペイロード対象を共有する場合、
    /// 1本の射で全部が「処理済み」になってしまう危険を検出して拒否する。
    #[test]
    fn coproduct_with_shared_payload_is_rejected_as_ambiguous() {
        let mut category = Category::new();
        let unit = category.add_object(Object::scalar("Unit")).unwrap();
        let coproduct = category
            .add_object(Object::coproduct(
                "Toggle",
                vec![("On".to_string(), unit.clone()), ("Off".to_string(), unit)],
            ))
            .unwrap();

        let err =
            LawChecker::check_coproduct_variants_are_consumed(&category, &coproduct).unwrap_err();
        assert_eq!(
            err,
            CategoryError::AmbiguousCoproductVariants {
                coproduct,
                payload: ObjectId::from("Unit"),
            }
        );
    }
}
