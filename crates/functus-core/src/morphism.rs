//! 圏の射 (Morphism)。API 操作・状態遷移・純粋変換を表す。

use std::fmt;

use crate::object::ObjectId;

/// 射を一意に識別する ID。
///
/// 由来ごとにバリアントを分けることで、`identity` / `compose` が内部で
/// 生成する ID とユーザーが登録する基本射の ID が、文字列の命名規則ではなく
/// 型として衝突不可能になる。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MorphismId {
    /// フロントエンド・DSL 由来の基本射。
    Named(String),
    /// 対象ごとに一意に存在する恒等射。
    Identity(ObjectId),
    /// `compose` によって構成された射。要素は合成順(先に適用される射が先頭)。
    Composed(Vec<MorphismId>),
}

impl MorphismId {
    /// 基本射の ID を作る。
    pub fn named(id: impl Into<String>) -> Self {
        Self::Named(id.into())
    }
}

impl From<&str> for MorphismId {
    fn from(id: &str) -> Self {
        Self::named(id)
    }
}

impl fmt::Display for MorphismId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MorphismId::Named(name) => write!(f, "{name}"),
            MorphismId::Identity(obj) => write!(f, "id[{obj}]"),
            MorphismId::Composed(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ";")?;
                    }
                    write!(f, "{part}")?;
                }
                Ok(())
            }
        }
    }
}

/// 圏の射。`dom` から `cod` への向きを持つ。
///
/// 恒等射・合成射であるかは `id` の `MorphismId::Identity` /
/// `MorphismId::Composed` バリアントで判別できるため、`Morphism` 自体は
/// 由来を表す別フィールドを持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morphism {
    /// この射の ID。
    pub id: MorphismId,
    /// 域(この射の入力側の対象)。
    pub dom: ObjectId,
    /// 余域(この射の出力側の対象)。
    pub cod: ObjectId,
}
