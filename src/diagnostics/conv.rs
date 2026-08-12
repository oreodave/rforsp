//! Conversions from phase-specific Errors to Diagnostic
//!
//! Each compiler phase's internal error type which we expect to eventually
//! report to the user should have a conversion here into Diagnostic.
//!
//! Diagnostics with no error type behind them are constructed here too, by a
//! named constructor per [`Class`].  Construction lives in one module so that
//! every user-facing string in the compiler has a single home.

use std::fmt::Write as _;

use crate::{
    diagnostics::{Class, Diagnostic, Phase, Site},
    lexer::{LexError, LexErrorKind},
    source::{SourceError, SourceTableError},
};

impl From<SourceError> for Diagnostic {
    fn from(e: SourceError) -> Self {
        match e {
            SourceError::TooLarge { name, len, limit } => Self::new(
                Class::SourceTooLarge,
                Site::None,
                format!("{name}: contains {len} bytes when limit is {limit}"),
            ),
        }
    }
}

impl From<SourceTableError> for Diagnostic {
    fn from(e: SourceTableError) -> Self {
        match e {
            SourceTableError::SourceCreate(e) => Self::from(e),
            SourceTableError::Io { name, err } => Self::new(
                Class::SourceReadError,
                Site::None,
                format!("{name}: {err}"),
            ),
        }
    }
}

impl From<LexError> for Diagnostic {
    fn from(e: LexError) -> Self {
        let site = Site::Raw(e.origin);
        let class = match e.kind {
            LexErrorKind::UnknownCharacter => Class::LexUnknownCharacter,
            LexErrorKind::BindInvalid => Class::LexBindInvalid,
            LexErrorKind::LoadInvalid => Class::LexLoadInvalid,
        };
        let message = match e.kind {
            LexErrorKind::UnknownCharacter => "Unrecognised character",
            LexErrorKind::BindInvalid => {
                "Expected Symbol immediately after Bind ($)"
            }
            LexErrorKind::LoadInvalid => {
                "Expected Symbol immediately after Load (^)"
            }
        };

        Self::new(class, site, message)
    }
}

/// Tag a compiler bug with the location in the *compiler's* source that
/// detected it.
///
/// Call this directly at the point of detection.  If it is ever wrapped in a
/// per-class constructor, that constructor needs `#[track_caller]` too, or the
/// location reported is the wrapper's.
///
/// # Panics
/// - If `diag` is not a [`Phase::ICE`] diagnostic, since nothing else has a
///   detection site to report.
#[must_use]
#[track_caller]
pub fn ice(mut diag: Diagnostic) -> Diagnostic {
    assert_eq!(
        diag.class.phase(),
        Phase::ICE,
        "only a compiler bug carries a detection site"
    );

    let location = std::panic::Location::caller();
    let _ = write!(diag.message, ", detected at {location}");
    diag
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceTable, Span, SyntaxOrigin};

    fn sample_source_error() -> SourceError {
        SourceError::TooLarge {
            name: "hello".to_string(),
            len: 1000,
            limit: 100,
        }
    }

    #[test]
    fn source_err() {
        let source_error = sample_source_error();
        let diag = Diagnostic::from(source_error);
        assert_eq!(diag.class, Class::SourceTooLarge);
        assert_eq!(diag.class.phase(), Phase::Source);
        assert!(diag.message.contains("hello"));
        assert!(diag.message.contains("contains 1000"));
        assert!(diag.message.contains("limit is 100"));
    }

    #[test]
    fn source_table_err() {
        let source_error = sample_source_error();
        let diag = Diagnostic::from(SourceTableError::from(source_error));
        assert_eq!(diag.class, Class::SourceTooLarge);
        assert_eq!(diag.class.phase(), Phase::Source);
        assert!(diag.message.contains("hello"));
        assert!(diag.message.contains("contains 1000"));
        assert!(diag.message.contains("limit is 100"));
    }

    #[test]
    fn source_table_io_err() {
        use std::io;
        let name = "hello".to_string();
        let err = io::Error::from(io::ErrorKind::NotFound);
        let diag = Diagnostic::from(SourceTableError::Io { name, err });
        assert_eq!(diag.class, Class::SourceReadError);
        assert_eq!(diag.class.phase(), Phase::Source);
        assert!(diag.message.contains("hello"));
    }

    #[test]
    fn lex_err() {
        let mut table = SourceTable::new();
        let source = table
            .add_source_raw("t", "$12".into())
            .expect("within bound");
        let origin = SyntaxOrigin {
            source,
            span: Span::new(0, 3),
        };

        for (kind, class) in [
            (LexErrorKind::UnknownCharacter, Class::LexUnknownCharacter),
            (LexErrorKind::BindInvalid, Class::LexBindInvalid),
            (LexErrorKind::LoadInvalid, Class::LexLoadInvalid),
        ] {
            let diag = Diagnostic::from(LexError { origin, kind });
            assert_eq!(diag.class, class);
            assert_eq!(
                diag.class.phase(),
                Phase::Lex,
                "{kind:?} escaped its phase"
            );
            assert_eq!(diag.site, Site::Raw(origin));
            assert!(!diag.message.is_empty(), "{kind:?} needs a message");
        }
    }

    #[test]
    fn ice_reports_its_call_site() {
        // Without `#[track_caller]` the location would be `ice`'s own line
        // rather than this one, so pinning the line is what checks the
        // attribute is in effect.
        let line = line!() + 1;
        let diag = ice(Diagnostic::new(
            Class::ICEDroppedOutput,
            Site::None,
            "lex discarded output from a call that reported nothing",
        ));

        assert!(
            diag.message
                .contains(&format!(", detected at {}:{line}:", file!())),
            "{}",
            diag.message
        );
    }
}
