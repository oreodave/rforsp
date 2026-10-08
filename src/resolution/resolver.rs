//! Main resolution routine.

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    parser::{HirForm, HirKind},
    resolution::{
        ArmRole, Resolution, ResolutionError, ResolutionErrorKind,
        ResolutionMap, ResolutionResult,
        env::Environment,
        recognition::{self, Recognition},
    },
    runtime::{PrimitiveRegistry, RuntimeVariableRegistry},
    source::SyntaxId,
};

/// Resolve a complete sequence of [`HirForm`]s.
///
/// This constructs its own [`Diagnostics`] and always passes them back to the
/// caller.
#[must_use]
pub fn resolve(
    forms: &[HirForm],
    variables: &RuntimeVariableRegistry,
    primitives: &PrimitiveRegistry,
) -> (ResolutionResult, Diagnostics) {
    let mut diags = Diagnostics::new();
    let mut resolver = Resolver::new(variables, primitives, &mut diags, forms);
    resolver.walk();
    let result = resolver.finish();
    (result, diags)
}

/// An explicit traversal action.
enum Work<'forms> {
    /// A sequence of forms to resolve.
    Sequence(&'forms [HirForm]),
    /// Finish a closure body, recording it.
    FinishBody(SyntaxId),
    /// Finish a recursive closure body, recording it.
    FinishRecursiveBody(SyntaxId),
    /// A branch of a conditional that needs to be resolved
    Arm(&'forms HirForm, ArmRole),
    /// Finish the branch arm.
    FinishArm,
}

/// Resolver state machine.
struct Resolver<'diags, 'forms> {
    /// Active lexical environment and body builders.
    environment: Environment,
    /// Resolutions recorded so far.
    map: ResolutionMap,
    /// Diagnostic set that we need to add to.
    diagnostics: &'diags mut Diagnostics,
    /// Current work stack
    work: Vec<Work<'forms>>,
}

impl<'diags, 'forms> Resolver<'diags, 'forms> {
    /// Construct resolver state for an entry body.
    fn new(
        var_registry: &RuntimeVariableRegistry,
        prim_registry: &PrimitiveRegistry,
        diagnostics: &'diags mut Diagnostics,
        forms: &'forms [HirForm],
    ) -> Self {
        Self {
            environment: Environment::new(var_registry, prim_registry),
            map: ResolutionMap::default(),
            diagnostics,
            work: vec![Work::Sequence(forms)],
        }
    }

    /// Resolve all forms without using the host call stack.
    fn walk(&mut self) {
        while let Some(action) = self.work.pop() {
            match action {
                // A sequence of forms pending resolution.
                Work::Sequence(forms) => {
                    self.resolve_body(forms);
                }

                // This is a completed body, so record its complete layout.
                Work::FinishBody(id) => {
                    let body = self.environment.close_body();
                    self.map.insert(id, Resolution::MakesClosure(body));
                }

                // This is a completed recursive body, so record its complete
                // layout.
                Work::FinishRecursiveBody(id) => {
                    let body = self.environment.close_body();
                    self.map
                        .insert(id, Resolution::MakesRecursiveClosure(body));
                }

                // A branch arm requires recording a resolution of the top-level
                // then resolving its insides.
                Work::Arm(form, role) => {
                    self.map.insert(form.id, Resolution::BranchArm(role));

                    // If the arm is a vector, then we need to resolve its
                    // insides.
                    if let HirForm {
                        kind: HirKind::Vector(forms),
                        ..
                    } = form
                    {
                        self.environment.enter_arm();
                        self.work.push(Work::FinishArm);
                        self.work.push(Work::Sequence(forms));
                    }
                }

                // A fully resolved arm should already have an entry in the
                // resolution map, so we just need to clean up.
                Work::FinishArm => {
                    self.environment.leave_arm();
                }
            }
        }
    }

    /// Continue to resolve a body of `forms`.
    ///
    /// This involves resolving the topmost [`HirForm`] in the sequence and
    /// pushing the rest as Work to continue.
    fn resolve_body(&mut self, forms: &'forms [HirForm]) {
        // Check if the current sequence of forms is "recognisable".
        if let Some((recognition, remaining)) =
            recognition::recognise(forms, &self.environment)
        {
            self.work.push(Work::Sequence(remaining));
            self.resolve_recognition(recognition);
            return;
        }

        // Otherwise, we need to resolve the top most form of the current work.
        let Some((form, remaining)) = forms.split_first() else {
            return;
        };

        // Push the remaining forms onto the work stack before we resolve the
        // topmost form.
        self.work.push(Work::Sequence(remaining));

        // Resolve topmost form.
        match form {
            // Data forms have no effect on the resolution map.
            HirForm {
                kind: HirKind::Int(_) | HirKind::Quote(_) | HirKind::List(_),
                ..
            } => {}

            // A binding requires environment mutation and a Bound resolution
            // map entry.
            &HirForm {
                id,
                kind: HirKind::Bind(sym),
            } => {
                let binding = self.environment.bind(id, sym);
                self.map.insert(id, Resolution::Bound(binding));
            }

            // A reference (call or load) requires resolution in the environment
            // as well as a Reference resolution map entry.
            &HirForm {
                id,
                kind: HirKind::Load(sym) | HirKind::Call(sym),
            } => {
                let Some(target) = self.environment.resolve(sym) else {
                    self.map.insert(id, Resolution::Poison);
                    self.error(id, ResolutionErrorKind::UnresolvedSymbol);
                    return;
                };
                self.map.insert(id, Resolution::Ref(target));
            }

            // An unquoted vector is a body, which requires a new lexical scope
            // and further resolution on all its members
            HirForm {
                id,
                kind: HirKind::Vector(forms),
            } => {
                self.environment.open_body();
                self.work.push(Work::FinishBody(*id));
                self.work.push(Work::Sequence(forms));
            }
        }
    }

    /// Resolve one recognised HIR pattern.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a recognition is consumed exactly once"
    )]
    fn resolve_recognition(&mut self, recognition: Recognition<'forms>) {
        match recognition {
            Recognition::Conditional {
                then_arm,
                else_arm,
                operator,
            } => {
                // Mark the operator as a conditional
                self.map.insert(operator.id, Resolution::Conditional);
                // Push the `then` and `else` branches in REVERSE order (so the
                // `then` branch is resolved first).
                self.work.push(Work::Arm(else_arm, ArmRole::Else));
                self.work.push(Work::Arm(then_arm, ArmRole::Then));
            }

            Recognition::Recursive {
                operand_id,
                body,
                operator,
            } => {
                // Mark the operator as recursive.
                self.map.insert(operator.id, Resolution::Recursive);
                // Setup the work environment to resolve the inner body.
                self.environment.open_body();
                self.work.push(Work::FinishRecursiveBody(operand_id));
                self.work.push(Work::Sequence(body));
            }

            Recognition::RecursiveData {
                operand_id,
                operator_id,
            } => {
                self.map.insert(operator_id, Resolution::Recursive);
                self.map.insert(operand_id, Resolution::Poison);
                self.error(operand_id, ResolutionErrorKind::RecDataOperand);
            }
        }
    }

    /// Finish the entry body and return the durable result.
    fn finish(self) -> ResolutionResult {
        let (bindings, entry) = self.environment.finish();
        ResolutionResult {
            map: self.map,
            bindings,
            entry,
        }
    }

    /// Push a new error into the diagnostics set
    fn error(&mut self, origin: SyntaxId, kind: ResolutionErrorKind) {
        self.diagnostics
            .push(Diagnostic::from(ResolutionError { origin, kind }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Compilation,
        diagnostics::{Class, Site},
        interner::{SYM_IF, SYM_REC, SymId},
        resolution::{BindingId, BodyLayout, CaptureId, CaptureSource, Target},
        source::{SourceId, Span},
    };

    struct Fixture {
        compilation: Compilation,
        source: SourceId,
    }

    impl Fixture {
        fn new() -> Self {
            let mut compilation = Compilation::new();
            let source = compilation
                .table
                .add_source_raw("resolver-test", "x".into())
                .expect("test source should be valid");
            Self {
                compilation,
                source,
            }
        }

        fn id(&mut self) -> SyntaxId {
            self.compilation
                .table
                .add_origin(self.source, Span::new(0, 1))
        }

        fn sym(&mut self, name: &str) -> SymId {
            self.compilation.interner.intern(name)
        }

        fn resolve(
            &self,
            forms: &[HirForm],
        ) -> (ResolutionResult, Diagnostics) {
            super::resolve(
                forms,
                &self.compilation.variables,
                &self.compilation.primitives,
            )
        }
    }

    fn bound(map: &ResolutionMap, id: SyntaxId) -> BindingId {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::Bound(binding) => Some(*binding),
                _ => None,
            })
            .expect("expected a binding")
    }

    fn local(map: &ResolutionMap, id: SyntaxId) -> BindingId {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::Ref(Target::Local(binding)) => Some(*binding),
                _ => None,
            })
            .expect("expected a local reference")
    }

    fn captured(map: &ResolutionMap, id: SyntaxId) -> CaptureId {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::Ref(Target::Captured(capture)) => Some(*capture),
                _ => None,
            })
            .expect("expected a captured reference")
    }

    fn closure(map: &ResolutionMap, id: SyntaxId) -> &BodyLayout {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::MakesClosure(layout) => Some(layout),
                _ => None,
            })
            .expect("expected a closure layout")
    }

    #[test]
    fn data_is_not_walked() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let quoted_bind = fixture.id();
        let quote = fixture.id();
        let listed_call = fixture.id();
        let list = fixture.id();
        let forms = vec![
            HirForm::new(
                quote,
                HirKind::Quote(Box::new(HirForm::new(
                    quoted_bind,
                    HirKind::Bind(x),
                ))),
            ),
            HirForm::new(
                list,
                HirKind::List(vec![HirForm::new(
                    listed_call,
                    HirKind::Call(x),
                )]),
            ),
        ];

        // Quotes and lists are data, so neither they nor their children resolve.
        let (result, _) = fixture.resolve(&forms);
        assert!(result.map.is_empty());
        assert_eq!(result.entry.local_count(), 0);
    }

    #[test]
    fn bindings_resolve_sequentially() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let plus = fixture.sym("+");
        let before = fixture.id();
        let bind_1 = fixture.id();
        let load_1 = fixture.id();
        let bind_2 = fixture.id();
        let call_2 = fixture.id();
        let primitive = fixture.id();
        let forms = vec![
            HirForm::new(before, HirKind::Call(x)),
            HirForm::new(bind_1, HirKind::Bind(x)),
            HirForm::new(load_1, HirKind::Load(x)),
            HirForm::new(bind_2, HirKind::Bind(x)),
            HirForm::new(call_2, HirKind::Call(x)),
            HirForm::new(primitive, HirKind::Call(plus)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert!(
            diagnostics.has_errors(),
            "Expected diagnostics to have errors given call before bind"
        );

        // A reference before its bind poisons.
        assert!(matches!(result.map.get(before), Some(Resolution::Poison)));

        // Each bind is fresh and visible only to later forms.
        let first = bound(&result.map, bind_1);
        let second = bound(&result.map, bind_2);
        assert_ne!(first, second);
        assert_eq!(local(&result.map, load_1), first);
        assert_eq!(local(&result.map, call_2), second);
        assert_eq!(result.bindings.get(first).origin, bind_1);
        assert_eq!(result.bindings.get(second).origin, bind_2);
        assert_eq!(result.entry.local_count(), 2);

        // Primordial names remain available.
        assert!(matches!(
            result.map.get(primitive),
            Some(Resolution::Ref(Target::Primitive(_)))
        ));
    }

    #[test]
    fn runtime_variables_are_not_captured_unless_shadowed() {
        let mut fixture = Fixture::new();
        let stdout = fixture.sym("*stdout*");
        let variable_ref = fixture.id();
        let variable_body = fixture.id();
        let bind = fixture.id();
        let captured_ref = fixture.id();
        let captured_body = fixture.id();
        let forms = vec![
            HirForm::new(
                variable_body,
                HirKind::Vector(vec![HirForm::new(
                    variable_ref,
                    HirKind::Call(stdout),
                )]),
            ),
            HirForm::new(bind, HirKind::Bind(stdout)),
            HirForm::new(
                captured_body,
                HirKind::Vector(vec![HirForm::new(
                    captured_ref,
                    HirKind::Call(stdout),
                )]),
            ),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert!(!diagnostics.has_errors());
        assert!(matches!(
            result.map.get(variable_ref),
            Some(Resolution::Ref(Target::Variable(_)))
        ));
        assert_eq!(closure(&result.map, variable_body).captures(), []);
        assert!(matches!(
            result.map.get(captured_ref),
            Some(Resolution::Ref(Target::Captured(_)))
        ));
        assert_eq!(closure(&result.map, captured_body).captures().len(), 1);
    }

    #[test]
    fn unresolved_references_report_and_continue() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let y = fixture.sym("y");
        let missing_x = fixture.id();
        let missing_y = fixture.id();
        let vector = fixture.id();
        let bind_x = fixture.id();
        let resolved_x = fixture.id();
        let forms = vec![
            HirForm::new(missing_x, HirKind::Call(x)),
            HirForm::new(
                vector,
                HirKind::Vector(vec![HirForm::new(
                    missing_y,
                    HirKind::Load(y),
                )]),
            ),
            HirForm::new(bind_x, HirKind::Bind(x)),
            HirForm::new(resolved_x, HirKind::Call(x)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert_eq!(diagnostics.error_count(), 2);
        assert_eq!(diagnostics.items().len(), 2);
        for (diagnostic, origin) in
            diagnostics.items().iter().zip([missing_x, missing_y])
        {
            assert_eq!(diagnostic.class, Class::ResolutionUnresolvedSymbol);
            assert_eq!(diagnostic.site, Site::Syntax(origin));
        }

        assert!(matches!(
            result.map.get(missing_x),
            Some(Resolution::Poison)
        ));
        assert!(matches!(
            result.map.get(missing_y),
            Some(Resolution::Poison)
        ));
        assert!(matches!(
            result.map.get(vector),
            Some(Resolution::MakesClosure(_))
        ));
        assert_eq!(local(&result.map, resolved_x), bound(&result.map, bind_x));
    }

    #[test]
    fn captures_follow_depth_first_demand() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let y = fixture.sym("y");
        let bind_x = fixture.id();
        let bind_y = fixture.id();
        let inner_y = fixture.id();
        let inner_x = fixture.id();
        let inner_vector = fixture.id();
        let middle_x = fixture.id();
        let middle_vector = fixture.id();
        let forms = vec![
            HirForm::new(bind_x, HirKind::Bind(x)),
            HirForm::new(bind_y, HirKind::Bind(y)),
            HirForm::new(
                middle_vector,
                HirKind::Vector(vec![
                    HirForm::new(
                        inner_vector,
                        HirKind::Vector(vec![
                            HirForm::new(inner_y, HirKind::Call(y)),
                            HirForm::new(inner_x, HirKind::Call(x)),
                        ]),
                    ),
                    HirForm::new(middle_x, HirKind::Call(x)),
                ]),
            ),
        ];

        let (result, _) = fixture.resolve(&forms);

        // The inner body demands y before x, fixing the middle slot order.
        let y_slot = captured(&result.map, inner_y);
        let x_slot = captured(&result.map, inner_x);
        assert_ne!(y_slot, x_slot);
        assert!(matches!(
            closure(&result.map, middle_vector).captures(),
            [CaptureSource::Local(_), CaptureSource::Local(_)]
        ));

        // The middle body's later x reference reuses the forwarded slot.
        assert_eq!(captured(&result.map, middle_x), x_slot);

        // The inner body reads those middle captures in the same order.
        assert_eq!(
            closure(&result.map, inner_vector).captures(),
            [
                CaptureSource::Captured(y_slot),
                CaptureSource::Captured(x_slot),
            ]
        );
    }

    #[test]
    fn conditional_recognition_drives_arm_scopes_and_remainder() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let then_binding = fixture.id();
        let then_arm = fixture.id();
        let else_load = fixture.id();
        let else_arm = fixture.id();
        let operator = fixture.id();
        let trailing_binding = fixture.id();
        let forms = vec![
            HirForm::new(
                then_arm,
                HirKind::Vector(vec![HirForm::new(
                    then_binding,
                    HirKind::Bind(x),
                )]),
            ),
            HirForm::new(
                else_arm,
                HirKind::Vector(vec![HirForm::new(
                    else_load,
                    HirKind::Load(x),
                )]),
            ),
            HirForm::new(operator, HirKind::Call(SYM_IF)),
            HirForm::new(trailing_binding, HirKind::Bind(x)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert!(matches!(
            result.map.get(then_arm),
            Some(Resolution::BranchArm(ArmRole::Then))
        ));
        assert!(matches!(
            result.map.get(else_arm),
            Some(Resolution::BranchArm(ArmRole::Else))
        ));
        assert!(matches!(
            result.map.get(operator),
            Some(Resolution::Conditional)
        ));
        assert!(matches!(
            result.map.get(else_load),
            Some(Resolution::Poison)
        ));
        assert!(matches!(
            result.map.get(then_binding),
            Some(Resolution::Bound(_))
        ));
        assert!(matches!(
            result.map.get(trailing_binding),
            Some(Resolution::Bound(_))
        ));
        assert_eq!(diagnostics.error_count(), 1);
        assert_eq!(diagnostics.items()[0].site, Site::Syntax(else_load));
    }

    #[test]
    fn recursive_recognition_builds_recursive_closure_and_continues() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let outer_binding = fixture.id();
        let inner_load = fixture.id();
        let operand = fixture.id();
        let operator = fixture.id();
        let trailing_load = fixture.id();
        let forms = vec![
            HirForm::new(outer_binding, HirKind::Bind(x)),
            HirForm::new(
                operand,
                HirKind::Vector(vec![HirForm::new(
                    inner_load,
                    HirKind::Load(x),
                )]),
            ),
            HirForm::new(operator, HirKind::Call(SYM_REC)),
            HirForm::new(trailing_load, HirKind::Load(x)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        let recursive_layout = result
            .map
            .get(operand)
            .and_then(|resolution| match resolution {
                Resolution::MakesRecursiveClosure(layout) => Some(layout),
                _ => None,
            })
            .expect("expected a recursive closure layout");
        assert!(matches!(
            recursive_layout.captures(),
            [CaptureSource::Local(_)]
        ));
        assert!(matches!(
            result.map.get(operator),
            Some(Resolution::Recursive)
        ));
        assert!(matches!(
            result.map.get(inner_load),
            Some(Resolution::Ref(Target::Captured(_)))
        ));
        assert_eq!(
            local(&result.map, trailing_load),
            bound(&result.map, outer_binding)
        );
        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn recursive_data_reports_and_continues() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let operand = fixture.id();
        let operator = fixture.id();
        let trailing_binding = fixture.id();
        let forms = vec![
            HirForm::new(operand, HirKind::Int(1)),
            HirForm::new(operator, HirKind::Call(SYM_REC)),
            HirForm::new(trailing_binding, HirKind::Bind(x)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert!(matches!(result.map.get(operand), Some(Resolution::Poison)));
        assert!(matches!(
            result.map.get(operator),
            Some(Resolution::Recursive)
        ));
        assert!(matches!(
            result.map.get(trailing_binding),
            Some(Resolution::Bound(_))
        ));
        assert_eq!(diagnostics.error_count(), 1);
        assert_eq!(
            diagnostics.items()[0].class,
            Class::ResolutionRecDataOperand
        );
        assert_eq!(diagnostics.items()[0].site, Site::Syntax(operand));
    }

    #[test]
    fn shadowed_conditional_operator_uses_normal_resolution() {
        let mut fixture = Fixture::new();
        let binding = fixture.id();
        let then_data = fixture.id();
        let else_data = fixture.id();
        let operator = fixture.id();
        let forms = vec![
            HirForm::new(binding, HirKind::Bind(SYM_IF)),
            HirForm::new(then_data, HirKind::Int(1)),
            HirForm::new(else_data, HirKind::Int(0)),
            HirForm::new(operator, HirKind::Call(SYM_IF)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);

        assert_eq!(local(&result.map, operator), bound(&result.map, binding));
        assert!(result.map.get(then_data).is_none());
        assert!(result.map.get(else_data).is_none());
        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn deep_nesting_is_iterative() {
        const DEPTH: usize = 20_000;

        let mut fixture = Fixture::new();
        let mut form = HirForm::new(fixture.id(), HirKind::Int(0));
        for _ in 0..DEPTH {
            form = HirForm::new(fixture.id(), HirKind::Vector(vec![form]));
        }

        // This depth exceeds a recursive visitor's safe host stack.
        let (result, _) = fixture.resolve(&[form]);
        assert_eq!(result.map.len(), DEPTH);
    }
}
