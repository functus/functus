---
name: scratch-files-working-copy-hazard
description: functus は jj colocated かつ複数セッションが同時に @ を動かすため、検証用スクラッチファイルをリポジトリ内に書くとレビュー対象のコミットに巻き込まれうる。仮説検証の手段を選ぶときに読む。
metadata:
  type: project
---

レビュー中の仮説検証のために `crates/<crate>/tests/` へ一時的なテストファイルを書かない。書いた場合は、そのコマンドの直後に消えることを前提にする。

**Why:** functus は jj の colocated リポジトリで、jj は任意のコマンド実行時に作業コピーを自動スナップショットする。さらに、このリポジトリでは別の Claude セッションが並行して `jj edit` や `jj new` で `@` を動かすことが実際に起きる。2026-09-13 のレビューでは、`crates/functus-core/tests/zz_scratch_review.rs` を書いて `cargo test` で仮説を実測した直後に、別セッションが `@` を build/16-law-checker から build/1-rust-workspace-setup へ移していたことが判明した。次に自分が `jj` コマンドを打った時点で作業コピーが更新され、スクラッチファイルも法則チェッカーの実装ファイルも消えた。結果として、どの bookmark にも混入せず jj の内部 snapshot ref に残っただけで済んだが、タイミング次第ではレビュー対象のコミットにスクラッチファイルが入りうる。

**How to apply:**

- 仮説を実測したくなったら、まずファイルを書かずに済む手段を検討する。`jj file show -r <bookmark> <path>` で対象リビジョンの内容を読む方法は作業コピーに触れない
- どうしても実行して確かめる必要がある場合は、書いた直後に `jj diff --summary` で `@` に何が入ったかを確認し、削除後にもう一度確認する
- レビュー中に `jj log` や `jj status` の出力が想定と食い違ったら、まず `jj op log` と `jj workspace list` で他セッションの操作を疑う。自分の操作のせいだと決めつけない
- レビュー対象の bookmark が汚れていないことは `jj diff -r <bookmark> --summary` で確認する。行番号を引用する前に `jj file show -r <bookmark>` で読み直す

関連: [[morphism-id-namespace]]
