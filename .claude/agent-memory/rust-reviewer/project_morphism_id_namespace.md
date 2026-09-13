---
name: morphism-id-namespace
description: functus-core の射 ID は導出名とユーザー定義名が同じキー空間を共有しており、#15 以降も衝突を作り込みやすい。Category を触る変更のレビュー時に必ず確認する。
metadata:
  type: project
---

functus-core の `Category` は、導出される射の ID とユーザーが `add_primitive_morphism` に渡す ID を同一のキー空間に置いている。#14 の初回実装では `identity` が `id[A]`、`compose` が構成要素を `;` で連結した文字列を ID にしており、同名の基本射を登録すると `or_insert_with` が黙って既存の射を返し、単位律と結合律の両方が破れることを再現で確認した。

**Why:** #15 が効果モナドを射に付与し、#16 の LawChecker が dom と cod の整合性を静的検証する。効果を持ち上げた射のような導出射が増えるたびに導出名が増え、文字列の禁止リストで防ぐ方式では追加のたびに漏れが silent aliasing のバグになる。構造的なキー、つまり `MorphismId` を `Named` と `Identity(ObjectId)` と `Composed(Vec<MorphismId>)` の余積にする方式なら衝突が表現不可能になり、新しい導出射はバリアント追加だけで済む。

**How to apply:** `Category` の射 ID や対象 ID の生成に関わる変更をレビューするときは、導出 ID がユーザー由来の ID と衝突しうるかを最初に確認する。`insert` や `or_insert_with` で既存エントリを黙って上書きまたは再利用していたら Critical として扱う。あわせて `tests/laws.rs` の法則テストが ID の等価性だけを見ていないかを確認する。ID が構成要素から決定的に導出される設計では、その等価性判定は構成上必ず成立してしまい、法則が破れても Green のままになる。
