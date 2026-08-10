//! Conversions from phase-specific Errors to Diagnostic
//!
//! Each compiler phase's internal error type which we expect to eventually
//! report to the user should have a conversion here into Diagnostic.

use crate::{
    diagnostics::{Class, Diagnostic, Site},
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(diag.message.contains("hello"));
        assert!(diag.message.contains("contains 1000"));
        assert!(diag.message.contains("limit is 100"));
    }

    #[test]
    fn source_table_err() {
        let source_error = sample_source_error();
        let diag = Diagnostic::from(SourceTableError::from(source_error));
        assert_eq!(diag.class, Class::SourceTooLarge);
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
        assert!(diag.message.contains("hello"));
    }
}
