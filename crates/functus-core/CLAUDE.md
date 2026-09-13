# functus-core

圏論的 IR を定義するクレート。依存方向は `.claude/rules/workspace-layout.md` に従う(workspace 内の他クレートに依存しない)。

## モジュール構成

- `object.rs`: `Object` / `ObjectKind` (Scalar / Product / Coproduct)
- `morphism.rs`: `Morphism` / `MorphismId` (Named / Identity / Composed)
- `category.rs`: `Category` (合成 `compose` と型整合チェック)
- 効果モナド (`Effect`) は #15、`LawChecker` は #16 で追加する

## 設計判断

- **`MorphismId` は文字列 newtype ではなく余積 (`Named` / `Identity` / `Composed`) にしている。**
  `identity` / `compose` が内部で生成する ID とユーザーが登録する基本射の ID を、命名規則
  (`id[X]` や `;` 区切り)で衝突回避するのではなく、型として衝突不可能にするため。
  #15 が効果を持ち上げた導出射を増やす際も、`MorphismId` にバリアントを追加するだけでよく、
  予約語リストの保守が要らない。
- **`Category::add_object` は前方参照を許可しない。** 積・余積のフィールドが参照する対象は、
  参照する側より先に登録済みである必要がある。到達可能性を伴う相互参照が必要になった場合は、
  ここで緩めず `LawChecker` (#16) 側の検証責務として扱う。
- **`compose(f, g)` は図式順。** `f` を先に適用する。数学的な `f∘g` ではなく `g∘f` に相当するため、
  DESIGN.md の関手法則の記法と読み合わせる際は順序の向きに注意する。
