//! Generalised drivers for each phase of the compiler.

use crate::{
    diagnostics::{Aborted, Diagnostics, Phase},
    source::{SourceId, SourceTable},
};

/// Add a set of files to the given [`SourceTable`].
///
/// # Errors
/// - If any [`Diagnostic`][crate::diagnostics::Diagnostic]s are created while
///   adding files to the table.
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

    let succeeded = !local.has_errors();

    diagnostics.merge(local);
    succeeded
        .then_some(sources)
        .ok_or(Aborted::new(Phase::Source))
}
