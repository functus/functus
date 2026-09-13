//! 圏論的 IR (Category / Object / Morphism / 効果モナド / LawChecker) を定義するクレート。
//! 効果モナドと LawChecker は #15〜#16 で追加する。
#![deny(missing_docs)]

mod category;
mod morphism;
mod object;

pub use category::{Category, CategoryError};
pub use morphism::{Morphism, MorphismId};
pub use object::{Object, ObjectId, ObjectKind};
