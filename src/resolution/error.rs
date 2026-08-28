//! Errors from Resolution.

use crate::{interner::SymId, source::SyntaxId};

/// Possible types of error that may arise during Resolution.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ResolutionErrorKind {
    /// A symbol that couldn't be resolved was used in a Call or Load.
    UnresolvedSymbol(SymId),
    /// Recognised static data was used in conjunction with `rec`.
    RecDataOperand,
}

/// Resolution error.
pub struct ResolutionError {
    /// Point of origin for error.
    pub origin: SyntaxId,
    /// Type of error.
    pub kind: ResolutionErrorKind,
}
