//! Generalised drivers for each phase of the compiler.
//!
//! Each driver attempts every source, accumulates the diagnostics together,
//! and gates once at the phase boundary.

use crate::{
    context::Compilation,
    diagnostics::{Aborted, Class, Diagnostics, Phase, Site, conv::ice},
    lexer::{Token, TokenKind, tokenise},
    log::{Log, log_hir, log_tokens},
    parser::{HirForm, dfs, parse},
    source::SourceId,
};

/// Compile a set of `filenames`.
///
/// # Errors
/// - If a compilation phase fails
pub fn compile(
    filenames: &[String],
    ctx: &mut Compilation,
    diagnostics: &mut Diagnostics,
    log: Log,
    log_out: &mut impl std::fmt::Write,
) -> Result<(), Aborted> {
    // FIXME(oreo)[2026-08-12 15:42]: Wire in resolution, lowering,
    // verification.
    let sources = sources_from_files(filenames, diagnostics, ctx)?;
    let lexes = lex_sources(&sources, diagnostics, ctx)?;

    // `log_out` is a String at every call site, so these cannot fail.  A
    // failed log must not abort a compilation that otherwise succeeded.
    let _ = log_tokens(&sources, &lexes, log, ctx, log_out);

    let body = parse_streams(&sources, &lexes, diagnostics, ctx)?;

    let _ = log_hir(&lexes, &body, log, ctx, log_out);

    Ok(())
}

/// Pass a phase's results through only if that phase reported no errors.
///
/// The merge is unconditional.  The decision reads the phase's own
/// [`Diagnostics`] so errors from an earlier phase do not gate this one.
fn gate<T>(
    global_diags: &mut Diagnostics,
    local_diags: Diagnostics,
    container: T,
    phase: Phase,
) -> Result<T, Aborted> {
    let succeeded = !local_diags.has_errors();
    global_diags.merge(local_diags);
    if succeeded {
        Ok(container)
    } else {
        Err(Aborted::new(phase))
    }
}

/// Add a set of files to the given [`SourceTable`][crate::source::SourceTable].
///
/// # Errors
/// - If any error [`Diagnostic`][crate::diagnostics::Diagnostic]s are created
///   while adding files to the table.
fn sources_from_files(
    filenames: &[String],
    diagnostics: &mut Diagnostics,
    ctx: &mut Compilation,
) -> Result<Vec<SourceId>, Aborted> {
    let mut local = Diagnostics::new();
    let sources = filenames
        .iter()
        .filter_map(|filename| {
            ctx.table
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
/// - If any error [`Diagnostic`][crate::diagnostics::Diagnostic]s are created
///   while lexing the given sources.
fn lex_sources(
    source_ids: &[SourceId],
    diagnostics: &mut Diagnostics,
    ctx: &Compilation,
) -> Result<Vec<Vec<Token>>, Aborted> {
    let mut local = Diagnostics::new();
    let tokens_set = source_ids
        .iter()
        .filter_map(|&id| {
            let (tokens, mut lexer_diags) = tokenise(id, &ctx.table);

            // `tokenise` withholds its tokens only when it reported.  No
            // tokens and no errors means the phase dropped its output.
            if tokens.is_none() && !lexer_diags.has_errors() {
                lexer_diags.push(ice(
                    Class::ICEDroppedOutput,
                    Site::Source(id),
                    "lexing produced no tokens",
                ));
            }

            local.merge(lexer_diags);
            tokens
        })
        .collect::<Vec<_>>();

    gate(diagnostics, local, tokens_set, Phase::Lex)
}

/// Parse a collection of [`Token`] streams, merging their results all together
/// into one sequence of [`HirForm`].
///
/// The merged sequence is in the order of the [`Token`] streams given.
///
/// # Errors
/// - If any error [`Diagnostic`][crate::diagnostics::Diagnostic]s are created
///   while parsing the given [`Token`] streams.
fn parse_streams(
    source_ids: &[SourceId],
    token_streams: &[Vec<Token>],
    diagnostics: &mut Diagnostics,
    ctx: &mut Compilation,
) -> Result<Vec<HirForm>, Aborted> {
    debug_assert_eq!(
        source_ids.len(),
        token_streams.len(),
        "Expected source_ids and token_streams to be paired."
    );

    let mut local = Diagnostics::new();
    let body = source_ids
        .iter()
        .zip(token_streams.iter())
        .filter_map(|(&source_id, token_stream)| {
            let (forms, mut parse_diags) = parse(
                token_stream,
                source_id,
                &mut ctx.table,
                &mut ctx.interner,
            );

            // `parse` withholds its body only when it reported.  No body and
            // no errors means the phase dropped its output.
            if forms.is_none() && !parse_diags.has_errors() {
                parse_diags.push(ice(
                    Class::ICEDroppedOutput,
                    Site::Source(source_id),
                    "parsing produced no forms",
                ));
            }

            // Every form comes from exactly one non-closing token, so the two
            // counts agree on a clean parse.
            // `parser::parse::tests::parse_text` asserts the same pair.  The
            // two are computed separately for testing purposes.
            if let Some(forms) = &forms {
                let expected = token_stream
                    .iter()
                    .filter(|t| {
                        !matches!(
                            t.kind,
                            TokenKind::VecEnd | TokenKind::ListEnd
                        )
                    })
                    .count();
                let mut got = 0usize;
                dfs(forms, |_, _| got += 1);

                if got != expected {
                    let message = format!(
                        "parsed {got} forms from {expected} non-closing tokens"
                    );
                    parse_diags.push(ice(
                        Class::ICEDroppedOutput,
                        Site::Source(source_id),
                        message,
                    ));
                }
            }

            local.merge(parse_diags);
            forms
        })
        .flatten()
        .collect::<Vec<_>>();

    gate(diagnostics, local, body, Phase::Parse)
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Diagnostic;

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
        // Any diagnostic in the local accumulator gates.
        let mut global = Diagnostics::new();
        assert_eq!(gate(&mut global, Diagnostics::new(), 7, Phase::Lex), Ok(7));
        assert_eq!(
            gate(&mut global, dirty("a"), 7, Phase::Lex),
            Err(Aborted::new(Phase::Lex))
        );
    }

    #[test]
    fn gate_merges() {
        // The merge happens even when the caller already holds an error.
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
    fn gate_ignores_earlier_errors() {
        // An earlier phase's errors do not gate this one.  That phase gated
        // on them already.
        let mut global = dirty("earlier phase");
        assert_eq!(gate(&mut global, Diagnostics::new(), 7, Phase::Lex), Ok(7));
    }

    #[test]
    fn sources_attempts_all_files() {
        let mut ctx = Compilation::new();
        let mut diags = Diagnostics::new();
        let files = ["/nonexistent/a".to_owned(), "/nonexistent/b".to_owned()];

        assert_eq!(
            sources_from_files(&files, &mut diags, &mut ctx,),
            Err(Aborted::new(Phase::Source))
        );
        assert_eq!(
            diags.error_count(),
            2,
            "one missing file cannot hide another"
        );
    }

    #[test]
    fn lex_gates_per_source() {
        let mut ctx = Compilation::new();
        let good = ctx
            .table
            .add_source_raw("good", "1 2 add".to_owned())
            .expect("raw source");
        let bad = ctx
            .table
            .add_source_raw("bad", "^ $".to_owned())
            .expect("raw source");

        // A clean source's tokens are discarded because a sibling failed.
        let mut diags = Diagnostics::new();
        assert_eq!(
            lex_sources(&[good, bad], &mut diags, &ctx,),
            Err(Aborted::new(Phase::Lex))
        );
        assert_eq!(diags.error_count(), 2);

        let mut diags = Diagnostics::new();
        let tokens = lex_sources(&[good], &mut diags, &ctx)
            .expect("a clean source passes");
        assert_eq!(tokens[0].len(), 3);
        assert!(!diags.has_errors());
    }
}
