//! Errors from Resolution.

use crate::source::SyntaxId;

/// Possible types of error that may arise during Resolution.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ResolutionErrorKind {
    /// A symbol that couldn't be resolved was used in a Call or Load.
    UnresolvedSymbol,
}

/// Resolution error.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ResolutionError {
    /// Point of origin for error.
    pub origin: SyntaxId,
    /// Type of error.
    pub kind: ResolutionErrorKind,
}
