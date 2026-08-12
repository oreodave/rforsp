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

impl Token {
    /// Construct the span for the actual symbol a [`TokenKind::Bind`] and
    /// [`TokenKind::Load`] lex.
    ///
    /// If the given Token is not of those kinds, the original span is returned.
    ///
    /// # Panics
    /// - If a [`TokenKind::Bind`] or [`TokenKind::Load`] spans its sigil alone.
    ///   Only a hand-built Token can; a bare sigil is a lex error.
    #[must_use]
    pub const fn symbol_span(&self) -> Span {
        if matches!(self.kind, TokenKind::Bind | TokenKind::Load) {
            Span::from_u32(self.span.start + 1, self.span.end)
        } else {
            self.span
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_span_drops_the_sigil() {
        // An offset in either direction is silent: the interned name simply
        // stops matching the one a later `^x` looks up.
        let bind = Token {
            kind: TokenKind::Bind,
            span: Span::new(4, 6),
        };
        assert_eq!(bind.symbol_span(), Span::new(5, 6));

        let load = Token {
            kind: TokenKind::Load,
            span: Span::new(0, 3),
        };
        assert_eq!(load.symbol_span(), Span::new(1, 3));
    }

    #[test]
    fn symbol_span_passes_other_kinds_through() {
        let span = Span::new(2, 7);
        for kind in [
            TokenKind::Symbol,
            TokenKind::Number,
            TokenKind::Quote,
            TokenKind::VecStart,
        ] {
            assert_eq!(Token { kind, span }.symbol_span(), span, "{kind:?}");
        }
    }
}
