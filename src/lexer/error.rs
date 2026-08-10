//! Errors that may arise during lexing.

use crate::source::SyntaxOrigin;

/// Possible types of errors that may arise during Lexing.
pub enum LexErrorKind {
    /// Character is not known
    UnknownCharacter,
    /// Bind operator ($) is not followed by a symbol
    BindInvalid,
    /// Load operator (^) is not followed by a symbol
    LoadInvalid,
}

/// Lexing error type
pub struct LexError {
    /// Point of origin for error.
    pub origin: SyntaxOrigin,
    /// Type of error.
    pub kind: LexErrorKind,
}
