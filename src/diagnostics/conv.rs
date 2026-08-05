//! Conversions from phase-specific Errors to Diagnostic
//!
//! Each compiler phase's internal error type which we expect to eventually
//! report to the user should have a conversion here into Diagnostic.

use crate::{
    diagnostics::{Class, Diagnostic, Site},
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
