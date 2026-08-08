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

    /// Render a collection of [`Diagnostics`] into the renderer's [`Write`]
    /// target.
    ///
    /// # Errors
    /// - Repeated back from `write!`/`writeln!` calls.
    pub fn render_all(&mut self, diags: &Diagnostics) -> fmt::Result {
        for diag in diags.items() {
            self.render(diag)?;
        }

        if diags.suppressed() > 0 {
            if !diags.items().is_empty() {
                writeln!(self.out)?;
            }
            writeln!(
                self.out,
                "{} {} suppressed",
                diags.suppressed(),
                if diags.suppressed() == 1 {
                    "diagnostic"
                } else {
                    "diagnostics"
                }
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceId, Span};

    fn add(t: &mut SourceTable, name: &str, text: &str) -> SourceId {
        t.add_source_raw(name, text.into()).unwrap()
    }

    fn diag(t: &SourceTable, site: Site, msg: &str) -> String {
        let mut s = String::new();
        Renderer::new(t, &mut s)
            .render(&Diagnostic::new(Class::SourceTooLarge, site, msg))
            .unwrap();
        s
    }

    fn diags(t: &SourceTable, d: &Diagnostics) -> String {
        let mut s = String::new();
        Renderer::new(t, &mut s).render_all(d).unwrap();
        s
    }

    #[test]
    fn site_geometry() {
        let mut t = SourceTable::new();
        let a = add(&mut t, "a", "hello\nworld!\n");
        let b = add(&mut t, "b", "Foo\nbar\n");
        let em = add(&mut t, "em", "ab\u{1f34e}cd\n");
        let eof = add(&mut t, "eof", "abc\ndef");
        let syn = t.add_origin(a, Span::new(0, 5));

        // Site::None - no location prefix.
        let s = diag(&t, Site::None, "m");
        assert!(s.contains("error[TOO_LARGE]: m"));

        // Site::Source - name only, no position.
        let s = diag(&t, Site::Source(b), "m");
        assert!(s.contains("b: error[TOO_LARGE]: m"));

        // Site::Raw and Site::Syntax for the same span render identically, and
        // both carry the [CODE] token, error label and a single-line caret.
        let raw = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: a,
                span: Span::new(0, 5),
            }),
            "m",
        );
        let syntax = diag(&t, Site::Syntax(syn), "m");
        assert_eq!(raw, syntax);
        for s in [&raw, &syntax] {
            assert!(s.contains("a:1:1: error[TOO_LARGE]: m"));
            assert!(s.contains("1 | hello"));
            assert!(s.contains("| ^^^^^"));
        }

        // Empty span: a single caret at the position.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: a,
                span: Span::new(2, 2),
            }),
            "m",
        );
        assert!(s.contains("a:1:3: "));
        assert!(s.contains("|   ^"));

        // Multi-byte codepoint before the span: carets align by character.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: em,
                span: Span::new(6, 8),
            }),
            "m",
        );
        assert!(s.contains("1 | ab\u{1f34e}cd"));
        assert!(s.contains("|    ^^"));

        // EOF-touching span.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: eof,
                span: Span::new(4, 7),
            }),
            "m",
        );
        assert!(s.contains("2 | def"));
        assert!(s.contains("| ^^^"));
    }

    #[test]
    fn multiline() {
        let mut t = SourceTable::new();
        let ml = add(&mut t, "ml", "l1\nl2\nl3\nl4\n");

        // Non-adjacent start/end: both lines shown with an elision line.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: ml,
                span: Span::new(0, 12),
            }),
            "m",
        );
        assert!(s.contains("1 | l1"));
        assert!(s.contains("4 | l4"));
        assert_eq!(s.matches("...").count(), 1);

        // Adjacent start/end: both lines shown, no elision.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: ml,
                span: Span::new(0, 6),
            }),
            "m",
        );
        assert!(s.contains("1 | l1"));
        assert!(s.contains("2 | l2"));
        assert_eq!(s.matches("...").count(), 0);
    }

    #[test]
    fn render_all() {
        let t = SourceTable::new();

        // With a cap, overflow diagnostics are suppressed and summarised.
        let mut acc = Diagnostics::with_cap(1);
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "a"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "b"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "c"));
        assert_eq!(acc.items().len(), 1);
        assert_eq!(acc.suppressed(), 2);
        let s = diags(&t, &acc);
        assert!(s.contains("error[TOO_LARGE]: a"));
        assert!(!s.contains("error[TOO_LARGE]: c"));
        assert!(s.contains("2 diagnostics suppressed"));

        // With no suppression, items are separated and no summary appears.
        let mut acc = Diagnostics::new();
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "a"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "b"));
        let s = diags(&t, &acc);

        assert!(s.contains("error[TOO_LARGE]: a"));
        assert!(s.contains("error[TOO_LARGE]: b"));
        assert!(!s.contains("suppressed"));
    }
}
