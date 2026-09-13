//! 圏論的法則のテスト。issue #14 のゴール
//! 「単体テストで合成の結合律・単位律が検証されること」に対応する。

use functus_core::{Category, CategoryError, Effect, EffectStack, MorphismId, Object};
use proptest::prelude::*;

fn chain_category() -> Result<(Category, MorphismId, MorphismId, MorphismId), CategoryError> {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"))?;
    let b = category.add_object(Object::scalar("B"))?;
    let c = category.add_object(Object::scalar("C"))?;
    let d = category.add_object(Object::scalar("D"))?;

    let f = category.add_primitive_morphism("f", a, b.clone())?;
    let g = category.add_primitive_morphism("g", b, c.clone())?;
    let h = category.add_primitive_morphism("h", c, d)?;
    Ok((category, f, g, h))
}

#[test]
fn composition_is_associative() -> Result<(), CategoryError> {
    let (mut category, f, g, h) = chain_category()?;

    let fg = category.compose(&f, &g)?;
    let left = category.compose(&fg, &h)?;

    let gh = category.compose(&g, &h)?;
    let right = category.compose(&f, &gh)?;

    assert_eq!(left, right, "(f;g);h と f;(g;h) は同じ射に構成されるべき");
    assert_eq!(
        left,
        MorphismId::Composed(vec![f.clone(), g.clone(), h.clone()]),
        "結合律で得られる射は f,g,h を合成順に平坦化したものであるべき"
    );

    let composed = category
        .morphism(&left)
        .expect("compose が返した ID は必ず登録されている");
    let f_dom = category
        .morphism(&f)
        .expect("f は chain_category で登録済み")
        .dom
        .clone();
    let h_cod = category
        .morphism(&h)
        .expect("h は chain_category で登録済み")
        .cod
        .clone();
    assert_eq!(composed.dom, f_dom, "合成後の域は f の域と一致するべき");
    assert_eq!(composed.cod, h_cod, "合成後の余域は h の余域と一致するべき");
    Ok(())
}

#[test]
fn identity_is_left_and_right_unit() -> Result<(), CategoryError> {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"))?;
    let b = category.add_object(Object::scalar("B"))?;
    let f = category.add_primitive_morphism("f", a.clone(), b.clone())?;

    let id_a = category.identity(&a)?;
    let id_b = category.identity(&b)?;

    let left_unit = category.compose(&id_a, &f)?;
    let right_unit = category.compose(&f, &id_b)?;

    assert_eq!(left_unit, f, "id_A ; f は f と同じ射であるべき");
    assert_eq!(right_unit, f, "f ; id_B は f と同じ射であるべき");
    Ok(())
}

/// 恒等射との合成は ID だけでなく効果も保存しなければならない。
/// `compose` の恒等射ショートカットは `merge_effects` より手前にあるため、
/// この性質は実装の配置に依存せず常に成り立つべき単位律の一部として固定する。
#[test]
fn identity_composition_preserves_effects() -> Result<(), CategoryError> {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"))?;
    let b = category.add_object(Object::scalar("B"))?;
    let err = category.add_object(Object::scalar("Err"))?;
    let effects = EffectStack::wrapping(vec![Effect::Fallible { error: err }]);
    let f =
        category.add_effectful_primitive_morphism("f", a.clone(), b.clone(), effects.clone())?;

    let id_a = category.identity(&a)?;
    let id_b = category.identity(&b)?;

    let left_unit = category.compose(&id_a, &f)?;
    let right_unit = category.compose(&f, &id_b)?;

    assert_eq!(category.morphism(&left_unit).unwrap().effects, effects);
    assert_eq!(category.morphism(&right_unit).unwrap().effects, effects);
    Ok(())
}

/// `EffectStack::pure()` は効果合成の両側単位元である。
#[test]
fn pure_effects_are_the_unit_of_composition() -> Result<(), CategoryError> {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"))?;
    let b = category.add_object(Object::scalar("B"))?;
    let c = category.add_object(Object::scalar("C"))?;
    let effects = EffectStack::wrapping(vec![Effect::Async]);

    let f = category.add_effectful_primitive_morphism("f", a, b.clone(), effects.clone())?;
    let g = category.add_primitive_morphism("g", b, c)?;

    let fg = category.compose(&f, &g)?;
    assert_eq!(
        category.morphism(&fg).unwrap().effects,
        effects,
        "純粋な射との合成は相手の効果をそのまま保存するべき"
    );
    Ok(())
}

/// 効果の合成は結合律を満たす: 純粋な射と効果付きの射が混在する鎖でも、
/// (f;g);h と f;(g;h) は同じ効果に落ち着く。
#[test]
fn effect_merge_is_associative_across_a_mixed_chain() -> Result<(), CategoryError> {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"))?;
    let b = category.add_object(Object::scalar("B"))?;
    let c = category.add_object(Object::scalar("C"))?;
    let d = category.add_object(Object::scalar("D"))?;
    let err = category.add_object(Object::scalar("Err"))?;

    let effects = EffectStack::wrapping(vec![Effect::Async, Effect::Fallible { error: err }]);
    // f だけが効果を持ち、g・h は純粋。単位元(EffectStack::pure())を挟んでも
    // 結合の仕方によらず同じ効果に落ち着くことを確認する。
    let f = category.add_effectful_primitive_morphism("f", a, b.clone(), effects.clone())?;
    let g = category.add_primitive_morphism("g", b, c.clone())?;
    let h = category.add_primitive_morphism("h", c, d)?;

    let fg = category.compose(&f, &g)?;
    let left = category.compose(&fg, &h)?;

    let gh = category.compose(&g, &h)?;
    let right = category.compose(&f, &gh)?;

    assert_eq!(category.morphism(&left).unwrap().effects, effects);
    assert_eq!(category.morphism(&right).unwrap().effects, effects);
    Ok(())
}

// Codex / rust-reviewer が指摘した回帰: `;` や `id[...]` を含む名前を持つ
// 基本射を混ぜても、合成の結合律・単位律は構造的に壊れてはならない。
proptest! {
    #[test]
    fn composition_is_associative_even_with_adversarial_names(
        names in prop::collection::vec("(f;g|id\\[X\\]|[a-z]{1,4})", 3..6),
    ) {
        let mut category = Category::new();
        let mut objects = Vec::with_capacity(names.len() + 1);
        for i in 0..=names.len() {
            objects.push(category.add_object(Object::scalar(format!("O{i}"))).unwrap());
        }

        let mut morphisms = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let unique_name = format!("{name}#{i}");
            let m = category
                .add_primitive_morphism(unique_name, objects[i].clone(), objects[i + 1].clone())
                .unwrap();
            morphisms.push(m);
        }

        // 左結合: (((m0;m1);m2);...) と、右結合: (...;(m_{n-2};m_{n-1})) が
        // 同じ射に構成されることを確認する。
        let mut left = morphisms[0].clone();
        for m in &morphisms[1..] {
            left = category.compose(&left, m).unwrap();
        }

        let mut right = morphisms[morphisms.len() - 1].clone();
        for m in morphisms[..morphisms.len() - 1].iter().rev() {
            right = category.compose(m, &right).unwrap();
        }

        prop_assert_eq!(left, MorphismId::Composed(morphisms));
        prop_assert_eq!(&category.morphism(&right).unwrap().dom, &objects[0]);
        prop_assert_eq!(&category.morphism(&right).unwrap().cod, objects.last().unwrap());
    }
}
