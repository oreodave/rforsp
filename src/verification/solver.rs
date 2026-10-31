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
        let mut work: Vec<(ValueType, ValueType)> = Vec::new();
        work.push((a, b));
        while let Some((a_t, b_t)) = work.pop() {
            // Resolve both types before attempting to unify.
            match (self.resolve_type(a_t), self.resolve_type(b_t)) {
                // We are done if both are equivalent type variables.
                (ValueType::Var(a), ValueType::Var(b)) if a == b => {}

                // For differing type variables, we need to link them together
                // i.e. introduce constraint a = b.
                (ValueType::Var(a), ValueType::Var(b)) => {
                    *self.types.get_mut(a) = Some(ValueType::Var(b));
                }

                // For a type variable unified with a concrete type (by prior
                // cases, this must be a concrete type), we need to link the
                // type variable to that concrete type.
                (ValueType::Var(x_t), type_value)
                | (type_value, ValueType::Var(x_t)) => {
                    // By definition of `self.resolve_type`, we know that the
                    // two variants must be fully resolved.  If we still have a
                    // [`ValueType::Var`] in the mix, that means it is the final
                    // link with nothing attached to it.

                    let attached = self.types.get_mut(x_t);
                    debug_assert!(
                        attached.is_none(),
                        "Expected {x_t:?} to not have an attached type."
                    );
                    *attached = Some(type_value.clone());
                }

                // Two matching concrete types are equivalent a priori.
                (ValueType::Integer, ValueType::Integer)
                | (ValueType::List, ValueType::List)
                | (ValueType::Vector, ValueType::Vector) => {}

                // Two closure types need to have their attached effects
                // unified.
                (ValueType::Closure(_), ValueType::Closure(_)) => {
                    todo!("Implement closure type unification");
                }

                // Since all possible cases regarding actual equivalences that
                // may occur in the type algebra are covered above, any other
                // case is a contradiction.
                (_, _) => {
                    return Err(());
                }
            }
        }

        Ok(())
    }

    /// Resolve some [`ValueType`] `x` by following the chain of resolutions
    /// within this [`Solver`].
    ///
    /// This is a no-op for concrete [`ValueType`] variants.
    ///
    /// For [`ValueType::Var`] variants, continuously follow equivalences on the
    /// attached type variable.  This is done until either:
    /// - a concrete type is found, which is then returned.
    /// - otherwise, return the final [`ValueType::Var`] in the chain.
    ///
    /// In this manner, the type variable is resolve to what the [`Solver`]
    /// currently believes its equivalent type is.
    fn resolve_type(&self, x: ValueType) -> ValueType {
        let mut work = Vec::new();
        work.push(x);
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
}
