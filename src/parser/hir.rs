//! Source shaped IR the parser produces.
//!
//! This is the first top level representation of a user program.  This maps to
//! the generalised semihomoiconic AST of rForsp as a language.

use std::mem;

use crate::{interner::SymId, source::SyntaxId};

/// Syntactic category of an [`HirForm`].
#[derive(Debug)]
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
#[derive(Debug)]
pub struct HirForm {
    /// Origin of this form in the
    /// [`SourceTable`][crate::source::SourceTable].
    pub id: SyntaxId,
    /// Syntactic category of this form.
    pub kind: HirKind,
}

impl HirKind {
    /// The kind left behind when a form's own kind is taken from it.
    ///
    /// Any childless variant would do; this one is the cheapest to construct
    /// and to drop.
    const STOLEN: Self = Self::Int(0);

    /// Whether this kind owns any [`HirForm`]s.
    #[must_use]
    pub const fn has_children(&self) -> bool {
        matches!(self, Self::Quote(_) | Self::List(_) | Self::Vector(_))
    }

    /// Get a labelling string.
    #[must_use]
    pub const fn label_str(&self) -> &'static str {
        match self {
            Self::Int(_) => "Int",
            Self::Quote(_) => "Quote",
            Self::List(_) => "List",
            Self::Vector(_) => "Vector",
            Self::Bind(_) => "Bind",
            Self::Load(_) => "Load",
            Self::Call(_) => "Call",
        }
    }
}

impl HirForm {
    /// Construct a form of `kind` originating at `id`.
    #[must_use]
    pub const fn new(id: SyntaxId, kind: HirKind) -> Self {
        Self { id, kind }
    }
}

impl Drop for HirForm {
    /// Dismantle this [`HirForm`] iteratively.
    ///
    /// A derived Drop naively recurs through all forms that potentially own
    /// other [`HirForm`]s.  On pathological inputs this will overflow the stack
    /// which is not good behaviour.
    ///
    /// This manual implementation iterates through children rather than
    /// recurring, bypassing the machine stack.  This avoids the stack overflow
    /// possibility entirely.
    fn drop(&mut self) {
        // A leaf owns no forms, so the glue is already safe for it and
        // allocating a worklist per leaf would dominate the cost of freeing
        // a tree.
        if !self.kind.has_children() {
            return;
        }

        let mut worklist = vec![mem::replace(&mut self.kind, HirKind::STOLEN)];
        while let Some(kind) = worklist.pop() {
            match kind {
                HirKind::Quote(mut boxed) => {
                    worklist
                        .push(mem::replace(&mut boxed.kind, HirKind::STOLEN));
                    // `boxed` now holds a leaf, so dropping it here returns
                    // immediately rather than descending.
                }
                HirKind::List(mut forms) | HirKind::Vector(mut forms) => {
                    for form in &mut forms {
                        worklist.push(mem::replace(
                            &mut form.kind,
                            HirKind::STOLEN,
                        ));
                    }
                }
                HirKind::Int(_)
                | HirKind::Bind(_)
                | HirKind::Load(_)
                | HirKind::Call(_) => (),
            }
        }
    }
}

/// Perform a Depth First Search on the given sequence of [`HirForm`]s.
///
/// On each node, call the given function `f` with that node and its depth,
/// counting the forms in `forms` as depth zero.
pub fn dfs(forms: &[HirForm], mut f: impl FnMut(&HirForm, usize)) {
    let mut stack = Vec::<(&HirForm, usize)>::new();
    stack.extend(forms.iter().rev().map(|form| (form, 0)));
    while let Some((form, depth)) = stack.pop() {
        f(form, depth);
        let child = depth + 1;
        match &form.kind {
            HirKind::List(xs) | HirKind::Vector(xs) => {
                stack.extend(xs.iter().rev().map(|x| (x, child)));
            }
            HirKind::Quote(form) => {
                stack.push((&**form, child));
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
        dfs(&forms, |form, _| {
            if let HirKind::Int(n) = form.kind {
                ints.push(n);
            }
        });
        assert_eq!(ints, [1, 2, 3, 4]);

        // Every node exactly once, containers and the quote included.
        let mut count = 0;
        dfs(&forms, |_, _| count += 1);
        assert_eq!(count, 7);

        // Depth is per-entry, so it must fall back to zero for `4` after the
        // vector's subtree rather than continuing to climb.  A quote raises
        // depth like any other parent.
        let mut depths = Vec::new();
        dfs(&forms, |_, depth| depths.push(depth));
        assert_eq!(depths, [0, 1, 1, 2, 1, 2, 0]);
    }

    #[test]
    fn deep_nesting_walks_and_drops() {
        // Deeper than the compiler's own drop glue, or a recursive walk,
        // survives on a test thread's stack.  Both are under test: the walk
        // here, and the drop of `forms` when this function returns.  A
        // recursive version of either aborts the process rather than
        // reporting anything.
        const DEPTH: usize = 200_000;

        let id = any_id();
        let mut form = HirForm::new(id, HirKind::Int(0));
        for _ in 0..DEPTH {
            form = HirForm::new(id, HirKind::Vector(vec![form]));
        }

        let forms = vec![form];
        let mut count = 0usize;
        dfs(&forms, |_, _| count += 1);
        assert_eq!(count, DEPTH + 1);
    }
}
