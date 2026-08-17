//! Errors that may arise during parsing.

use crate::source::SyntaxOrigin;

/// Possible types of errors that may arise during Parsing.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ParseErrorKind {
    /// Integer literal outside the range of an `i64`.
    IntOverflow,
    /// A quote (') directly wrapping another quote, as in `''x`.
    NestedQuote,
    /// A quote (') with no following form to wrap.
    QuoteWithoutForm,
    /// A binding form (`$x` or `^x`) in datum position, which has no datum to
    /// denote.
    BindingInDatum,
    /// A vector left open at end of input.
    UnterminatedVector,
    /// A list left open at end of input.
    UnterminatedList,
    /// A closer token for the wrong kind of innermost open container.
    MismatchedCloser,
    /// A closer with no open container at all.
    UnexpectedCloser,
}

/// Parsing error.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ParseError {
    /// Point of origin for error.
    pub origin: SyntaxOrigin,
    /// Type of error.
    pub kind: ParseErrorKind,
}
