//! Main lowering routine.

use crate::{lowering::Module, parser::HirForm, resolution::ResolutionResult};

/// Lower a complete sequence of [`HirForm`]s with a given [`ResolutionResult`].
#[must_use]
#[expect(
    clippy::todo,
    reason = "Lowering implementation follows the IR sketch"
)]
pub fn lower(_forms: &[HirForm], _resolution: ResolutionResult) -> Module {
    todo!("lower resolved HIR into a module")
}
