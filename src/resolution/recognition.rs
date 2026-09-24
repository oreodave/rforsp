//! Local HIR pattern recognition.

use crate::{
    interner::{SYM_IF, SYM_REC},
    parser::{HirForm, HirKind},
    resolution::builder::Environment,
    source::SyntaxId,
};

/// A local HIR pattern recognised during resolution.
pub(super) enum Recognition<'a> {
    /// Recognised conditional: two operands representing branches (must be
    /// anything but a Bind, Load, or Call).
    Conditional {
        /// True branch.
        then_arm: &'a HirForm,
        /// False branch.
        else_arm: &'a HirForm,
        /// Conditional operator.
        operator: &'a HirForm,
    },
    /// Recognised recursion: operand is vector in executable position.
    Recursive {
        /// Recursive body [`SyntaxId`].
        operand_id: SyntaxId,
        /// Recursive body contents.
        body: &'a [HirForm],
        /// Recursion operator.
        operator: &'a HirForm,
    },
    /// Bad recursion: operand is definitely data and cannot be considered
    /// executable code.
    RecursiveData {
        /// [`SyntaxId`] of operand.
        operand_id: SyntaxId,
        /// [`SyntaxId`] of operator.
        operator_id: SyntaxId,
    },
}

/// Attempt to recognise some sequence of pre-determined forms, returning the
/// result of the recognition as well as the remaining forms if successful.
pub(super) fn recognise<'forms>(
    forms: &'forms [HirForm],
    env: &Environment,
) -> Option<(Recognition<'forms>, &'forms [HirForm])> {
    // Checks if the given `index` is valid in `forms`, if `forms[index]`
    // matches a `HirKind::Call` to the given `sym`, and whether `sym` itself is
    // a primitive within the current `env`.
    let is_prim_call = |index, sym| {
        forms.get(index).is_some_and(|form: &HirForm| {
            matches!(form.kind, HirKind::Call(item) if item == sym)
                && env.is_primitive(sym)
        })
    };

    if is_prim_call(1, SYM_REC) {
        recognise_recursive(forms)
    } else if is_prim_call(2, SYM_IF) {
        recognise_conditional(forms)
    } else {
        None
    }
}

/// Attempt to recognise a recursive form.
fn recognise_recursive(
    forms: &[HirForm],
) -> Option<(Recognition<'_>, &[HirForm])> {
    // We know forms[1] is SYM_REC, so we need to validate forms[0] which is the
    // operand.

    match &forms[0].kind {
        // Ideal case - the form is a vector and it's definitely in executable
        // position.
        HirKind::Vector(body) => Some((
            Recognition::Recursive {
                operand_id: forms[0].id,
                body,
                operator: &forms[1],
            },
            &forms[2..],
        )),

        // Operand is a load, call, or bind => we cannot recognise this
        // recursion as we cannot determine the data on the stack following the
        // operand.
        HirKind::Load(_) | HirKind::Call(_) | HirKind::Bind(_) => None,

        // Otherwise this is data which we can be certain ISN'T allowed before
        // recursion, so we need to ensure we record it.
        HirKind::Int(_) | HirKind::Quote(_) | HirKind::List(_) => Some((
            Recognition::RecursiveData {
                operator_id: forms[1].id,
                operand_id: forms[0].id,
            },
            &forms[2..],
        )),
    }
}

/// Attempt to recognise a conditional form.
fn recognise_conditional(
    forms: &[HirForm],
) -> Option<(Recognition<'_>, &[HirForm])> {
    // forms[2] is `SYM_IF`.  We need to validate the branches.
    for form in &forms[0..2] {
        match form.kind {
            HirKind::Bind(_) | HirKind::Load(_) | HirKind::Call(_) => {
                return None;
            }
            HirKind::Int(_)
            | HirKind::Quote(_)
            | HirKind::List(_)
            | HirKind::Vector(_) => {}
        }
    }

    Some((
        Recognition::Conditional {
            then_arm: &forms[0],
            else_arm: &forms[1],
            operator: &forms[2],
        },
        &forms[3..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Compilation,
        source::{SourceId, SourceTable, Span},
    };

    struct Fixture {
        environment: Environment,
        table: SourceTable,
        source: SourceId,
    }

    impl Fixture {
        fn new() -> Self {
            let mut compilation = Compilation::new();
            let source = compilation
                .table
                .add_source_raw("recognition-test", "x".into())
                .expect("test source should be valid");
            let environment = Environment::new(&compilation.primitives);
            Self {
                environment,
                table: compilation.table,
                source,
            }
        }

        fn form(&mut self, kind: HirKind) -> HirForm {
            let id = self.table.add_origin(self.source, Span::new(0, 1));
            HirForm::new(id, kind)
        }
    }

    #[test]
    fn recognises_recursive_vector_and_remainder() {
        let mut fixture = Fixture::new();

        let body_form = fixture.form(HirKind::Int(1));
        let body_form_id = body_form.id;

        let operand = fixture.form(HirKind::Vector(vec![body_form]));
        let operand_id = operand.id;

        let operator = fixture.form(HirKind::Call(SYM_REC));
        let operator_id = operator.id;

        let trailing = fixture.form(HirKind::Int(2));
        let trailing_id = trailing.id;

        let forms = vec![operand, operator, trailing];

        let (recognition, remaining) = recognise(&forms, &fixture.environment)
            .expect("recursive vector should match");

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, trailing_id);
        assert!(
            matches!(&recognition, Recognition::Recursive { .. }),
            "expected recursive recognition"
        );

        let Recognition::Recursive {
            operand_id: recognised_operand,
            body,
            operator,
        } = recognition
        else {
            unreachable!();
        };

        assert_eq!(recognised_operand, operand_id);
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].id, body_form_id);
        assert_eq!(operator.id, operator_id);
    }

    #[test]
    fn recognises_recursive_data_operands() {
        let mut fixture = Fixture::new();
        let quoted = fixture.form(HirKind::Int(1));
        let kinds = [
            HirKind::Int(0),
            HirKind::Quote(Box::new(quoted)),
            HirKind::List(Vec::new()),
        ];

        for kind in kinds {
            let operand = fixture.form(kind);
            let operand_id = operand.id;
            let operator = fixture.form(HirKind::Call(SYM_REC));
            let operator_id = operator.id;
            let forms = vec![operand, operator];

            let (recognition, remaining) =
                recognise(&forms, &fixture.environment).expect(
                    "data before built-in rec should match its error path",
                );
            assert!(remaining.is_empty());
            assert!(matches!(
                recognition,
                Recognition::RecursiveData {
                    operand_id: found_operand,
                    operator_id: found_operator,
                } if found_operand == operand_id && found_operator == operator_id
            ));
        }
    }

    #[test]
    fn computed_recursive_operands_keep_the_dynamic_path() {
        let mut fixture = Fixture::new();
        let sym = SYM_IF;

        for kind in [HirKind::Bind(sym), HirKind::Load(sym), HirKind::Call(sym)]
        {
            let operand = fixture.form(kind);
            let operator = fixture.form(HirKind::Call(SYM_REC));
            let forms = vec![operand, operator];
            assert!(recognise(&forms, &fixture.environment).is_none());
        }
    }

    #[test]
    fn recognises_conditional_arms_and_remainder() {
        let mut fixture = Fixture::new();

        let quoted = fixture.form(HirKind::Int(2));

        let then_body = fixture.form(HirKind::Int(1));
        let then_arm = fixture.form(HirKind::Vector(vec![then_body]));
        let then_id = then_arm.id;

        let else_arm = fixture.form(HirKind::Quote(Box::new(quoted)));
        let else_id = else_arm.id;

        let operator = fixture.form(HirKind::Call(SYM_IF));
        let operator_id = operator.id;

        let trailing = fixture.form(HirKind::Int(3));
        let trailing_id = trailing.id;

        let forms = vec![then_arm, else_arm, operator, trailing];

        let (recognition, remaining) = recognise(&forms, &fixture.environment)
            .expect("conditional should match");

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, trailing_id);
        assert!(
            matches!(&recognition, Recognition::Conditional { .. }),
            "expected conditional recognition"
        );

        let Recognition::Conditional {
            then_arm,
            else_arm,
            operator,
        } = recognition
        else {
            unreachable!();
        };

        assert_eq!(then_arm.id, then_id);
        assert_eq!(else_arm.id, else_id);
        assert_eq!(operator.id, operator_id);
    }

    #[test]
    fn environment_dependent_forms_are_not_conditional_arms() {
        let mut fixture = Fixture::new();
        let sym = SYM_IF;

        for kind in [HirKind::Bind(sym), HirKind::Load(sym), HirKind::Call(sym)]
        {
            let then_arm = fixture.form(kind);
            let else_arm = fixture.form(HirKind::Int(0));
            let operator = fixture.form(HirKind::Call(SYM_IF));
            let forms = vec![then_arm, else_arm, operator];
            assert!(recognise(&forms, &fixture.environment).is_none());
        }

        for kind in [HirKind::Bind(sym), HirKind::Load(sym), HirKind::Call(sym)]
        {
            let then_arm = fixture.form(HirKind::Int(0));
            let else_arm = fixture.form(kind);
            let operator = fixture.form(HirKind::Call(SYM_IF));
            let forms = vec![then_arm, else_arm, operator];
            assert!(recognise(&forms, &fixture.environment).is_none());
        }
    }

    #[test]
    fn shadowed_operators_are_not_recognised() {
        let mut fixture = Fixture::new();
        let bind_if = fixture.form(HirKind::Int(0)).id;
        let _ = fixture.environment.bind(bind_if, SYM_IF);

        let then_arm = fixture.form(HirKind::Int(1));
        let else_arm = fixture.form(HirKind::Int(2));
        let operator = fixture.form(HirKind::Call(SYM_IF));

        let forms = vec![then_arm, else_arm, operator];
        assert!(recognise(&forms, &fixture.environment).is_none());

        let bind_rec = fixture.form(HirKind::Int(0)).id;
        let _ = fixture.environment.bind(bind_rec, SYM_REC);

        let operand = fixture.form(HirKind::Vector(Vec::new()));
        let operator = fixture.form(HirKind::Call(SYM_REC));

        let forms = vec![operand, operator];
        assert!(recognise(&forms, &fixture.environment).is_none());
    }
}
