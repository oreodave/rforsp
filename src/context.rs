//! Compilation context.

use crate::{interner::Interner, source::SourceTable};

/// Compilation Context.
///
/// State that is threaded through the differing compilation phases.  It holds
/// only the _shared_ state of the phases i.e. stuff that must persist
/// throughout the compilation.
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
