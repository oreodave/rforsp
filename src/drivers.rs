//! Generalised drivers for each phase of the compiler.

use crate::{
    diagnostics::{Aborted, Diagnostics, Phase},
    lexer::{Token, tokenise},
    source::{SourceId, SourceTable},
};

/// The gate that ensures that the results of a compiler phase only pass through
/// if the [`Diagnostics`] of that phase have no errors.
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
/// - If any error [`Diagnostic`][crate::diagnostics::Diagnostic]s are created
///   while adding files to the table.
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
/// - If any error [`Diagnostic`][crate::diagnostics::Diagnostic]s are created
///   while lexing the given sources.
pub fn lex_sources(
    source_ids: &[SourceId],
    source_table: &SourceTable,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<Vec<Token>>, Aborted> {
    let mut local = Diagnostics::new();
    let tokens_set = source_ids
        .iter()
        .filter_map(|&id| {
            let (tokens, local_1) = tokenise(id, source_table);
            local.merge(local_1);
            tokens
        })
        .collect::<Vec<_>>();

    gate(diagnostics, local, tokens_set, Phase::Lex)
}
