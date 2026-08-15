//! Each Phase's Error Kind to [`Class`]

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
