//! Main lowering algorithm.

use crate::{
    diagnostics::Diagnostics,
    lowering::Module,
    parser::HirForm,
    runtime::{PrimitiveRegistry, RuntimeVariableRegistry},
};

/// Lower the given [`HirForm`]s, returning an SIR [`Module`] if no errors
/// arise, otherwise [`None`].
///
/// This constructs its own [`Diagnostics`] which is always passed back to the
/// caller.
#[must_use]
pub fn lower(
    _forms: &[HirForm],
    _primitives: &PrimitiveRegistry,
    _variables: &RuntimeVariableRegistry,
) -> (Option<Module>, Diagnostics) {
    todo!("implement");
}
