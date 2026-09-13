# rust-reviewer メモリ索引

- [Phase1 Issue 本文の陳腐化](project_phase1_issue_text_stale.md) — Issue #1-#4 の DoD は workspace 設計より前の記述。乖離をコードの欠陥として扱わない
- [射 ID のキー空間の衝突](project_morphism_id_namespace.md) — 導出 ID とユーザー定義 ID が同居し、圏の法則が黙って破れる。Category を触る変更で必ず確認する
- [スクラッチファイルと作業コピーの危険](project_scratch_files_working_copy_hazard.md) — 並行セッションが @ を動かす。検証用ファイルをリポジトリ内に書く前に読む
- [Issue 本文は IR 確定より前](project_issue_text_predates_ir.md) — Phase1 の Issue は #14 以前に書かれており、独自内部モデル前提の記述との乖離は指摘対象外
- [スコープ外の三重一貫性](review_scope_doc_consistency.md) — 未対応範囲は CLAUDE.md・lib.rs の //!・エラーメッセージの3箇所で一致させる。黙って落とす実装は Critical
