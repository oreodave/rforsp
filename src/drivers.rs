//! Generalised drivers for each phase of the compiler.

use crate::{
    diagnostics::{
        Aborted, Class, Diagnostic, Diagnostics, Phase, Site, conv::ice,
    },
    lexer::{Token, tokenise},
    source::{SourceId, SourceTable},
};

/// The gate that ensures that the results of a compiler phase only pass through
/// if the local [`Diagnostics`] of that phase has no errors.
fn gate<T>(
    global_diags: &mut Diagnostics,
    local_diags: Diagnostics,
    container: T,
    phase: Phase,
) -> Result<T, Aborted> {
    let succeeded = !local_diags.has_errors();
    global_diags.merge(local_diags);
    succeeded
        .then_some(container)
        .ok_or_else(|| Aborted::new(phase))
}

/// Add a set of files to the given [`SourceTable`].
///
/// # Errors
/// - If any error [`Diagnostic`]s are created while adding files to the table.
pub fn sources_from_files(
    filenames: &[String],
    source_table: &mut SourceTable,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<SourceId>, Aborted> {
    let mut local = Diagnostics::new();
    let sources = filenames
        .iter()
        .filter_map(|filename| {
            source_table
                .add_source_file(filename)
                .map_err(|e| local.push(e.into()))
                .ok()
        })
        .collect::<Vec<_>>();

    gate(diagnostics, local, sources, Phase::Source)
}

/// Lex a sequence of [`SourceId`] into Token Streams.
///
/// # Errors
/// - If any error [`Diagnostic`]s are created while lexing the given sources.
pub fn lex_sources(
    source_ids: &[SourceId],
    source_table: &SourceTable,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<Vec<Token>>, Aborted> {
    let mut local = Diagnostics::new();
    let tokens_set = source_ids
        .iter()
        .filter_map(|&id| {
            let (tokens, mut lexer_diags) = tokenise(id, source_table);
            if tokens.is_none() && !lexer_diags.has_errors() {
                lexer_diags.push(ice(Diagnostic::new(
                    Class::ICEDroppedOutput,
                    Site::Source(id),
                    "lexing produced no tokens",
                )));
            }
            local.merge(lexer_diags);
            tokens
        })
        .collect::<Vec<_>>();

    gate(diagnostics, local, tokens_set, Phase::Lex)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An error diagnostic with no location.
    fn err(message: &str) -> Diagnostic {
        Diagnostic::new(Class::SourceReadError, Site::None, message)
    }

    /// An accumulator holding one error.
    fn dirty(message: &str) -> Diagnostics {
        let mut d = Diagnostics::new();
        d.push(err(message));
        d
    }

    #[test]
    fn gate_polarity() {
        // `gate` will always return Err if `locals` has a Diagnostic of some
        // kind.
        let mut global = Diagnostics::new();
        assert_eq!(gate(&mut global, Diagnostics::new(), 7, Phase::Lex), Ok(7));
        assert_eq!(
            gate(&mut global, dirty("a"), 7, Phase::Lex),
            Err(Aborted::new(Phase::Lex))
        );
    }

    #[test]
    fn gate_merges() {
        // Merges always happen, regardless of whether the global diagnostics
        // already has an error.
        let mut global = Diagnostics::new();
        let mut clean = Diagnostics::new();
        clean.push(err("a"));

        assert!(gate(&mut global, clean, (), Phase::Source).is_err());
        assert!(gate(&mut global, dirty("b"), (), Phase::Lex).is_err());

        let msgs: Vec<&str> =
            global.items().iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, ["a", "b"]);
        assert_eq!(global.error_count(), 2);
    }

    #[test]
    fn gate_ok_irregardless() {
        // Errors already present in `global` will not trigger gating; we
        // presume previous calls should have gated these.
        let mut global = dirty("earlier phase");
        assert_eq!(gate(&mut global, Diagnostics::new(), 7, Phase::Lex), Ok(7));
    }

    #[test]
    fn sources_attempts_all_files() {
        let mut table = SourceTable::new();
        let mut diags = Diagnostics::new();
        let files = ["/nonexistent/a".to_owned(), "/nonexistent/b".to_owned()];

        assert_eq!(
            sources_from_files(&files, &mut table, &mut diags),
            Err(Aborted::new(Phase::Source))
        );
        assert_eq!(
            diags.error_count(),
            2,
            "one missing file cannot hide another"
        );
    }

    #[test]
    fn lex_attempts_all_sources() {
        let mut table = SourceTable::new();
        let good = table
            .add_source_raw("good", "1 2 add".to_owned())
            .expect("raw source");
        let bad = table
            .add_source_raw("bad", "^ $".to_owned())
            .expect("raw source");

        // A clean source's tokens are discarded because a sibling failed.
        let mut diags = Diagnostics::new();
        assert_eq!(
            lex_sources(&[good, bad], &table, &mut diags),
            Err(Aborted::new(Phase::Lex))
        );
        assert_eq!(diags.error_count(), 2);

        let mut diags = Diagnostics::new();
        let tokens = lex_sources(&[good], &table, &mut diags)
            .expect("a clean source passes");
        assert_eq!(tokens[0].len(), 3);
        assert!(!diags.has_errors());
    }
}
