# functus 全体設計ドキュメント

> 圏論の概念を用いてコードの品質を担保するツールエコシステムの設計

このドキュメントは以下のファイルに分割されている。各ファイルは単独で読める形になっているが、
初めて読む場合は上から順に読むことを推奨する。

1. [ビジョン](design/01-vision.md) — Correct-by-Construction という目標
2. [アーキテクチャ概要](design/02-architecture.md) — 3層構造と Cargo workspace のクレート構成
3. [圏論的 IR の設計](design/03-categorical-ir.md) — 対象・射・関手・モナドの対応関係と法則による品質担保
4. [コンポーネント設計](design/04-components.md) — functus-core / フロントエンド / コード生成 / DSL / verify / cli
5. [ロードマップ](design/05-roadmap.md) — Phase 0–4 のマイルストーン
6. [品質担保の原則](design/06-principles.md) — 3 原則のまとめ

## 関連ドキュメント

- Rust の実装規約・ワークスペース構成の詳細: `.claude/rules/`
- Claude Code ハーネス全体の運用: `CLAUDE.md`
