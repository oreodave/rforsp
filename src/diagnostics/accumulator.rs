//! Diagnostics Accumulator
//!
//! The accumulator is a collection of diagnostics.  This is used in any phase
//! of the compiler that may fail.

use crate::diagnostics::{Diagnostic, Severity};

/// Accumulator of [`Diagnostic`]s.
///
/// Each compiler phase pushes [`Diagnostic`]s into this structure.
#[derive(Debug, Default)]
pub struct Diagnostics {
    /// Recorded diagnostics.
    items: Vec<Diagnostic>,
    /// Number of errors recorded
    errors: usize,
}

impl Diagnostics {
    /// Construct an empty accumulator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            errors: 0,
        }
    }

    /// The stored diagnostics in report order.
    #[must_use]
    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Whether any [`Severity::Error`]s has been reported.
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// The number of fatal diagnostics reported.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.errors
    }

    /// Record a diagnostic.
    ///
    /// [`Severity::Error`] count toward
    /// [`error_count`][Diagnostics::error_count].
    pub fn push(&mut self, diag: Diagnostic) {
        let severity = diag.class.severity();
        if severity == Severity::Error {
            self.errors += 1;
        }

        self.items.push(diag);
    }

    /// Discard everything accumulated.
    pub fn clear(&mut self) {
        self.items.clear();
        self.errors = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Class, Site};

    const SITE: Site = Site::None;

    #[test]
    fn errors_are_fatal() {
        let mut d = Diagnostics::new();
        d.push(Diagnostic::new(Class::SourceTooLarge, SITE, "x"));
        d.push(Diagnostic::new(Class::SourceReadError, SITE, "x"));
        assert!(d.has_errors());
        assert_eq!(d.error_count(), 2);
    }

    #[test]
    fn clear_resets_state() {
        let mut d = Diagnostics::new();
        d.push(Diagnostic::new(Class::SourceTooLarge, SITE, "x"));
        assert!(d.has_errors());
        d.clear();
        assert!(!d.has_errors());
        assert_eq!(d.error_count(), 0);
        assert_eq!(d.items().len(), 0);
    }
}
