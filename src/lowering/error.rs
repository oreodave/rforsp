//! Errors that may arise during Lowering.

use crate::source::SyntaxId;

/// Possible types of errors that may arise during Lowering.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LoweringErrorKind {
    /// A symbol referenced in a Call or Load that couldn't be resolved.
    UnresolvedSymbol,
}

/// Lowering error.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct LoweringError {
    /// Point of origin for error.
    pub origin: SyntaxId,
    /// Type of error.
    pub kind: LoweringErrorKind,
}
