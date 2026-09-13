//! 圏論的法則の単体テスト。issue #14 のゴール
//! 「単体テストで合成の結合律・単位律が検証されること」に対応する。

use functus_core::{Category, MorphismId, Object};

fn chain_category() -> (Category, MorphismId, MorphismId, MorphismId) {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"));
    let b = category.add_object(Object::scalar("B"));
    let c = category.add_object(Object::scalar("C"));
    let d = category.add_object(Object::scalar("D"));

    let f = category.add_primitive_morphism("f", a, b.clone()).unwrap();
    let g = category.add_primitive_morphism("g", b, c.clone()).unwrap();
    let h = category.add_primitive_morphism("h", c, d).unwrap();
    (category, f, g, h)
}

#[test]
fn composition_is_associative() {
    let (mut category, f, g, h) = chain_category();

    let fg = category.compose(&f, &g).unwrap();
    let left = category.compose(&fg, &h).unwrap();

    let gh = category.compose(&g, &h).unwrap();
    let right = category.compose(&f, &gh).unwrap();

    assert_eq!(left, right, "(f;g);h と f;(g;h) は同じ射に構成されるべき");

    let composed = category.morphism(&left).unwrap();
    let f_dom = category.morphism(&f).unwrap().dom.clone();
    let h_cod = category.morphism(&h).unwrap().cod.clone();
    assert_eq!(composed.dom, f_dom, "合成後の域は f の域と一致するべき");
    assert_eq!(composed.cod, h_cod, "合成後の余域は h の余域と一致するべき");
}

#[test]
fn identity_is_left_and_right_unit() {
    let mut category = Category::new();
    let a = category.add_object(Object::scalar("A"));
    let b = category.add_object(Object::scalar("B"));
    let f = category
        .add_primitive_morphism("f", a.clone(), b.clone())
        .unwrap();

    let id_a = category.identity(&a).unwrap();
    let id_b = category.identity(&b).unwrap();

    let left_unit = category.compose(&id_a, &f).unwrap();
    let right_unit = category.compose(&f, &id_b).unwrap();

    assert_eq!(left_unit, f, "id_A ; f は f と同じ射であるべき");
    assert_eq!(right_unit, f, "f ; id_B は f と同じ射であるべき");
}
