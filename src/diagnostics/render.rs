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

/// Different types of snippet lines to render.
#[derive(Copy, Clone)]
enum LineRole {
    /// This is the only line in the snippet.
    Only,
    /// This is the first line in the snippet.
    First,
    /// This is the last line in the snippet.
    Last,
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

    /// Render the source snippet for a diagnostic: the lines of source the span
    /// lies in, with carets marking the span.
    ///
    /// Only [`Site::Raw`] / [`Site::Syntax`] sites produce a snippet;
    /// [`Site::None`] and source-wide [`Site::Source`] render nothing.
    ///
    /// A span crossing adjacent lines renders them separately, but a span
    /// crossing several lines renders its start and end lines joined by an
    /// elision line.
    fn render_snippet(&mut self, site: Site) -> fmt::Result {
        let Some(origin) = site_to_origin(self.table, site) else {
            return Ok(());
        };

        let (start_line, end_line) = self.table.lines_of(&origin);
        let location = self.table.location_of(&origin);
        let gutter_width = end_line.to_string().len();
        let padding = " ".repeat(gutter_width);

        let lines = if start_line == end_line {
            vec![(start_line, LineRole::Only)]
        } else {
            vec![(start_line, LineRole::First), (end_line, LineRole::Last)]
        };

        writeln!(self.out, "{padding} |")?;
        for (i, &(line, role)) in lines.iter().enumerate() {
            let text = self.table.line_text(origin.source, line);

            // Write the text
            writeln!(self.out, "{line:>gutter_width$} | {text}")?;

            // We now need to compute what to highlight - we derive this from
            // the LineRole.
            let line_end = text.chars().count() + 1;
            let end_col = if location.end.line == line {
                location.end.col
            } else {
                line_end
            };

            let (from, to) = match role {
                LineRole::Only => (location.start.col, end_col),
                LineRole::First => (location.start.col, line_end),
                LineRole::Last => (1, end_col),
            };
            let col = from - 1;
            let width = to.saturating_sub(from).max(1);
            let spaces = " ".repeat(col);
            let carets = "^".repeat(width);

            // Write the carets highlighting the text
            writeln!(self.out, "{padding} | {spaces}{carets}")?;

            // Write a continuation line ("...") if and only if the start and
            // end lines are not adjacent.
            if i == 0 && start_line + 1 < end_line {
                writeln!(self.out, "{padding} | ...")?;
            }
        }

        writeln!(self.out, "{padding} |")?;
        Ok(())
    }

    /// Render a singular [`Diagnostic`] into the renderer's [`Write`] target.
    ///
    /// # Errors
    /// - Repeated back from `write!`/`writeln!` calls.
    pub fn render(&mut self, diag: &Diagnostic) -> fmt::Result {
        self.render_site(diag.site)?;
        self.render_class(diag.class)?;
        writeln!(self.out, "{}", diag.message)?;
        self.render_snippet(diag.site)
    }
}
