//! Token data type

use crate::source::Span;

/// Kinds of Tokens
pub enum Kind {
    /// Numeric token
    Number,
    /// Any generic symbol
    Symbol,
    /// $<Symbol>
    Bind,
    /// ^<Symbol>
    Load,
    /// [
    VecStart,
    /// ]
    VecEnd,
    /// (
    ListStart,
    /// )
    ListEnd,
}

/// Token type
pub struct Token {
    /// Kind of token
    kind: Kind,
    /// Span within source of Token.
    span: Span,
}
