//! Compilation context.

use crate::{
    interner::Interner, runtime::PrimitiveRegistry, source::SourceTable,
};

/// Compilation Context.
///
/// Holds only state that must outlive a single phase.
/// [`Diagnostics`][crate::diagnostics::Diagnostics] are deliberately absent
/// since each phase accumulates its own and the driver gates on them.
pub struct Compilation {
    /// [`SourceTable`] for the current context.
    pub table: SourceTable,
    /// [`Interner`] for the current context.
    pub interner: Interner,
    /// [`PrimitiveRegistry`] for the current context.
    pub primitives: PrimitiveRegistry,
}

impl Compilation {
    /// Construct a new Compilation Context.
    #[must_use]
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let primitives = PrimitiveRegistry::with_builtins(&mut interner);
        Self {
            table: SourceTable::new(),
            interner,
            primitives,
        }
    }
}

impl Default for Compilation {
    fn default() -> Self {
        Self::new()
    }
}
