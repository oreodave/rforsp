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

    /// Record a diagnostic.
    ///
    /// [`Severity::Error`] and [`Severity::Bug`] count toward
    /// [`error_count`][Diagnostics::error_count].
    pub fn push(&mut self, diag: Diagnostic) {
        if matches!(diag.class.severity(), Severity::Error | Severity::Bug) {
            self.errors += 1;
        }

        self.items.push(diag);
    }

    /// Discard everything accumulated.
    pub fn clear(&mut self) {
        self.items.clear();
        self.errors = 0;
    }

    /// Merge the given [`Diagnostics`] into the current set.
    ///
    /// The merge is unconditional: a call that succeeded may still have
    /// produced diagnostics worth keeping, and only its return value decides
    /// whether its *output* is kept.
    ///
    /// The error count crosses with the items.  [`push`][Diagnostics::push] is
    /// the only way in and counts an error exactly when it stores one, so the
    /// counts agree on both sides and adding them preserves that.
    pub fn merge(&mut self, diagnostics: Self) {
        self.errors += diagnostics.errors;
        self.items.extend(diagnostics.items);
    }

    /// The stored diagnostics in report order.
    #[must_use]
    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Whether any [`Severity::Error`]s or [`Severity::Bug`]s have been
    /// reported.
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// The number of fatal diagnostics reported.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.errors
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
    fn merge_carries_items_and_errors() {
        // A merged-in error must gate the receiver.  Carrying the items but
        // not the count would leave the gate open on diagnostics that render
        // perfectly well, which is the failure that is hardest to notice.
        let mut into = Diagnostics::new();
        let mut from = Diagnostics::new();
        from.push(Diagnostic::new(Class::SourceReadError, SITE, "a"));
        from.push(Diagnostic::new(Class::SourceReadError, SITE, "b"));

        into.merge(from);
        assert!(into.has_errors(), "a merged error must gate the receiver");
        assert_eq!(into.error_count(), 2);

        // Merging accumulates onto what is already there, in report order.
        let mut more = Diagnostics::new();
        more.push(Diagnostic::new(Class::SourceTooLarge, SITE, "c"));
        into.merge(more);
        assert_eq!(into.error_count(), 3);

        let messages: Vec<&str> =
            into.items().iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages, ["a", "b", "c"], "merge appends in report order");
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
