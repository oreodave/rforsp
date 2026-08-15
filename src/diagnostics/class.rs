//! Classification of Diagnostics
//!
//! Each [`Diagnostic`][crate::diagnostics::Diagnostic] has a generalised class.
//! This class should cover most of the internal error variants of each
//! possibly-fallible compiler phase, as well as other variants of
//! warnings/notes/etc.
//!
//! The [`Severity`] of a [`Diagnostic`][crate::diagnostics::Diagnostic] is
//! derived from the [`Class`].

use crate::diagnostics::phase::Phase;

/// How serious a diagnostic is.
///
/// Only [`Severity::Error`] and [`Severity::Bug`] are fatal; a stage may
/// complete successfully while carrying [`Severity::Note`] and
/// [`Severity::Warning`] diagnostics.
///
/// Ordering runs from least to most severe.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Severity {
    /// Additional context attached to another diagnostic.
    Note,
    /// The program is accepted; something may be wrong.
    Warning,
    /// The program is rejected; this is the user's fault.
    Error,
    /// An invariant in the compiler was broken; this is the compiler author's
    /// fault.
    Bug,
}

/// Declare [`Class`] together with every table a class must populate.
///
/// One row per class - documentation, variant, owning [`Phase`], [`Severity`],
/// and stable code - generating the enum, [`Class::ALL`], and the three
/// lookups.  A class therefore cannot exist without all four, so the tables
/// cannot fall out of step with the variant set.
macro_rules! classes {
    ($(
        $(#[$meta:meta])*
        $variant:ident => $phase:ident, $severity:ident, $code:literal;
    )*) => {
        /// Classification of diagnostics.
        #[derive(Debug, PartialEq, Eq, Copy, Clone)]
        pub enum Class {
            $($(#[$meta])* $variant,)*
        }

        impl Class {
            /// Every [`Class`], in declaration order.
            ///
            /// This is the compiler's full error surface, enumerable rather
            /// than merely greppable.  It is generated from the same rows as
            /// the enum, so it cannot omit a variant.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            /// Get the [`Phase`] for this [`Class`].
            #[must_use]
            pub const fn phase(&self) -> Phase {
                match self {
                    $(Self::$variant => Phase::$phase,)*
                }
            }

            /// Get the [`Severity`] for this [`Class`]
            #[must_use]
            pub const fn severity(&self) -> Severity {
                match self {
                    $(Self::$variant => Severity::$severity,)*
                }
            }

            /// Convert Class to a stable diagnostic code.
            ///
            /// The code is bare; the renderer namespaces it with
            /// [`Class::phase`].
            #[must_use]
            pub const fn as_code(&self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)*
                }
            }
        }
    };
}

classes! {
    /// Source is too large.  Mirrors
    /// [`TooLarge`][crate::source::SourceError::TooLarge].
    SourceTooLarge => Source, Error, "TOO_LARGE";

    /// File could not be read due to IO error.  Mirrors
    /// [`Io`][crate::source::SourceTableError::Io].
    SourceReadError => Source, Error, "IO_ERROR";

    /// Encountered an unknown character during lexing.  Mirrors
    /// [`UnknownCharacter`][crate::lexer::LexErrorKind::UnknownCharacter]
    LexUnknownCharacter => Lex, Error, "UNKNOWN_CHARACTER";

    /// Use of BIND operator ($) was invalid.  Mirrors
    /// [`BindInvalid`][crate::lexer::LexErrorKind::BindInvalid]
    LexBindInvalid => Lex, Error, "BIND_INVALID";

    /// Use of LOAD operator (^) was invalid.  Mirrors
    /// [`LoadInvalid`][crate::lexer::LexErrorKind::LoadInvalid]
    LexLoadInvalid => Lex, Error, "LOAD_INVALID";

    /// Integer literal does not fit an `i64`.  Mirrors
    /// [`IntOverflow`][crate::parser::ParseErrorKind::IntOverflow]
    ParseIntOverflow => Parse, Error, "INT_OVERFLOW";

    /// A quote directly wrapping another quote.  Mirrors
    /// [`NestedQuote`][crate::parser::ParseErrorKind::NestedQuote]
    ParseNestedQuote => Parse, Error, "NESTED_QUOTE";

    /// A quote with no following form.  Mirrors
    /// [`QuoteWithoutForm`][crate::parser::ParseErrorKind::QuoteWithoutForm]
    ParseQuoteWithoutForm => Parse, Error, "QUOTE_WITHOUT_FORM";

    /// A binding form where a datum is required.  Mirrors
    /// [`BindingInDatum`][crate::parser::ParseErrorKind::BindingInDatum]
    ParseBindingInDatum => Parse, Error, "BINDING_IN_DATUM";

    /// Poisoned/dropped output from a compiler phase despite no new Diagnostics
    /// generated in a phase.
    ICEDroppedOutput => ICE, Bug, "DROPPED_OUTPUT";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A class's code as the renderer spells it, namespaced by owning phase.
    fn qualified(class: Class) -> String {
        format!("{}::{}", class.phase().as_str(), class.as_code())
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Note < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Bug);
    }

    #[test]
    fn codes_are_unique() {
        // Two classes sharing a code cannot be told apart by a test asserting
        // on the class of a diagnostic, which is what codes exist for.
        let mut codes: Vec<String> =
            Class::ALL.iter().copied().map(qualified).collect();
        let declared = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(
            codes.len(),
            declared,
            "two classes share a diagnostic code: {codes:?}"
        );
    }

    #[test]
    fn codes_are_greppable() {
        // Codes are meant to be greppable and to render inside `error[..]`,
        // so they stay UPPER_SNAKE and nothing else.
        for &class in Class::ALL {
            let code = class.as_code();
            assert!(!code.is_empty(), "{class:?} has an empty code");
            assert!(
                code.bytes().all(|b| b.is_ascii_uppercase()
                    || b.is_ascii_digit()
                    || b == b'_'),
                "{class:?} has a code that is not UPPER_SNAKE: {code}"
            );
        }
    }

    #[test]
    fn no_phase_emits_another_phases_codes() {
        // The variant name is the only place a class's phase is written down
        // twice, so it is the only available cross-check on `phase`.
        for &class in Class::ALL {
            let name = format!("{class:?}").to_lowercase();
            let phase = class.phase().as_str();
            assert!(
                name.starts_with(phase),
                "{class:?} belongs to phase {phase} and must be named for it"
            );
        }
    }
}
