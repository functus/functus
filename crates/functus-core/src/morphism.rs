//! 圏の射 (Morphism)。API 操作・状態遷移・純粋変換を表す。

use crate::object::ObjectId;

/// 射を一意に識別する ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MorphismId(pub String);

impl MorphismId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MorphismId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

/// 射の由来。
#[derive(Debug, Clone, PartialEq)]
pub enum MorphismKind {
    /// 対象ごとに一意に存在する恒等射。
    Identity,
    /// フロントエンド・DSL から直接導入された基本射。
    Primitive,
    /// `compose` によって構成された射。要素は合成順 (先に適用される射が先頭)。
    Composed(Vec<MorphismId>),
}

/// 圏の射。`dom` から `cod` への向きを持つ。
#[derive(Debug, Clone, PartialEq)]
pub struct Morphism {
    pub id: MorphismId,
    pub dom: ObjectId,
    pub cod: ObjectId,
    pub kind: MorphismKind,
}
