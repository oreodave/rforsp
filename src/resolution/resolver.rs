//! Main resolution routine.

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    interner::Interner,
    parser::{HirForm, HirKind},
    resolution::{
        ArmRole, Resolution, ResolutionError, ResolutionErrorKind,
        ResolutionMap, ResolutionResult,
        builder::Environment,
        recognition::{self, Recognition},
    },
    runtime::PrimitiveRegistry,
    source::{SourceTable, SyntaxId},
};

/// Resolve a complete sequence of [`HirForm`]s.
///
/// This constructs its own [`Diagnostics`] and always passes them back to the
/// caller.
#[must_use]
pub fn resolve(
    forms: &[HirForm],
    _source_table: &SourceTable,
    _interner: &Interner,
    primitives: &PrimitiveRegistry,
) -> (Option<ResolutionResult>, Diagnostics) {
    let mut diags = Diagnostics::new();
    let mut resolver = Resolver::new(primitives, &mut diags);
    resolver.walk(forms);
    let result = resolver.finish();
    (Some(result), diags)
}

/// An explicit traversal action.
enum Work<'forms> {
    /// A body of forms to resolve.
    Body(&'forms [HirForm]),
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
struct Resolver<'diags> {
    /// Active lexical environment and body builders.
    environment: Environment,
    /// Resolutions recorded so far.
    map: ResolutionMap,
    /// Diagnostic set that we need to add to.
    diagnostics: &'diags mut Diagnostics,
}

impl<'diags> Resolver<'diags> {
    /// Construct resolver state for an entry body.
    fn new(
        registry: &PrimitiveRegistry,
        diagnostics: &'diags mut Diagnostics,
    ) -> Self {
        Self {
            environment: Environment::new(registry),
            map: ResolutionMap::default(),
            diagnostics,
        }
    }

    /// Resolve all forms without using the host call stack.
    fn walk<'forms>(&mut self, forms: &'forms [HirForm]) {
        let mut work: Vec<Work<'forms>> = vec![Work::Body(forms)];
        while let Some(action) = work.pop() {
            match action {
                // A sequence of forms pending resolution.
                Work::Body(forms) => {
                    // Check if the current sequence of forms is "recognisable".
                    if let Some((recognition, remaining)) =
                        recognition::recognise(forms, &self.environment)
                    {
                        work.push(Work::Body(remaining));
                        self.resolve_recognition(recognition, &mut work);
                        continue;
                    }

                    // Otherwise, we need to resolve the top most form of the
                    // current work.
                    let Some((form, remaining)) = forms.split_first() else {
                        continue;
                    };

                    // Push the remaining forms onto the work stack before we
                    // resolve this form.
                    work.push(Work::Body(remaining));

                    // Resolve topmost form.
                    self.resolve_form(form, &mut work);
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

                // A to-be-resolved arm requires recording that it's a branch
                // arm then resolving its innards.
                Work::Arm(form, role) => {
                    self.map.insert(form.id, Resolution::BranchArm(role));
                    if let HirForm {
                        kind: HirKind::Vector(forms),
                        ..
                    } = form
                    {
                        self.environment.enter_arm();
                        work.push(Work::FinishArm);
                        work.push(Work::Body(forms));
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

    /// Resolve a form, mutating the resolution map and potentially adding extra
    /// work to the work stack if required.
    fn resolve_form<'forms>(
        &mut self,
        form: &'forms HirForm,
        work: &mut Vec<Work<'forms>>,
    ) {
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
            // and further work on all its members
            HirForm {
                id,
                kind: HirKind::Vector(forms),
            } => {
                self.environment.open_body();
                // This is a marker to ensure the main loop actually adds a
                // resolution map entry for this body once it is fully resolved.
                work.push(Work::FinishBody(*id));
                work.push(Work::Body(forms));
            }
        }
    }

    /// Resolve one recognised HIR pattern.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a recognition is consumed exactly once"
    )]
    fn resolve_recognition<'forms>(
        &mut self,
        recognition: Recognition<'forms>,
        work: &mut Vec<Work<'forms>>,
    ) {
        match recognition {
            Recognition::Conditional {
                then_arm,
                else_arm,
                operator,
            } => {
                // Mark the operator as a conditional
                self.map.insert(operator.id, Resolution::Conditional);
                // Push the then and else branches in REVERSE order (so the
                // `then` branch is resolved first).
                work.push(Work::Arm(else_arm, ArmRole::Else));
                work.push(Work::Arm(then_arm, ArmRole::Then));
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
                work.push(Work::FinishRecursiveBody(operand_id));
                work.push(Work::Body(body));
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
            let (results, diagnostics) = super::resolve(
                forms,
                &self.compilation.table,
                &self.compilation.interner,
                &self.compilation.primitives,
            );
            (results.expect("Resolution never returns None"), diagnostics)
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
