//! 射に付与する効果 (Effect)。非同期・失敗をモナドスタックとして表現する。

use std::fmt;

use crate::object::ObjectId;

/// 射に付与できる効果の1層。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// 非同期実行 (Promise / Future)。
    Async,
    /// 失敗しうる計算 (Result / Either)。エラー型を保持する。
    Fallible {
        /// エラー値の対象。
        error: ObjectId,
    },
}

/// 射に付与する効果の積み重ね。先頭が最も外側の層になる。
///
/// 例えば `[Effect::Async, Effect::Fallible { error: ApiError }]` は
/// `Async<Result<_, ApiError>>` を表す。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectStack(Vec<Effect>);

impl EffectStack {
    /// 効果を持たないスタック(純粋な射)を作る。
    pub fn pure() -> Self {
        Self(Vec::new())
    }

    /// 外側から順に並べた層からスタックを作る。
    pub fn wrapping(layers: Vec<Effect>) -> Self {
        Self(layers)
    }

    /// 効果の層を外側から順に返す。
    pub fn layers(&self) -> &[Effect] {
        &self.0
    }

    /// 効果を1つも持たないか(純粋な射か)を返す。
    pub fn is_pure(&self) -> bool {
        self.0.is_empty()
    }

    /// `payload` (成功時の値の対象) にこのスタックを適用した型表記を組み立てる。
    ///
    /// 例: `payload` が `User`、スタックが `[Async, Fallible{ApiError}]` なら
    /// `Async<Result<User, ApiError>>` を返す。
    pub fn render(&self, payload: &ObjectId) -> String {
        self.0
            .iter()
            .rev()
            .fold(payload.to_string(), |acc, effect| match effect {
                Effect::Async => format!("Async<{acc}>"),
                Effect::Fallible { error } => format!("Result<{acc}, {error}>"),
            })
    }
}

impl fmt::Display for EffectStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(&ObjectId::new("_")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_stack_has_no_layers() {
        assert!(EffectStack::pure().is_pure());
    }

    #[test]
    fn render_nests_outer_to_inner() {
        let stack = EffectStack::wrapping(vec![
            Effect::Async,
            Effect::Fallible {
                error: ObjectId::from("ApiError"),
            },
        ]);
        let rendered = stack.render(&ObjectId::from("User"));
        assert_eq!(rendered, "Async<Result<User, ApiError>>");
    }
}
