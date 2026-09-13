# functus-core

圏論的 IR を定義するクレート。依存方向は `.claude/rules/workspace-layout.md` に従う(workspace 内の他クレートに依存しない)。

## モジュール構成の予定

- `object.rs`: `Object` / `ObjectKind` (Scalar / Product / Coproduct)
- `morphism.rs`: `Morphism` / `MorphismKind` (Identity / Primitive / Composed)
- `category.rs`: `Category` (合成 `compose` と型整合チェック)
- 効果モナド (`Effect`) は #15、`LawChecker` は #16 で追加する

現時点の `lib.rs` はクレートの説明コメントのみで上記モジュールを持たない。#14 で埋まる。
