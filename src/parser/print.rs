//! Printer for HIR

use std::fmt;

use crate::{
    interner::Interner,
    parser::{HirForm, HirKind},
};

/// Types of objects to print in the stack
enum ToPrint<'a> {
    /// Standard [`HirForm`].
    Form(&'a HirForm),
    /// End of a List.
    CloseList,
    /// End of a Vec.
    CloseVec,
}

/// Print a sequence of [`HirForm`]s into surface level rForsp syntax.
///
/// # Errors
/// - From writing to `out`.
pub fn print_forms(
    forms: &[HirForm],
    interner: &Interner,
    out: &mut impl fmt::Write,
) -> fmt::Result {
    let mut stack = Vec::<ToPrint>::new();
    stack.extend(forms.iter().map(ToPrint::Form).rev());

    let mut need_space = false;
    while let Some(form) = stack.pop() {
        // NOTE(oreo)[2026-08-16 12:32]: we can just put a single space between
        // each member and it'll parse just fine because of `RESTRICTED_CHARS`
        // as well as `WHITESPACE_CHARS`; whitespace is always trivia when
        // tokenised.
        //
        // Only a form takes a leading space.  A closer hugs what it closes,
        // and an opener or a quote clears the flag so the thing it introduces
        // hugs it in turn.  None of that is required to re-lex - it is what
        // makes the output the program the user wrote.
        if need_space && matches!(form, ToPrint::Form(_)) {
            write!(out, " ")?;
        }

        need_space = true;
        match form {
            ToPrint::CloseList => {
                write!(out, ")")?;
            }

            ToPrint::CloseVec => {
                write!(out, "]")?;
            }

            ToPrint::Form(form) => match &form.kind {
                HirKind::Int(x) => {
                    write!(out, "{x}")?;
                }
                HirKind::Bind(s) => {
                    let sym = interner.resolve(*s);
                    write!(out, "${sym}")?;
                }
                HirKind::Load(s) => {
                    let sym = interner.resolve(*s);
                    write!(out, "^{sym}")?;
                }
                HirKind::Call(s) => {
                    let sym = interner.resolve(*s);
                    write!(out, "{sym}")?;
                }
                HirKind::Quote(child) => {
                    write!(out, "'")?;
                    stack.push(ToPrint::Form(child));
                    need_space = false;
                }
                HirKind::Vector(children) => {
                    write!(out, "[")?;
                    stack.push(ToPrint::CloseVec);
                    stack.extend(children.iter().map(ToPrint::Form).rev());
                    need_space = false;
                }
                HirKind::List(children) => {
                    write!(out, "(")?;
                    stack.push(ToPrint::CloseList);
                    stack.extend(children.iter().map(ToPrint::Form).rev());
                    need_space = false;
                }
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lexer::tokenise,
        parser::parse,
        source::{SourceTable, Span, SyntaxId},
    };

    /// Lex, parse and print `text`, yielding what the printer produced.
    ///
    /// Panics unless `text` lexes and parses cleanly: a tree produced
    /// alongside diagnostics is partial by A-029 and carries no
    /// well-formedness contract, so it is not a printer input.
    fn print_of(text: &str) -> String {
        let mut table = SourceTable::new();
        let mut interner = Interner::new();
        let id = table
            .add_source_raw("t", text.into())
            .expect("within bound");
        let tokens = tokenise(id, &table).0.expect("lexes cleanly");
        let (forms, _) = parse(id, &tokens, &mut table, &mut interner);
        let forms = forms.expect("parses cleanly");

        let mut out = String::new();
        print_forms(&forms, &interner, &mut out)
            .expect("writing to a String cannot fail");
        out
    }

    /// A [`SyntaxId`] to hang synthetic forms off.
    ///
    /// The printer reads payloads and never origins, so one id serves every
    /// node and the table it came from need not outlive this call.
    fn any_id() -> SyntaxId {
        let mut table = SourceTable::new();
        let source =
            table.add_source_raw("t", "x".into()).expect("within bound");
        table.add_origin(source, Span::new(0, 1))
    }

    #[test]
    fn prints_surface_syntax() {
        // Exact output, so a change in spacing has to be made on purpose.
        // Every case below is already normalised, so the printer is the
        // identity on it and the expectation reads as the source - which is
        // the property that makes this printer usable in a diagnostic and not
        // only in a round trip.
        for (source, expected) in [
            ("", ""),
            ("1 2", "1 2"),
            ("-1 -9223372036854775808", "-1 -9223372036854775808"),
            // The three symbol kinds differ only by sigil, so a printer that
            // collapsed them would round trip vacuously.
            ("$a ^b c", "$a ^b c"),
            // Symbol material is anything outside `RESTRICTED_CHARS`, which
            // is exactly the invariant letting a bare space separate forms.
            // If that set ever shrinks, these stop round tripping first.
            ("λ +x a-b", "λ +x a-b"),
            // A quote binds its child with no separator.
            ("'x", "'x"),
            // Children must come back in source order, not stack order.
            ("[1 2 3]", "[1 2 3]"),
            ("[]", "[]"),
            ("()", "()"),
            ("'[]", "'[]"),
            // A-045 names this one: legal, and nests a quote inside a data
            // vector.
            ("'['x]", "'['x]"),
            ("(1 (2) [3])", "(1 (2) [3])"),
        ] {
            assert_eq!(
                print_of(source),
                expected,
                "printing {source:?} did not give {expected:?}"
            );
        }
    }

    #[test]
    fn printing_normalises_notation() {
        // The printer prints the tree, not the text the tree came from.
        // Everything HIR drops as notation - layout, comments, a leading
        // zero - has to be absent from the output, or the printer is echoing
        // its input somewhere and the round trip proves nothing.
        for (source, expected) in [
            ("[  1\n  2 ]", "[1 2]"),
            ("1 ; a comment\n2", "1 2"),
            ("007", "7"),
            ("'  x", "'x"),
        ] {
            assert_eq!(
                print_of(source),
                expected,
                "printing {source:?} did not normalise to {expected:?}"
            );
        }
    }

    #[test]
    fn printing_is_a_fixpoint() {
        // The property the round trip rests on: printed output re-lexes and
        // re-parses, and printing what came back changes nothing.  Anything
        // the printer drops or adds shows up here as a second print that
        // disagrees with the first - or as `print_of` failing to parse at
        // all.
        for source in [
            "1 2",
            "$a ^b c",
            "'x",
            "[1 2 3]",
            "'['x]",
            "(1 (2) [3])",
            "[$a [^b] 'c]",
            "'[]",
            "'(1)",
            "λ +x a-b",
        ] {
            let once = print_of(source);
            let twice = print_of(&once);
            assert_eq!(
                once, twice,
                "printing {source:?} is not a fixpoint: {once:?} then {twice:?}"
            );
        }
    }

    #[test]
    fn payload_is_printed_rather_than_the_span() {
        // `log_hir` prints the text a form's origin covers, which reproduces
        // the source whatever the parser actually interned.  This printer
        // must read the payload instead, or the round trip agrees with a
        // parser that interned the wrong thing.
        let mut table = SourceTable::new();
        let mut interner = Interner::new();
        let source = table
            .add_source_raw("t", "not-the-symbol".into())
            .expect("within bound");
        let id = table.add_origin(source, Span::new(0, 14));
        let sym = interner.intern("x");

        let forms = vec![HirForm::new(id, HirKind::Call(sym))];
        let mut out = String::new();
        print_forms(&forms, &interner, &mut out)
            .expect("writing to a String cannot fail");

        assert_eq!(out, "x", "printer read the origin rather than the SymId");
    }

    #[test]
    fn deep_nesting_prints() {
        // Deeper than the machine stack takes, as in `hir::tests`.  A
        // recursive printer aborts the process here rather than reporting
        // anything.  Bracket counts rather than the whole string, so this
        // stays a statement about recursion and not about spacing.
        const DEPTH: usize = 200_000;

        let id = any_id();
        let mut form = HirForm::new(id, HirKind::Int(0));
        for _ in 0..DEPTH {
            form = HirForm::new(id, HirKind::Vector(vec![form]));
        }

        let forms = vec![form];
        let mut out = String::new();
        print_forms(&forms, &Interner::new(), &mut out)
            .expect("writing to a String cannot fail");

        assert_eq!(
            out.matches('[').count(),
            DEPTH,
            "an opener per vector was not printed"
        );
        assert_eq!(
            out.matches(']').count(),
            DEPTH,
            "a closer per vector was not printed"
        );
    }
}
