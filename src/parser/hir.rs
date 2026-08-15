//! Source shaped IR the parser produces.
//!
//! This is the first top level representation of a user program.  This maps to
//! the generalised semihomoiconic AST of rForsp as a language.

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
