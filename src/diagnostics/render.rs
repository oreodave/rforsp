//! Generalised renderer for all Diagnostics.
//!
//! This is the generator of strings for a Diagnostic/collection of Diagnostics.

use std::fmt::{self, Write};

use crate::{
    diagnostics::{Class, Diagnostic, Diagnostics, Severity, Site},
    source::{SourceTable, SyntaxOrigin},
};

/// Renderer state - used to make rendering process easier.
///
/// Holds both the [`SourceTable`] diagnostics refer to and the [`Write`]
/// target they are rendered into.
pub struct Renderer<'a, W: Write> {
    /// [`SourceTable`] that [`Diagnostics`] refer to.
    table: &'a SourceTable,
    /// Destination that diagnostics are rendered into.
    out: &'a mut W,
}

/// Convert a given [`Site`] to a possible [`SyntaxOrigin`].
fn site_to_origin(table: &SourceTable, site: Site) -> Option<SyntaxOrigin> {
    match site {
        Site::None | Site::Source(_) => None,
        Site::Raw(origin) => Some(origin),
        Site::Syntax(id) => Some(*table.get_origin(id)),
    }
}

impl<'a, W: Write> Renderer<'a, W> {
    /// Construct a new Render state using the given [`SourceTable`] as backing,
    /// rendering into the given [`Write`] target.
    pub const fn new(table: &'a SourceTable, out: &'a mut W) -> Self {
        Self { table, out }
    }

    /// Render the location represented by [`Site`].
    fn render_site(&mut self, site: Site) -> fmt::Result {
        if let Site::Source(id) = site {
            let source = self.table.get_source(id);
            write!(self.out, "{}: ", source.name)?;
        } else if let Some(origin) = site_to_origin(self.table, site) {
            let location = self.table.location_of(&origin);
            write!(
                self.out,
                "{}:{}:{}: ",
                location.name, location.start.line, location.start.col
            )?;
        }
        Ok(())
    }

    /// Render the given `class`.
    fn render_class(&mut self, class: Class) -> fmt::Result {
        write!(
            self.out,
            "{}[{}]: ",
            match class.severity() {
                Severity::Note => "note",
                Severity::Warning => "warning",
                Severity::Error => "error",
            },
            class.as_code()
        )
    }
}
