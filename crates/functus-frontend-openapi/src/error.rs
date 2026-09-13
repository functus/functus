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

    /// 2xx かつスキーマ付きのレスポンスが operation に無い。
    #[error("操作 `{operation_id}` に 2xx かつスキーマ付きのレスポンスが無い")]
    MissingSuccessResponse {
        /// 操作 ID。
        operation_id: String,
    },

    /// 圏への登録に失敗した(対象・射の重複や型不整合)。
    #[error(transparent)]
    Category(#[from] CategoryError),
}
