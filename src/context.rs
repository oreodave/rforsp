//! Compilation context.

use crate::{interner::Interner, source::SourceTable};

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
}

impl Compilation {
    /// Construct a new Compilation Context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            table: SourceTable::new(),
            interner: Interner::new(),
        }
    }
}

impl Default for Compilation {
    fn default() -> Self {
        Self::new()
    }
}
