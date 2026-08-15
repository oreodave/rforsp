//! Parser from token streams.
//!
//! This is the core routine which compiles a stream of [`Token`]s to
//! [`HirForm`]s.

use crate::{
    diagnostics::Diagnostics,
    interner::Interner,
    lexer::Token,
    parser::HirForm,
    source::{SourceId, SourceTable},
};

/// Parse the token stream of the source given by [`SourceId`] in a
/// [`SourceTable`], returning the body of [`HirForm`]s if no errors arise,
/// otherwise [`None`].
///
/// This constructs its own [`Diagnostics`] which is always passed back to the
/// caller.
///
/// `table` is taken mutably because every form registers its origin through
/// [`SourceTable::add_origin`]; no [`Source`][crate::source::Source] reference
/// may be held across that call.
#[must_use]
#[expect(clippy::todo, unused_variables, reason = "phase 2 stub")]
pub fn parse(
    source_id: SourceId,
    tokens: &[Token],
    table: &mut SourceTable,
    interner: &mut Interner,
) -> (Option<Vec<HirForm>>, Diagnostics) {
    todo!("phase 2: the container stack")
}
