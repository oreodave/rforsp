//! Main resolution routine.

use std::collections::BTreeMap;

use crate::{
    diagnostics::Diagnostics,
    interner::{Interner, SymId},
    parser::{HirForm, HirKind},
    resolution::{
        BindingId, BindingTable, BodyLayout, CaptureId, CaptureSource,
        Resolution, Target,
        scope::{ScopeBinding, ScopeBuilder},
    },
    runtime::PrimitiveRegistry,
    source::{SourceTable, SyntaxId},
};

/// Side table of [`Resolution`]s indexed by [`SyntaxId`]s.
#[derive(Default)]
pub struct ResolutionMap {
    /// Map between [`SyntaxId`]s and their resultant [`Resolution`].
    mapping: BTreeMap<SyntaxId, Resolution>,
}

impl ResolutionMap {
    /// Get the resolution recorded for a form.
    #[must_use]
    pub fn get(&self, id: SyntaxId) -> Option<&Resolution> {
        self.mapping.get(&id)
    }

    /// Get the number of forms with a resolution entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    /// Whether the map contains no resolution entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Record a form's resolution.
    fn insert(&mut self, id: SyntaxId, resolution: Resolution) {
        assert!(
            self.mapping.insert(id, resolution).is_none(),
            "a form must have at most one resolution"
        );
    }
}

/// Complete output of binding resolution.
pub struct ResolutionResult {
    /// Resolutions for forms within the entry body.
    pub map: ResolutionMap,
    /// Metadata for every binding minted during resolution.
    pub bindings: BindingTable,
    /// Frame layout for the entry body.
    pub entry: BodyLayout,
}

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
    /// Table of bindings generated during resolution.
    bindings: BindingTable,
    /// Scope stack used for lexical lookup.
    scopes: ScopeBuilder,
    /// Stack of active bodies, including the entry body.
    bodies: Vec<BodyLayout>,
    /// Resolutions recorded so far.
    map: ResolutionMap,
}

impl Resolver {
    /// Construct resolver state for an entry body.
    fn new(registry: &PrimitiveRegistry) -> Self {
        let mut scopes = ScopeBuilder::new(registry);
        scopes.push_body();
        Self {
            bindings: BindingTable::new(),
            scopes,
            bodies: vec![BodyLayout::new()],
            map: ResolutionMap::default(),
        }
    }

    /// Resolve all forms without using the host call stack.
    fn walk(&mut self, forms: &[HirForm]) {
        let mut work = Vec::new();
        work.extend(forms.iter().rev().map(Work::Form));

        while let Some(action) = work.pop() {
            // This is a finished body, so pop the related scope and body
            // form to generate a complete entry in the [`ResolutionMap`].
            if let Work::FinishBody(id) = action {
                self.scopes.pop_body();
                let body = self
                    .bodies
                    .pop()
                    .expect("a vector finish has a matching active body");
                assert!(
                    !self.bodies.is_empty(),
                    "the entry body finishes last"
                );
                self.map.insert(id, Resolution::MakesClosure(body));
                continue;
            }

            let Work::Form(form) = action else {
                unreachable!("Dealt with in previous form");
            };

            // This is a form that needs to be resolved.
            match &form.kind {
                // These have no effect on the resolution map.
                HirKind::Int(_) | HirKind::Quote(_) | HirKind::List(_) => {}
                // Delegate to helpers
                HirKind::Bind(sym) => self.bind(form.id, *sym),
                HirKind::Load(sym) | HirKind::Call(sym) => {
                    self.reference(form.id, *sym);
                }
                // A vector is a new body (closure), and has forms to resolve
                // within that lexical scope.
                HirKind::Vector(forms) => {
                    self.scopes.push_body();
                    self.bodies.push(BodyLayout::new());
                    work.push(Work::FinishBody(form.id));
                    work.extend(forms.iter().rev().map(Work::Form));
                }
            }
        }
    }

    /// Resolve a binding given the current state of the [`Resolver`].
    fn bind(&mut self, origin: SyntaxId, sym: SymId) {
        // Mint a new binding within the current body.
        let local = self
            .bodies
            .last_mut()
            .expect("the entry body is always active")
            .add_local();
        let binding = self.bindings.add(origin, local);
        self.scopes.bind(sym, binding);

        // Put back into the map.
        self.map.insert(origin, Resolution::Bound(binding));
    }

    /// Resolve a reference to some symbol given the current state of the
    /// [`Resolver`].
    fn reference(&mut self, origin: SyntaxId, sym: SymId) {
        let Some(lookup) = self.scopes.lookup(sym) else {
            // FIXME: Add diagnostic here.
            self.map.insert(origin, Resolution::Poison);
            return;
        };

        let target = match lookup.binding {
            // If we have zero body distance, the binding is local.
            ScopeBinding::Binding(binding) if lookup.body_distance == 0 => {
                Target::Local(binding)
            }
            ScopeBinding::Binding(binding) => {
                Target::Captured(self.capture(binding, lookup.body_distance))
            }
            ScopeBinding::Primitive(primitive) => Target::Primitive(primitive),
        };

        self.map.insert(origin, Resolution::Ref(target));
    }

    /// Forward an enclosing binding through every intervening closure.
    fn capture(
        &mut self,
        binding: BindingId,
        body_distance: usize,
    ) -> CaptureId {
        assert!(
            body_distance < self.bodies.len(),
            "scope distance must name an active enclosing body"
        );

        let start = self.bodies.len() - body_distance;
        let mut source = CaptureSource::Local(self.bindings.get(binding).local);
        let mut capture = None;

        for body in &mut self.bodies[start..] {
            let slot = body.add_capture(source);
            source = CaptureSource::Captured(slot);
            capture = Some(slot);
        }

        capture.expect("a captured binding crosses at least one body")
    }

    /// Finish the entry body and return the durable result.
    fn finish(mut self) -> ResolutionResult {
        self.scopes.pop_body();
        let entry = self.bodies.pop().expect("the entry body is always active");
        assert!(self.bodies.is_empty(), "all nested bodies must be finished");
        ResolutionResult {
            map: self.map,
            bindings: self.bindings,
            entry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Compilation,
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
            let (result, diagnostics) = super::resolve(
                forms,
                &self.compilation.table,
                &self.compilation.interner,
                &self.compilation.primitives,
            );
            (
                result.expect("the standalone walk returns its partial map"),
                diagnostics,
            )
        }
    }

    fn captured(map: &ResolutionMap, id: SyntaxId) -> CaptureId {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::Ref(Target::Captured(capture)) => Some(*capture),
                _ => None,
            })
            .expect("expected a captured reference")
    }

    fn bound(map: &ResolutionMap, id: SyntaxId) -> BindingId {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::Bound(binding) => Some(*binding),
                _ => None,
            })
            .expect("expected a binding")
    }

    fn closure_layout(map: &ResolutionMap, id: SyntaxId) -> &BodyLayout {
        map.get(id)
            .and_then(|resolution| match resolution {
                Resolution::MakesClosure(layout) => Some(layout),
                _ => None,
            })
            .expect("expected a closure layout")
    }

    #[test]
    fn sequential_bindings_shadow_and_unresolved_names_poison() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let plus = fixture.sym("+");
        let before = fixture.id();
        let first_bind = fixture.id();
        let first_load = fixture.id();
        let second_bind = fixture.id();
        let second_call = fixture.id();
        let primitive = fixture.id();
        let forms = vec![
            HirForm::new(before, HirKind::Call(x)),
            HirForm::new(first_bind, HirKind::Bind(x)),
            HirForm::new(first_load, HirKind::Load(x)),
            HirForm::new(second_bind, HirKind::Bind(x)),
            HirForm::new(second_call, HirKind::Call(x)),
            HirForm::new(primitive, HirKind::Call(plus)),
        ];

        let (result, diagnostics) = fixture.resolve(&forms);
        assert!(!diagnostics.has_errors());
        assert!(matches!(result.map.get(before), Some(Resolution::Poison)));

        let first = bound(&result.map, first_bind);
        let second = bound(&result.map, second_bind);
        assert_ne!(first, second);
        assert_eq!(result.bindings.get(first).origin, first_bind);
        assert_eq!(result.bindings.get(second).origin, second_bind);
        assert_ne!(
            result.bindings.get(first).local,
            result.bindings.get(second).local
        );
        assert!(matches!(
            result.map.get(first_load),
            Some(Resolution::Ref(Target::Local(binding))) if *binding == first
        ));
        assert!(matches!(
            result.map.get(second_call),
            Some(Resolution::Ref(Target::Local(binding))) if *binding == second
        ));
        assert!(matches!(
            result.map.get(primitive),
            Some(Resolution::Ref(Target::Primitive(_)))
        ));
        assert_eq!(result.entry.local_count(), 2);
    }

    #[test]
    fn nested_demand_forwards_captures_in_depth_first_order() {
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

        let (result, diagnostics) = fixture.resolve(&forms);
        assert!(!diagnostics.has_errors());
        let y_slot = captured(&result.map, inner_y);
        let x_slot = captured(&result.map, inner_x);
        assert_ne!(y_slot, x_slot);
        assert_eq!(captured(&result.map, middle_x), x_slot);

        let middle_layout = closure_layout(&result.map, middle_vector);
        assert_eq!(middle_layout.captures().len(), 2);
        assert!(matches!(
            middle_layout.captures()[0],
            CaptureSource::Local(_)
        ));
        assert!(matches!(
            middle_layout.captures()[1],
            CaptureSource::Local(_)
        ));

        let inner_layout = closure_layout(&result.map, inner_vector);
        assert_eq!(
            inner_layout.captures(),
            [
                CaptureSource::Captured(y_slot),
                CaptureSource::Captured(x_slot),
            ]
        );
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

        let (result, diagnostics) = fixture.resolve(&forms);
        assert!(!diagnostics.has_errors());
        assert!(result.map.is_empty());
        assert_eq!(result.entry.local_count(), 0);
        assert!(result.map.get(quote).is_none());
        assert!(result.map.get(quoted_bind).is_none());
        assert!(result.map.get(list).is_none());
        assert!(result.map.get(listed_call).is_none());
    }

    #[test]
    fn deep_vector_nesting_does_not_use_the_host_stack() {
        const DEPTH: usize = 20_000;

        let mut fixture = Fixture::new();
        let mut form = HirForm::new(fixture.id(), HirKind::Int(0));
        for _ in 0..DEPTH {
            form = HirForm::new(fixture.id(), HirKind::Vector(vec![form]));
        }
        let forms = vec![form];

        let (result, diagnostics) = fixture.resolve(&forms);
        assert!(!diagnostics.has_errors());
        assert_eq!(result.map.len(), DEPTH);
    }
}
