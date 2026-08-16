//! Generalised drivers for each phase of the compiler.
//!
//! These are the top level drivers that thread the various phases of the
//! compiler together.

use crate::{
    context::Compilation,
    diagnostics::{Aborted, Class, Diagnostics, Phase, Site, conv::ice},
    lexer::{Token, tokenise},
    source::SourceId,
};

/// Level of logs from [`compile`].
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Log {
    /// no logs.
    None,
    /// print a log of the tokens
    Tokens,
}

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
    // FIXME(oreo)[2026-08-12 15:42]: Wire in parsing, resolution, lowering,
    // verification.
    let sources = sources_from_files(filenames, diagnostics, ctx)?;
    let lexes = lex_sources(&sources, diagnostics, ctx)?;

    let _ = log_tokens(&sources, &lexes, log, ctx, log_out);

    Ok(())
}

/// The gate that ensures the results of a compiler phase only pass through if
/// the local [`Diagnostics`] of that phase has no errors.
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

            // Internal compiler invariant
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

/// Log tokens if and only if `log` == [`Log::Tokens`].
fn log_tokens(
    sources: &[SourceId],
    lexes: &[Vec<Token>],
    log: Log,
    ctx: &Compilation,
    log_out: &mut impl std::fmt::Write,
) -> std::fmt::Result {
    if log == Log::Tokens {
        for (&id, lex_stream) in sources.iter().zip(lexes) {
            let source = ctx.table.get_source(id);
            writeln!(
                log_out,
                "{}: {} bytes => {} tokens",
                source.name,
                source.len(),
                lex_stream.len()
            )?;
            for token in lex_stream {
                let kind = token.kind;
                let text = source.span_text(token.span);
                write!(log_out, "{kind:?}({text}), ")?;
            }
            writeln!(log_out)?;
        }
    }
    Ok(())
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
