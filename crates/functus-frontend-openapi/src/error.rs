//! OpenAPI → IR 変換のエラー型。

use functus_core::CategoryError;

/// OpenAPI から `functus_core::Category` への変換が失敗したときのエラー。
///
/// 新しい失敗種別を今後追加する見込みが高いため `#[non_exhaustive]` を付けている。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FrontendError {
    /// OpenAPI YAML のパースに失敗した。
    #[error("OpenAPI YAML のパースに失敗した: {0}")]
    Yaml(#[from] serde_norway::Error),

    /// `openapi` フィールドが v3.0.x 以外を示している。`openapiv3` クレートは
    /// v3.0.x 専用のため、v3.1 の文書を構文的に読めても意味的には正しく
    /// 変換できない(`nullable` の扱い等が異なる)。
    #[error("OpenAPI バージョン `{version}` は未対応。v3.0.x のみサポートする")]
    UnsupportedOpenApiVersion {
        /// 文書の `openapi` フィールドの値。
        version: String,
    },

    /// `#/components/schemas/<name>` 以外の形の `$ref` は Phase1 で未対応。
    #[error(
        "`{context}` の `$ref` (`{reference}`) は未対応。\
         `#/components/schemas/<name>` の形のみサポートする"
    )]
    UnsupportedReference {
        /// `$ref` が出現した文脈の説明(例: "getUser の path parameter id")。
        context: String,
        /// 未解決の参照文字列。
        reference: String,
    },

    /// Phase1 でサポートしないスキーマ形状(ネストしたオブジェクト・配列・oneOf 等)。
    #[error("`{schema}` は Phase1 でサポートしないスキーマ形状: {reason}")]
    UnsupportedSchema {
        /// 対象スキーマの名前や文脈。
        schema: String,
        /// サポートしない理由。
        reason: String,
    },

    /// パス上の operation に `operationId` が無い。
    #[error("`{method} {path}` に operationId が無い")]
    MissingOperationId {
        /// HTTP メソッド。
        method: String,
        /// パステンプレート。
        path: String,
    },

    /// 2xx レスポンスが operation に無い(本文の有無は問わない)。
    #[error("操作 `{operation_id}` に 2xx レスポンスが無い")]
    MissingSuccessResponse {
        /// 操作 ID。
        operation_id: String,
    },

    /// スキーマ対象名が組み込みスカラー名(`String`/`Uuid` 等)と衝突している。
    #[error(
        "対象 `{name}` は組み込みスカラー名として予約されているため、\
         同名かつスカラーでない対象を定義できない"
    )]
    ScalarNameCollision {
        /// 衝突した名前。
        name: String,
    },

    /// 非2xxレスポンス(`default` を含む)が2種類以上の異なるスキーマを持ち、
    /// `EffectStack` の `Fallible` 層は1つしか持てないため一意に決められない。
    #[error(
        "操作 `{operation_id}` の非2xxレスポンス({}) が異なるスキーマを持つため、\
         単一のエラー型に決められない",
        .labels.join(", ")
    )]
    MultipleErrorResponsesUnsupported {
        /// 操作 ID。
        operation_id: String,
        /// 異なるスキーマを持っていたレスポンスのラベル(ステータスコード等)。
        labels: Vec<String>,
    },

    /// 明示コードの2xxレスポンスが2種類以上の異なるスキーマを持ち、`cod` を
    /// 一意に決められない。
    #[error(
        "操作 `{operation_id}` の2xxレスポンス({}) が異なるスキーマを持つため、\
         単一の成功型に決められない",
        .labels.join(", ")
    )]
    MultipleSuccessResponsesUnsupported {
        /// 操作 ID。
        operation_id: String,
        /// 異なるスキーマを持っていたレスポンスのラベル(ステータスコード等)。
        labels: Vec<String>,
    },

    /// 圏への登録に失敗した(対象・射の重複や型不整合)。
    #[error(transparent)]
    Category(#[from] CategoryError),
}
