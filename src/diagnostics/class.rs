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

/// Classification of diagnostics.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Class {
    /// Source is too large.  Mirrors
    /// [`TooLarge`][crate::source::SourceError::TooLarge].
    SourceTooLarge,
    /// File could not be read due to IO error.  Mirrors
    /// [`Io`][crate::source::SourceTableError::Io].
    SourceReadError,
}

/// How serious a diagnostic is.
///
/// Only [`Severity::Error`] is fatal; a stage may complete successfully while
/// carrying [`Severity::Note`] and [`Severity::Warning`] diagnostics.  Ordering
/// runs from least to most severe.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Severity {
    /// Additional context attached to another diagnostic.
    Note,
    /// The program is accepted, but something is likely wrong.
    Warning,
    /// The program is rejected; this is the user's fault.
    Error,
}

impl Class {
    /// Get the [`Phase`] for this [`Class`].
    #[must_use]
    pub const fn phase(&self) -> Phase {
        match self {
            Self::SourceTooLarge | Self::SourceReadError => Phase::Source,
        }
    }

    /// Get the [`Severity`] for this [`Class`]
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::SourceTooLarge | Self::SourceReadError => Severity::Error,
        }
    }

    /// Convert Class to a stable diagnostic code.
    #[must_use]
    pub const fn as_code(&self) -> &'static str {
        match self {
            Self::SourceTooLarge => "TOO_LARGE",
            Self::SourceReadError => "I/O_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert!(Severity::Note < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }
}
