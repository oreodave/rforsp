//! Diagnostics Accumulator
//!
//! The accumulator is a collection of diagnostics.  This is used in any phase
//! of the compiler that may fail.

use crate::diagnostics::{Diagnostic, Severity};

/// The default number of diagnostics that are retained in the accumulator,
/// after which diagnostics are suppressed instead.
pub const DEFAULT_DIAGNOSTIC_CAP: usize = 20;

/// Accumulator of [`Diagnostic`]s.
///
/// Each compiler phase pushes [`Diagnostic`]s into this structure.
#[derive(Debug, Default)]
pub struct Diagnostics {
    /// Recorded diagnostics.
    items: Vec<Diagnostic>,
    /// Number of errors recorded, including suppressed.
    errors: usize,
    /// Number of suppressed errors.
    suppressed: usize,
    /// Capacity for diagnostics before suppression.
    cap: usize,
}

impl Diagnostics {
    /// Construct an empty accumulator with the
    /// [`default cap`][DEFAULT_DIAGNOSTIC_CAP].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            cap: DEFAULT_DIAGNOSTIC_CAP,
            errors: 0,
            suppressed: 0,
        }
    }

    /// Construct an accumulator with the given `cap`.
    #[must_use]
    pub const fn with_cap(cap: usize) -> Self {
        Self {
            items: Vec::new(),
            cap,
            errors: 0,
            suppressed: 0,
        }
    }

    /// The stored (never suppressed) diagnostics, in report order.
    #[must_use]
    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Whether any [`Severity::Error`]s has been reported.
    ///
    /// This is the phase gate: a stage that produced errors must not hand its
    /// output to the next stage.
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// The number of fatal diagnostics reported, including suppressed ones.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.errors
    }

    /// The number of diagnostics dropped past the cap.
    #[must_use]
    pub const fn suppressed(&self) -> usize {
        self.suppressed
    }

    /// Record a diagnostic.
    ///
    /// [`Severity::Error`] count toward
    /// [`error_count`][Diagnostics::error_count].  Below the cap the diagnostic
    /// is stored - past it, diagnostics are counted as suppressed and dropped.
    pub fn push(&mut self, diag: Diagnostic) {
        let severity = diag.class.severity();
        if severity == Severity::Error {
            self.errors += 1;
        }

        if self.items.len() >= self.cap {
            self.suppressed += 1;
        } else {
            self.items.push(diag);
        }
    }

    /// Discard everything accumulated.
    pub fn clear(&mut self) {
        self.items.clear();
        self.errors = 0;
        self.suppressed = 0;
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
    fn cap_suppresses_and_counts() {
        let mut d = Diagnostics::with_cap(2);
        for i in 0..5 {
            d.push(Diagnostic::new(Class::SourceReadError, SITE, "x"));
        }
        assert_eq!(d.error_count(), 5);
        assert_eq!(d.items().len(), 2);
        assert_eq!(d.suppressed(), 3);
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
        assert_eq!(d.suppressed(), 0);
    }
}
