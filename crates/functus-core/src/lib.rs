//! 圏論的 IR (Category / Object / Morphism / 効果モナド / LawChecker) を定義するクレート。
#![deny(missing_docs)]

mod category;
mod effect;
mod law_checker;
mod morphism;
mod object;

pub use category::{Category, CategoryError};
pub use effect::{Effect, EffectStack};
pub use law_checker::LawChecker;
pub use morphism::{Morphism, MorphismId};
pub use object::{Object, ObjectId, ObjectKind};
