---
paths:
  - "Cargo.toml"
  - "**/Cargo.toml"
  - "crates/**"
  - "**/*.rs"
---

# Workspace レイアウトとディレクトリの役割

functus は Cargo workspace で開発する。各ディレクトリの役割と依存方向は以下の通り。

## ディレクトリ構成

```
functus/
├── Cargo.toml              # workspace ルート ([workspace] のみ。依存はここで一元管理)
├── crates/
│   ├── functus-core/       # 圏論的 IR。対象・射・関手・モナド・法則チェッカー
│   ├── functus-frontend-openapi/  # OpenAPI v3.0/3.1 → IR 変換
│   ├── functus-dsl/        # DSL パーサー・型検査器 (Phase3)
│   ├── functus-gen/        # CodeGenerator トレイトと言語バックエンド (ts/ go/ rust/)
│   ├── functus-verify/     # 法則検証・プロパティテスト生成 (Phase3)
│   └── functus-cli/        # CLI バイナリ (Phase4)
├── examples/               # E2E サンプル (OpenAPI 定義 + 生成コード + React アプリ)
├── tests/fixtures/         # テスト用 OpenAPI YAML 等の共有フィクスチャ
├── docs/                   # 設計ドキュメント (DESIGN.md が全体設計の正)
└── .claude/                # Claude Code ハーネス (hooks / skills / agents / rules)
```

## 依存方向のルール(違反はレビューで Critical)

```
functus-cli ──▶ frontend-openapi / dsl / gen / verify ──▶ functus-core
```

- **functus-core は workspace 内の何にも依存しない**(外部クレートも最小限: serde / thiserror 程度)
- フロントエンド(openapi / dsl)とバックエンド(gen)は互いに依存しない。IR(core)経由でのみ通信する
- verify は core と gen に依存してよい。cli はすべてに依存してよい
- 循環依存は禁止(Cargo が拒否するが、feature 経由の実質的循環も禁止)

## 各クレートの規約

- 各クレートのルートに `CLAUDE.md` を置き、そのクレート固有の設計判断を記録する(クレート作成時に必須)
- 公開 API は各クレートの `lib.rs` で `pub use` により明示的に再エクスポートする(深いパスを外部に晒さない)
- バイナリは `functus-cli` のみ。他クレートに `main.rs` を置かない
- 新しい言語バックエンドは `crates/functus-gen/src/backends/<lang>.rs` に追加し、関手法則テストを必ず付ける
