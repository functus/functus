# functus-core

圏論的 IR を定義するクレート。依存方向は `.claude/rules/workspace-layout.md` に従う(workspace 内の他クレートに依存しない)。

## モジュール構成

- `object.rs`: `Object` / `ObjectKind` (Scalar / Product / Coproduct)
- `morphism.rs`: `Morphism` / `MorphismId` (Named / Identity / Composed)
- `effect.rs`: `Effect` / `EffectStack` (Async / Fallible のモナドスタック)
- `category.rs`: `Category` (合成 `compose` と型整合チェック)
- `law_checker.rs`: `LawChecker` (合成可能性・余積網羅性の読み取り専用検証)

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
- **`compose` と `LawChecker::check_composable` は同じ判定ロジックを共有する。**
  依存の向きは `LawChecker → Category`(`Category` は `LawChecker` を知らない)。
  共有ロジックは `Category::resolve_composition` という `pub(crate)` メソッドに置き、
  `compose` はこれを呼んで合成射を登録し、`check_composable` は同じメソッドを呼んで
  結果を読み捨てる。dom/cod の一致だけでなく効果の互換性(`merge_effects`)も
  この1箇所で判定するため、事前チェックが `Ok` を返したのに実際の `compose` が
  失敗する(逆も同様)という乖離が構造的に起きない。
- **`LawChecker::check_coproduct_variants_are_consumed` の「処理される」は
  暫定的なヒューリスティックであり、既知の限界がある。** 判定基準は「バリアントの
  ペイロード対象を `dom` に取る `MorphismId::Named` の射が1つ以上あるか」。
  `MorphismId::Identity` / `MorphismId::Composed` は候補から除外している
  (恒等射は no-op であり処理したことにならない)。また同じペイロード対象を
  複数のバリアントが共有している場合は判定不能として `AmbiguousCoproductVariants`
  で拒否する。塞ぎきれていない穴として、**余積と無関係な射がたまたま同じ
  ペイロード対象を `dom` に取っているだけでも「処理済み」と誤判定される**。
  これを原理的に解消するには、射がどの余積のどのバリアントを消費するかを
  IR が明示的に表現できる必要があり(入射射にバリアントのタグを持たせる等)、
  DSL の `match` 構文が備わる #20 以降で置き換わる想定である。
