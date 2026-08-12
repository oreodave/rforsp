//! HIR - the source-shaped IR the parser produces.
//!
//! It preserves every distinction the source makes and drops only notation, so
//! a diagnostic pointing at an [`HirForm`] points at something the user
//! recognises as their own program.
//!
//! HIR knows a [`SymId`] for every name, a [`SyntaxId`] - and therefore a
//! span - for every form, and the syntactic category of every form.

use crate::{interner::SymId, source::SyntaxId};

/// Syntactic category of an [`HirForm`].
#[derive(Debug, PartialEq, Eq)]
pub enum HirKind {
    /// An integer literal, converted.
    Int(i64),
    /// `'f`, wrapping exactly the next form.
    Quote(Box<HirForm>),
    /// `( .. )`, self-evaluating data.
    List(Vec<HirForm>),
    /// `[ .. ]`
    Vector(Vec<HirForm>),
    /// `$x`
    Bind(SymId),
    /// `^x`
    LoadRef(SymId),
    /// `x`, a bare name.
    CallRef(SymId),
}

/// A single HIR form: a syntactic category plus its origin.
///
/// A body of code is, implicitly, a `Vec<HirForm>` since rForsp is
/// concatenative.
#[derive(Debug, PartialEq, Eq)]
pub struct HirForm {
    /// Origin of this form in the
    /// [`SourceTable`][crate::source::SourceTable].
    pub id: SyntaxId,
    /// Syntactic category of this form.
    pub kind: HirKind,
}

impl HirForm {
    /// Construct a form of `kind` originating at `id`.
    #[must_use]
    pub const fn new(id: SyntaxId, kind: HirKind) -> Self {
        Self { id, kind }
    }
}
