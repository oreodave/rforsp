//! Generalised Diagnostic Type
//!
//! This module describes the Diagnostic type - a singular "thing to report to
//! the user".

use crate::diagnostics::Class;
use crate::source::{SourceId, SyntaxId, SyntaxOrigin};

/// Location of a diagnostic.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Site {
    /// No location: a missing file, a bad argument, any phase-0 failure that
    /// precedes a [`SourceId`].
    None,
    /// A whole source, with no meaningful position within it.
    Source(SourceId),
    /// A byte range in a source, for stages running before HIR exists.
    Raw(SyntaxOrigin),
    /// A form that already has an entry in the source table.
    Syntax(SyntaxId),
}

/// A single reported condition.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Classification.
    pub class: Class,
    /// Root message.
    pub message: String,
    /// Location.
    pub site: Site,
}

impl Diagnostic {
    /// Construct a diagnostic of the given `class` at `site` with `message`.
    #[must_use]
    pub fn new(class: Class, site: Site, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
            site,
        }
    }
}
