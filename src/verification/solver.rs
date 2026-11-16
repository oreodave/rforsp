//! Constraint solver.
//!
//! The constraint solver manages a system of rFST equations then attempts to
//! "solve" them.

use crate::{
    helper::Table,
    verification::algebra::{Row, RowVar, TypeVar, ValueType},
};

/// System of constraint equations.
#[derive(Debug, Default)]
pub struct Solver {
    /// Table of type unknowns attached to a derived [`ValueType`], which is
    /// None if and only if it is still unknown.
    types: Table<TypeVar, Option<ValueType>>,
    /// Table of row unknowns attached to a derived [`Row`], which is None if
    /// and only if it is still unknown.
    rows: Table<RowVar, Option<Row>>,
}

impl Solver {
    /// Construct an empty constraint system.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Introduce a new unknown value type.
    pub fn new_type(&mut self) -> ValueType {
        let id = self.types.push(None);
        ValueType::Var(id)
    }

    /// Introduce a new unknown stack row.
    pub fn new_row(&mut self) -> RowVar {
        self.rows.push(None)
    }

    /// Unify (read: equate) the two [`ValueType`]'s given.
    ///
    /// Returns err if and only if unifying the types leads to a contradiction.
    pub fn unify_type(&mut self, a: ValueType, b: ValueType) -> Result<(), ()> {
        let mut work_stack = vec![Unify::Types(a, b)];
        while let Some(work) = work_stack.pop() {
            match work {
                Unify::Types(a_t, b_t) => {
                    self.unify_type_once(a_t, b_t, &mut work_stack)
                }
                Unify::Rows(a_r, b_r) => {
                    self.unify_row_once(a_r, b_r, &mut work_stack)
                }
            }?;
        }

        Ok(())
    }

    /// Unify the two given types "once" i.e. if further work is required, only
    /// push it onto the work stack.
    ///
    /// NOTE: Only in the case of two closure effects needing to be unified is
    /// further work required.
    fn unify_type_once(
        &mut self,
        a_t: ValueType,
        b_t: ValueType,
        work: &mut Vec<Unify>,
    ) -> Result<(), ()> {
        // Resolve both types before attempting to unify.
        match (self.resolve_type(a_t), self.resolve_type(b_t)) {
            // We are already done if both are equivalent type variables.
            (ValueType::Var(a), ValueType::Var(b)) if a == b => {}

            // For differing type variables, we need to link them together
            // i.e. introduce constraint a = b.
            (ValueType::Var(a), ValueType::Var(b)) => {
                debug_assert!(
                    self.types.get(a).is_none(),
                    "Expected {a:?} to not have an attached type"
                );
                debug_assert!(
                    self.types.get(b).is_none(),
                    "Expected {b:?} to not have an attached type"
                );
                *self.types.get_mut(a) = Some(ValueType::Var(b));
            }

            // For a type variable unified with a concrete type (by prior cases,
            // this must be a concrete type), we need to link the type variable
            // to that concrete type.
            (ValueType::Var(x_t), type_value)
            | (type_value, ValueType::Var(x_t)) => {
                // Due to [`Solver::resolve_type`], we know that the two
                // variants must be fully resolved.  If we still have a
                // [`ValueType::Var`] in the mix, that means it is the final
                // link with nothing attached to it.

                let attached = self.types.get_mut(x_t);
                debug_assert!(
                    attached.is_none(),
                    "Expected {x_t:?} to not have an attached type."
                );
                *attached = Some(type_value);
            }

            // Two matching concrete types are equivalent a priori.
            (ValueType::Integer, ValueType::Integer)
            | (ValueType::List, ValueType::List)
            | (ValueType::Vector, ValueType::Vector) => {}

            // Two closure types need to have their attached effects unified.
            (ValueType::Closure(a_e), ValueType::Closure(b_e)) => {
                // This requires further work by unifying the two rows - input
                // and output.
                work.push(Unify::Rows(a_e.output, b_e.output));
                work.push(Unify::Rows(a_e.input, b_e.input));
            }

            // Since all possible cases regarding actual equivalences that may
            // occur in the type algebra are covered above, any other case is a
            // contradiction.
            (_, _) => {
                return Err(());
            }
        }

        Ok(())
    }

    /// Unify the two given rows "once".
    fn unify_row_once(
        &mut self,
        mut a_row: Row,
        mut b_row: Row,
        work: &mut Vec<Unify>,
    ) -> Result<(), ()> {
        // If both suffixes are nonempty then unify them.
        if !(a_row.suffix.is_empty() || b_row.suffix.is_empty()) {
            // To unify both suffixes is to unify the types within each from top
            // to bottom.
            let mut new_work = Vec::new();
            while !(a_row.suffix.is_empty() || b_row.suffix.is_empty()) {
                let a_t = a_row.suffix.pop().unwrap();
                let b_t = b_row.suffix.pop().unwrap();
                new_work.push(Unify::Types(a_t, b_t));
            }
            // We push a continuation to finish unifying the row.
            new_work.push(Unify::Rows(a_row, b_row));

            work.extend(new_work.into_iter().rev());
            return Ok(());
        }

        let (empty_row, other_row) = if a_row.suffix.is_empty() {
            (a_row, b_row)
        } else {
            (b_row, a_row)
        };

        match (empty_row.prefix, other_row.prefix) {
            // `empty_row == Row::empty`, but `other_row` has a suffix;
            // unification error.
            (None, _) if !other_row.suffix.is_empty() => Err(()),

            // `empty_row == Row::empty` and `other_row == Row::empty`, so these
            // are equivalent and thus unified a priori.
            (None, None) => Ok(()),

            // `empty_row == Row::empty`, `other_row` has an empty suffix but
            // some prefix.  Recur through the prefix to prove `other_row ==
            // Row::empty`.
            (None, Some(prefix)) => match self.classify_row_var(prefix) {
                // Closed prefix => `other_row == Row::empty`.
                RowState::Closed => Ok(()),
                // Unbound prefix, which can be set to `Row::empty` =>
                // `other_row == Row::empty`.
                RowState::Unbound(row_var) => {
                    *self.rows.get_mut(row_var) = Some(Row::empty());
                    Ok(())
                }
                // `other_row != Row::empty`.
                RowState::Row(_) => Err(()),
            },

            _ => todo!("Implement"),
        }
    }

    /// Resolve some [`ValueType`] by following the chain of equivalences
    /// established within this [`Solver`].
    ///
    /// This function acts as the identity for concrete [`ValueType`] variants.
    ///
    /// For [`ValueType::Var`] variants, continuously follow equivalences on the
    /// attached type variable.  This is done until either:
    /// - a concrete type is found, which is then returned.
    /// - otherwise, return the final [`ValueType::Var`] in the chain.
    ///
    /// In this manner, the type variable is resolve to what the [`Solver`]
    /// currently believes its actual type is.
    fn resolve_type(&self, x: ValueType) -> ValueType {
        let mut work = vec![x];
        while let Some(x) = work.pop() {
            match x {
                ValueType::Var(y) => match self.types.get(y) {
                    Some(ValueType::Var(z)) => work.push(ValueType::Var(*z)),
                    Some(other) => return other.clone(),
                    None => return ValueType::Var(y),
                },
                other => return other,
            }
        }

        unreachable!(
            "All possible cases must have been covered by the loop above"
        );
    }

    /// Classify the given [`RowVar`], returning a [`RowState`].  This recurs
    /// through the prefix of the attached [`Row`].
    fn classify_row_var(&self, mut row_var: RowVar) -> RowState {
        loop {
            match self.rows.get(row_var) {
                None => return RowState::Unbound(row_var),
                Some(row) if !row.suffix.is_empty() => {
                    return RowState::Row(row);
                }
                Some(row) => match row.prefix {
                    None => return RowState::Closed,
                    Some(prefix_var) => row_var = prefix_var,
                },
            }
        }
    }
}

/// Types of pending unification work required to unify two types.  Used in
/// [`Solver::unify_type`].
#[derive(Debug)]
enum Unify {
    /// Unify two types.
    Types(ValueType, ValueType),
    /// Unify two rows.
    Rows(Row, Row),
}

#[derive(Debug)]
/// Possible states that could be returned from [`Solver::classify_row_var`].
enum RowState<'a> {
    /// Resolves to a completely closed [`Row`] i.e. [`Row::empty`].
    Closed,
    /// Resolves to an unbound [`RowVar`].  This may be the initial [`RowVar`]
    /// passed in.
    Unbound(RowVar),
    /// Resolves to some non-empty [`Row`].  This may be the attached [`Row`] to
    /// the initial [`RowVar`] passed in.
    Row(&'a Row),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_types_unify_only_with_themselves() {
        let types = [ValueType::Integer, ValueType::List, ValueType::Vector];
        for a in &types {
            for b in &types {
                let mut solver = Solver::new();
                assert_eq!(
                    solver.unify_type(a.clone(), b.clone()),
                    (a == b).then_some(()).ok_or(())
                );
            }
        }
    }

    #[test]
    fn fresh_type_variable_resolves_to_itself() {
        let mut solver = Solver::new();
        let a = solver.new_type();
        let b = solver.new_type();
        assert_ne!(a, b);
        assert_eq!(solver.resolve_type(a.clone()), a);
        assert_eq!(solver.resolve_type(b.clone()), b);
        assert_eq!(solver.unify_type(a.clone(), a.clone()), Ok(()));
        assert_eq!(solver.resolve_type(a.clone()), a);
        assert_eq!(solver.resolve_type(b.clone()), b);
    }

    #[test]
    fn variable_binds_to_concrete_type_in_either_position() {
        for concrete in [ValueType::Integer, ValueType::List, ValueType::Vector]
        {
            for variable_first in [true, false] {
                let mut solver = Solver::new();
                let variable = solver.new_type();
                let (left, right) = if variable_first {
                    (variable.clone(), concrete.clone())
                } else {
                    (concrete.clone(), variable.clone())
                };
                assert_eq!(solver.unify_type(left, right), Ok(()));
                assert_eq!(solver.resolve_type(variable.clone()), concrete);
                assert_eq!(
                    solver.unify_type(variable, concrete.clone()),
                    Ok(())
                );
            }
        }
    }

    #[test]
    fn variable_equations_propagate_through_chains() {
        let mut solver = Solver::new();
        let a = solver.new_type();
        let b = solver.new_type();
        let c = solver.new_type();
        assert_eq!(solver.unify_type(a.clone(), b.clone()), Ok(()));
        assert_eq!(solver.unify_type(b.clone(), c.clone()), Ok(()));
        assert_eq!(solver.resolve_type(a.clone()), c);
        assert_eq!(solver.unify_type(c.clone(), ValueType::Vector), Ok(()));
        for variable in [a, b, c] {
            assert_eq!(solver.resolve_type(variable), ValueType::Vector);
        }
    }

    #[test]
    fn equating_aliases_with_the_same_resolution_is_idempotent() {
        let mut solver = Solver::new();
        let a = solver.new_type();
        let b = solver.new_type();
        assert_eq!(solver.unify_type(a.clone(), b.clone()), Ok(()));
        assert_eq!(solver.unify_type(b.clone(), a.clone()), Ok(()));
        assert_eq!(solver.unify_type(a.clone(), b.clone()), Ok(()));
        assert_eq!(solver.unify_type(b, ValueType::Integer), Ok(()));
        assert_eq!(solver.resolve_type(a), ValueType::Integer);
    }

    #[test]
    fn contradictory_equations_fail_through_aliases() {
        let mut solver = Solver::new();
        let a = solver.new_type();
        let b = solver.new_type();
        assert_eq!(solver.unify_type(a.clone(), b.clone()), Ok(()));
        assert_eq!(solver.unify_type(b.clone(), ValueType::List), Ok(()));
        assert_eq!(solver.unify_type(a.clone(), ValueType::Vector), Err(()));
        assert_eq!(solver.unify_type(ValueType::Integer, b), Err(()));
        assert_eq!(solver.resolve_type(a), ValueType::List);
    }
}
