# functus-core

圏論的 IR を定義するクレート。依存方向は `.claude/rules/workspace-layout.md` に従う(workspace 内の他クレートに依存しない)。

## モジュール構成

- `object.rs`: `Object` / `ObjectKind` (Scalar / Product / Coproduct)
- `morphism.rs`: `Morphism` / `MorphismId` (Named / Identity / Composed)
- `effect.rs`: `Effect` / `EffectStack` (Async / Fallible のモナドスタック)
- `category.rs`: `Category` (合成 `compose` と型整合チェック)
- `LawChecker` は #16 で追加する

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
- **効果は `Morphism.cod` を包むメタデータとして持つ。** `Morphism` は独立した「効果付き射」型を
  持たず、`effects: EffectStack` フィールドで `cod` をどう包むかを表現する
  (`effects.render(&cod)` が実際の返り値の型表記になる)。`compose` の効果合成規則は、
  一方が純粋なら他方の効果をそのまま採用し、両方が効果を持つ場合は同一のスタックなら
  そのまま採用、異なるスタックは `CategoryError::IncompatibleEffects` で拒否する。
  これは Kleisli 圏での bind が同一モナドの下でのみ定義される制約を反映したもので、
  異なるモナドを跨ぐ合成(lift)は Phase1 のスコープ外として意図的に未対応にしている。
