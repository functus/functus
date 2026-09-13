# 2. アーキテクチャ概要

[← 目次に戻る](../DESIGN.md)

```
入力フロントエンド          コア                      バックエンド
┌──────────────┐   ┌───────────────────┐   ┌─────────────────┐
│ OpenAPI v3.x │──▶│                   │──▶│ TypeScript/React │
├──────────────┤   │  圏論的 IR         │   ├─────────────────┤
│ functus DSL  │──▶│  (functus-core)    │──▶│ Go               │
├──────────────┤   │  + 法則検証        │   ├─────────────────┤
│ (将来) GraphQL│──▶│  (functus-verify) │──▶│ Rust             │
└──────────────┘   └───────────────────┘   └─────────────────┘
                            ▲
                       functus-cli
```

## クレート構成 (Cargo workspace)

| クレート | 役割 |
|---|---|
| `functus-core` | 圏論的 IR の定義。対象(型)・射(操作)・関手(言語への写像)・モナド(効果)・自然変換(表現間変換)と、その法則チェッカー |
| `functus-frontend-openapi` | OpenAPI v3.0/3.1 → IR 変換 (`openapiv3` クレート利用) |
| `functus-dsl` | 圏論モデルを直接記述する DSL のパーサー・型検査器 |
| `functus-gen` | `CodeGenerator` トレイトと各言語バックエンド (TS/React, Go, Rust) |
| `functus-verify` | 法則検証。生成コードに対するプロパティベーステスト生成 |
| `functus-cli` | `functus generate / check / verify / explain` |

クレート間の依存方向・各ディレクトリの役割の詳細は `.claude/rules/workspace-layout.md` を参照。
