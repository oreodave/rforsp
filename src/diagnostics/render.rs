//! Generalised renderer for all Diagnostics.
//!
//! This is the generator of strings for a Diagnostic/collection of Diagnostics.

use std::fmt::{self, Write};

use crate::{
    diagnostics::{Class, Diagnostic, Diagnostics, Severity, Site},
    source::{SourceId, SourceTable, SyntaxOrigin},
};

/// The default number of diagnostics that are rendered, after which diagnostics
/// are "suppressed" instead.
pub const DEFAULT_RENDERING_CAP: usize = 20;

/// Render a collection of [`Diagnostics`] related to a [`SourceTable`] into
/// `out`.
///
/// # Errors
/// - Repeated back from `write!`/`writeln!` calls.
pub fn render_diagnostics(
    diags: &Diagnostics,
    table: &SourceTable,
    out: &mut impl fmt::Write,
) -> fmt::Result {
    render_diagnostics_with_cap(diags, table, DEFAULT_RENDERING_CAP, out)
}

/// Render a collection of [`Diagnostics`] related to a [`SourceTable`] into
/// `out`.  `cap` decides how many are suppressed.
///
/// Diagnostics are rendered in source order rather than the order the phases
/// happened to report them in.
///
/// [`Severity::Bug`] is exempt from `cap` and rendered as a trailing section
/// always as they're critical if found during real word cases.  Ordering runs
/// before the cap, so what survives it is the head of the file rather than
/// whichever diagnostics happened to be reported first.
///
/// # Errors
/// - Repeated back from `write!`/`writeln!` calls.
fn render_diagnostics_with_cap(
    diags: &Diagnostics,
    table: &SourceTable,
    cap: usize,
    out: &mut impl fmt::Write,
) -> fmt::Result {
    let mut ordered: Vec<&Diagnostic> = diags.items().iter().collect();
    ordered.sort_by_key(|diag| position_of(table, diag.site));

    let ordinary = ordered
        .iter()
        .copied()
        .filter(|diag| diag.class.severity() != Severity::Bug);
    let len = ordinary.clone().count();
    let to_render = len.min(cap);
    let suppressed = len.saturating_sub(cap);

    let mut renderer = Renderer::new(table, out);
    for diag in ordinary.take(to_render) {
        renderer.render(diag)?;
    }

    let mut written = to_render > 0;
    if suppressed > 0 {
        renderer.render_suppressed(suppressed, written)?;
        written = true;
    }

    // Everything that is not ordinary is a bug, so the count settles whether
    // there are any without a second scan.
    if ordered.len() > len {
        renderer.render_bugs(&ordered, written)?;
    }

    Ok(())
}

/// The position a [`Site`] renders at, which orders Diagnostics.
fn position_of(
    table: &SourceTable,
    site: Site,
) -> Option<(SourceId, Option<u32>)> {
    match site {
        Site::None => None,
        Site::Source(id) => Some((id, None)),
        Site::Raw(origin) => Some((origin.source, Some(origin.span.start))),
        Site::Syntax(id) => {
            let origin = table.get_origin(id);
            Some((origin.source, Some(origin.span.start)))
        }
    }
}

/// Renderer state - used to make rendering process easier.
///
/// Holds both the [`SourceTable`] diagnostics refer to and the [`Write`]
/// target they are rendered into.
struct Renderer<'a, W: Write> {
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
    const fn new(table: &'a SourceTable, out: &'a mut W) -> Self {
        Self { table, out }
    }

    /// Render a singular [`Diagnostic`] into the renderer's [`Write`] target.
    ///
    /// # Errors
    /// - Repeated back from `write!`/`writeln!` calls.
    fn render(&mut self, diag: &Diagnostic) -> fmt::Result {
        self.render_site(diag.site)?;
        self.render_class(diag.class)?;
        writeln!(self.out, "{}", diag.message)?;
        self.render_snippet(diag.site)
    }

    /// Render the count of diagnostics dropped by the rendering cap.
    ///
    /// `separate` inserts a blank line first, and is set when something has
    /// already been rendered above.
    fn render_suppressed(
        &mut self,
        count: usize,
        separate: bool,
    ) -> fmt::Result {
        if separate {
            writeln!(self.out)?;
        }
        writeln!(
            self.out,
            "{count} {} suppressed",
            if count == 1 {
                "diagnostic"
            } else {
                "diagnostics"
            }
        )
    }

    /// Render every [`Severity::Bug`] among `items` as a trailing section.
    ///
    /// These are never suppressed, so this runs after the cap has been applied
    /// to everything else and renders whatever it finds.
    ///
    /// `separate` inserts a blank line first, and is set when something has
    /// already been rendered above.
    fn render_bugs(
        &mut self,
        items: &[&Diagnostic],
        separate: bool,
    ) -> fmt::Result {
        if separate {
            writeln!(self.out)?;
        }
        for diag in items
            .iter()
            .filter(|diag| diag.class.severity() == Severity::Bug)
        {
            self.render(diag)?;
        }
        writeln!(self.out, "this is a bug in rforsp; please report it")
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
            "{}[{}::{}]: ",
            match class.severity() {
                Severity::Note => "note",
                Severity::Warning => "warning",
                Severity::Error => "error",
                Severity::Bug => "bug",
            },
            class.phase().as_str(),
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
            let text = text.strip_suffix("\r").unwrap_or(text);

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
        render_diagnostics(d, t, &mut s).unwrap();
        s
    }

    fn diags_with_cap(t: &SourceTable, d: &Diagnostics, cap: usize) -> String {
        let mut s = String::new();
        render_diagnostics_with_cap(d, t, cap, &mut s).unwrap();
        s
    }

    #[test]
    fn site_geometry() {
        let mut t = SourceTable::new();
        let a = add(&mut t, "a", "hello\nworld!\n");
        let b = add(&mut t, "b", "Foo\nbar\n");
        let em = add(&mut t, "em", "ab\u{1f34e}cd\n");
        let eof = add(&mut t, "eof", "abc\ndef");
        let crlf = add(&mut t, "crlf", "hello\r\nworld!\r\n");
        let syn = t.add_origin(a, Span::new(0, 5));

        // Site::None - no location prefix.
        let s = diag(&t, Site::None, "m");
        assert!(s.contains("error[source::TOO_LARGE]: m"));

        // Site::Source - name only, no position.
        let s = diag(&t, Site::Source(b), "m");
        assert!(s.contains("b: error[source::TOO_LARGE]: m"));

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
            assert!(s.contains("a:1:1: error[source::TOO_LARGE]: m"));
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

        // The CR of a CRLF line is trivia, but it is still in the line's byte
        // slice.  Left in, it returns the terminal cursor to column 0 and the
        // caret line overwrites the source line, so the snippet trims it.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: crlf,
                span: Span::new(0, 5),
            }),
            "m",
        );
        assert!(!s.contains('\r'), "stray CR in {s:?}");
        assert!(s.contains("1 | hello\n"));
        assert!(s.contains("| ^^^^^"));

        // CR is not a line terminator, so line mapping stays LF-only: the
        // second line begins after the LF, at 2:1.
        let s = diag(
            &t,
            Site::Raw(SyntaxOrigin {
                source: crlf,
                span: Span::new(7, 13),
            }),
            "m",
        );
        assert!(s.contains("crlf:2:1: "));
        assert!(s.contains("2 | world!\n"));
        assert!(s.contains("| ^^^^^^"));
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
        let mut acc = Diagnostics::new();
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "a"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "b"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "c"));

        let s = diags_with_cap(&t, &acc, 1);
        assert!(s.contains("error[source::TOO_LARGE]: a"));
        assert!(!s.contains("error[source::TOO_LARGE]: c"));
        assert!(s.contains("2 diagnostics suppressed"));

        // With no suppression, items are separated and no summary appears.
        let mut acc = Diagnostics::new();
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "a"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "b"));
        let s = diags(&t, &acc);

        assert!(s.contains("error[source::TOO_LARGE]: a"));
        assert!(s.contains("error[source::TOO_LARGE]: b"));
        assert!(!s.contains("suppressed"));
    }

    #[test]
    fn ordering_is_by_position() {
        let mut t = SourceTable::new();
        let a = add(&mut t, "a", "hello\nworld!\n");
        let b = add(&mut t, "b", "Foo\nbar\n");
        let raw = |source, start, end| {
            Site::Raw(SyntaxOrigin {
                source,
                span: Span::new(start, end),
            })
        };

        // Reported in an order no reader would accept and no phase produces on
        // purpose: a later offset before an earlier one, a second file before
        // the first, and the positionless diagnostic last of all.
        let mut acc = Diagnostics::new();
        for (site, msg) in [
            (raw(a, 6, 11), "late"),
            (raw(b, 0, 3), "other"),
            (raw(a, 0, 5), "early"),
            (Site::None, "nowhere"),
            (Site::Source(a), "whole"),
        ] {
            acc.push(Diagnostic::new(Class::SourceTooLarge, site, msg));
        }

        let s = diags(&t, &acc);
        let at = |m: &str| s.find(&format!(": {m}\n")).expect(m);

        // A positionless diagnostic precedes every located one, a source-wide
        // one sits at the head of its own file, and sources follow the order
        // they were added in rather than the order they were reported in.
        assert!(at("nowhere") < at("whole"), "{s}");
        assert!(at("whole") < at("early"), "{s}");
        assert!(at("early") < at("late"), "{s}");
        assert!(at("late") < at("other"), "{s}");
    }

    #[test]
    fn ordering_is_stable_and_precedes_the_cap() {
        let mut t = SourceTable::new();
        let a = add(&mut t, "a", "hello\nworld!\n");
        let raw = |start, end| {
            Site::Raw(SyntaxOrigin {
                source: a,
                span: Span::new(start, end),
            })
        };

        // Two diagnostics at one position have nothing to order them by, so
        // the sort must leave them as reported rather than swap them.
        let mut acc = Diagnostics::new();
        acc.push(Diagnostic::new(Class::SourceTooLarge, raw(6, 11), "third"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, raw(0, 5), "first"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, raw(0, 5), "second"));

        let s = diags(&t, &acc);
        let at = |m: &str| s.find(&format!(": {m}\n")).expect(m);
        assert!(at("first") < at("second"), "{s}");
        assert!(at("second") < at("third"), "{s}");

        // Ordering runs before the cap, so what survives it is the head of the
        // file.  Capping first would keep whichever diagnostic a phase happened
        // to report earliest, which here is the last one in the source.
        let s = diags_with_cap(&t, &acc, 1);
        assert!(s.contains(": first\n"), "{s}");
        assert!(!s.contains(": third\n"), "{s}");
        assert!(s.contains("2 diagnostics suppressed"), "{s}");
    }

    #[test]
    fn render_bugs() {
        let t = SourceTable::new();
        let mut acc = Diagnostics::new();

        // A bug alone: nothing precedes it, so no stray leading blank line.
        acc.push(Diagnostic::new(Class::ICEDroppedOutput, Site::None, "ice"));
        let s = diags(&t, &acc);
        assert!(s.starts_with("bug[ice::DROPPED_OUTPUT]: ice"), "{s}");
        assert!(!s.contains("suppressed"), "{s}");

        // Pushed last, so under a cap of one it would be the first thing lost
        // were it subject to the cap at all.
        let mut acc = Diagnostics::new();
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "a"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "b"));
        acc.push(Diagnostic::new(Class::SourceTooLarge, Site::None, "c"));
        acc.push(Diagnostic::new(Class::ICEDroppedOutput, Site::None, "ice"));

        let s = diags_with_cap(&t, &acc, 1);
        assert!(s.contains("bug[ice::DROPPED_OUTPUT]: ice"), "{s}");
        assert!(s.contains("this is a bug in rforsp"), "{s}");

        // The cap applies to ordinary diagnostics alone, so the bug neither
        // occupies a slot nor counts towards the suppressed total.
        assert!(s.contains("error[source::TOO_LARGE]: a"), "{s}");
        assert!(!s.contains("error[source::TOO_LARGE]: b"), "{s}");
        assert!(s.contains("2 diagnostics suppressed"), "{s}");
    }
}
