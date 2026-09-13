//! 圏論的 IR (Category / Object / Morphism / 効果モナド / LawChecker) を定義するクレート。
//! LawChecker は #16 で追加する。
#![deny(missing_docs)]

mod category;
mod effect;
mod morphism;
mod object;

pub use category::{Category, CategoryError};
pub use effect::{Effect, EffectStack};
pub use morphism::{Morphism, MorphismId};
pub use object::{Object, ObjectId, ObjectKind};
