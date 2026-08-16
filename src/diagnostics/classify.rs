//! Each Phase's Error Kind to [`Class`].
//!
//! One conversion per phase.  A phase's error kinds map only into that
//! phase's classes.

use crate::{diagnostics::Class, lexer::LexErrorKind, parser::ParseErrorKind};

impl From<LexErrorKind> for Class {
    fn from(k: LexErrorKind) -> Self {
        match k {
            LexErrorKind::UnknownCharacter => Self::LexUnknownCharacter,
            LexErrorKind::BindInvalid => Self::LexBindInvalid,
            LexErrorKind::LoadInvalid => Self::LexLoadInvalid,
        }
    }
}

impl From<ParseErrorKind> for Class {
    fn from(e: ParseErrorKind) -> Self {
        match e {
            ParseErrorKind::IntOverflow => Self::ParseIntOverflow,
            ParseErrorKind::NestedQuote => Self::ParseNestedQuote,
            ParseErrorKind::QuoteWithoutForm => Self::ParseQuoteWithoutForm,
            ParseErrorKind::BindingInDatum => Self::ParseBindingInDatum,
            ParseErrorKind::UnterminatedVector => Self::ParseUnterminatedVector,
            ParseErrorKind::UnterminatedList => Self::ParseUnterminatedList,
            ParseErrorKind::MismatchedCloser => Self::ParseMismatchedCloser,
            ParseErrorKind::UnexpectedCloser => Self::ParseUnexpectedCloser,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Phase;

    #[test]
    fn lex_kinds_classify_within_lex() {
        // A kind mapped to the wrong class is invisible at the call site: the
        // diagnostic still renders, just under another phase's code.
        for (kind, class) in [
            (LexErrorKind::UnknownCharacter, Class::LexUnknownCharacter),
            (LexErrorKind::BindInvalid, Class::LexBindInvalid),
            (LexErrorKind::LoadInvalid, Class::LexLoadInvalid),
        ] {
            assert_eq!(Class::from(kind), class, "{kind:?}");
            assert_eq!(
                Class::from(kind).phase(),
                Phase::Lex,
                "{kind:?} escaped its phase"
            );
        }
    }

    #[test]
    fn parse_kinds_classify_within_parse() {
        for (kind, class) in [
            (ParseErrorKind::IntOverflow, Class::ParseIntOverflow),
            (ParseErrorKind::NestedQuote, Class::ParseNestedQuote),
            (
                ParseErrorKind::QuoteWithoutForm,
                Class::ParseQuoteWithoutForm,
            ),
            (ParseErrorKind::BindingInDatum, Class::ParseBindingInDatum),
            (
                ParseErrorKind::UnterminatedVector,
                Class::ParseUnterminatedVector,
            ),
            (
                ParseErrorKind::UnterminatedList,
                Class::ParseUnterminatedList,
            ),
            (
                ParseErrorKind::MismatchedCloser,
                Class::ParseMismatchedCloser,
            ),
            (
                ParseErrorKind::UnexpectedCloser,
                Class::ParseUnexpectedCloser,
            ),
        ] {
            assert_eq!(Class::from(kind), class, "{kind:?}");
            assert_eq!(
                Class::from(kind).phase(),
                Phase::Parse,
                "{kind:?} escaped its phase"
            );
        }
    }
}
