//! Errors that may arise during lexing.

use crate::source::SyntaxOrigin;

/// Possible types of errors that may arise during Lexing.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LexErrorKind {
    /// Character is not known.  Control and format characters share this
    /// kind: both render as nothing, so both fail for one reason.
    UnknownCharacter,
    /// Bind operator ($) is not followed by a symbol.
    BindInvalid,
    /// Load operator (^) is not followed by a symbol.
    LoadInvalid,
}

/// Lexing error.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct LexError {
    /// Point of origin for error.
    pub origin: SyntaxOrigin,
    /// Type of error.
    pub kind: LexErrorKind,
}
