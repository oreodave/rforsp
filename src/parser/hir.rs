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
    Load(SymId),
    /// `x`, a bare name.
    Call(SymId),
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

/// Perform a Depth First Search on the given sequence of [`HirForm`]s.
///
/// On each node, call the given function `f`.
pub fn dfs(forms: &[HirForm], mut f: impl FnMut(&HirForm)) {
    let mut stack = Vec::<&HirForm>::new();
    stack.extend(forms.iter().rev());
    while let Some(form) = stack.pop() {
        f(form);
        match &form.kind {
            HirKind::List(xs) | HirKind::Vector(xs) => {
                stack.extend(xs.iter().rev());
            }
            HirKind::Quote(form) => {
                stack.push(form);
            }
            HirKind::Int(_)
            | HirKind::Bind(_)
            | HirKind::Load(_)
            | HirKind::Call(_) => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceTable, Span};

    /// A [`SyntaxId`] to hang synthetic forms off.
    ///
    /// Nothing here inspects origins, so one id serves every node.
    fn any_id() -> SyntaxId {
        let mut table = SourceTable::new();
        let source =
            table.add_source_raw("t", "x".into()).expect("within bound");
        table.add_origin(source, Span::new(0, 1))
    }

    #[test]
    fn dfs_visits_every_form() {
        let id = any_id();
        let leaf = |n| HirForm::new(id, HirKind::Int(n));

        // [1 '2 (3)] 4
        let forms = vec![
            HirForm::new(
                id,
                HirKind::Vector(vec![
                    leaf(1),
                    HirForm::new(id, HirKind::Quote(Box::new(leaf(2)))),
                    HirForm::new(id, HirKind::List(vec![leaf(3)])),
                ]),
            ),
            leaf(4),
        ];

        // Children are pushed reversed, so they come back in source order.
        // A quote's child counts: it is a child like any other, and missing
        // it is invisible until the bijection reports a compiler bug against
        // a correct program.
        let mut ints = Vec::new();
        dfs(&forms, |form| {
            if let HirKind::Int(n) = form.kind {
                ints.push(n);
            }
        });
        assert_eq!(ints, [1, 2, 3, 4]);

        // Every node exactly once, containers and the quote included.
        let mut count = 0;
        dfs(&forms, |_| count += 1);
        assert_eq!(count, 7);
    }
}
