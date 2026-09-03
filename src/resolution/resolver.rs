//! Main resolution routine.

use crate::{
    diagnostics::Diagnostics,
    interner::{Interner, SymId},
    parser::{HirForm, HirKind},
    resolution::{
        Resolution, ResolutionMap, ResolutionResult, builder::Environment,
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
    let mut resolver = Resolver::new(primitives);
    resolver.walk(forms);
    let result = resolver.finish();
    (Some(result), Diagnostics::new())
}

/// An explicit traversal action.
enum Work<'a> {
    /// Resolve one form.
    Form(&'a HirForm),
    /// Finish the closure body belonging to this vector.
    FinishBody(SyntaxId),
}

/// Resolver state machine.
struct Resolver {
    /// Active lexical environment and body builders.
    environment: Environment,
    /// Resolutions recorded so far.
    map: ResolutionMap,
}

impl Resolver {
    /// Construct resolver state for an entry body.
    fn new(registry: &PrimitiveRegistry) -> Self {
        Self {
            environment: Environment::new(registry),
            map: ResolutionMap::default(),
        }
    }

    /// Resolve all forms without using the host call stack.
    fn walk(&mut self, forms: &[HirForm]) {
        let mut work = Vec::new();
        work.extend(forms.iter().rev().map(Work::Form));

        while let Some(action) = work.pop() {
            match action {
                // Data forms (integers, quotes, lists) have no affect on the
                // resolution map.
                Work::Form(HirForm {
                    kind: HirKind::Int(_) | HirKind::Quote(_) | HirKind::List(_),
                    ..
                }) => {}

                // Binding a symbol within the current scope.
                Work::Form(&HirForm {
                    id,
                    kind: HirKind::Bind(sym),
                }) => self.bind(id, sym),

                // Referencing a symbol.
                Work::Form(&HirForm {
                    id,
                    kind: HirKind::Load(sym) | HirKind::Call(sym),
                }) => self.reference(id, sym),

                // An unquoted vector is a body, so must have all its members
                // resolved first within their own lexical scope.
                Work::Form(HirForm {
                    id,
                    kind: HirKind::Vector(forms),
                }) => {
                    self.environment.open_body();
                    // We push this first so once all the member forms are
                    // resolved we can close this body.
                    work.push(Work::FinishBody(*id));
                    work.extend(forms.iter().rev().map(Work::Form));
                }

                // This is a completed body, so pop the related body form to
                // generate a complete entry in the [`ResolutionMap`].
                Work::FinishBody(id) => {
                    let body = self.environment.close_body();
                    self.map.insert(id, Resolution::MakesClosure(body));
                }
            }
        }
    }

    /// Resolve a binding in the current environment.
    fn bind(&mut self, origin: SyntaxId, sym: SymId) {
        let binding = self.environment.bind(origin, sym);
        self.map.insert(origin, Resolution::Bound(binding));
    }

    /// Resolve a reference in the current environment.
    fn reference(&mut self, origin: SyntaxId, sym: SymId) {
        let Some(target) = self.environment.resolve(sym) else {
            // FIXME(oreo)[2026-09-03 15:48]: Add diagnostic here.
            self.map.insert(origin, Resolution::Poison);
            return;
        };
        self.map.insert(origin, Resolution::Ref(target));
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
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Compilation,
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

        fn resolve(&self, forms: &[HirForm]) -> ResolutionResult {
            let (result, diagnostics) = super::resolve(
                forms,
                &self.compilation.table,
                &self.compilation.interner,
                &self.compilation.primitives,
            );
            assert!(!diagnostics.has_errors());
            result.expect("the standalone walk returns its partial result")
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
        let result = fixture.resolve(&forms);
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

        let result = fixture.resolve(&forms);

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

        let result = fixture.resolve(&forms);

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
        let result = fixture.resolve(&[form]);
        assert_eq!(result.map.len(), DEPTH);
    }
}
