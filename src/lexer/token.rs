//! Token data type

use crate::source::Span;

/// Kinds of Tokens
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TokenKind {
    /// [
    VecStart,
    /// ]
    VecEnd,
    /// (
    ListStart,
    /// )
    ListEnd,
    /// '
    Quote,
    /// Numeric token
    Number,
    /// Any generic symbol
    Symbol,
    /// `$<Symbol>`
    Bind,
    /// `^<Symbol>`
    Load,
}

/// Token type
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Token {
    /// Kind of token
    pub kind: TokenKind,
    /// Span within source of Token.
    pub span: Span,
}
