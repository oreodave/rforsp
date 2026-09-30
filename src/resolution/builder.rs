//! Lexical environment and body layout construction.

use std::collections::HashMap;

use crate::{
    interner::SymId,
    resolution::{
        BindingId, BindingTable, BodyLayout, CaptureId, CaptureSource, Target,
    },
    runtime::{PrimitiveRegistry, RuntimeVariableRegistry},
    source::SyntaxId,
};

/// State used to construct one body.
struct BodyBuilder {
    /// Frame layout accumulated for this body.
    layout: BodyLayout,
    /// Body scope followed by any active arm scopes.
    scopes: Vec<HashMap<SymId, BindingId>>,
}

impl BodyBuilder {
    /// Construct an empty body with its body scope active.
    fn new() -> Self {
        Self {
            layout: BodyLayout::new(),
            scopes: vec![HashMap::new()],
        }
    }

    /// Add a binding to the innermost lexical scope.
    fn bind(&mut self, sym: SymId, binding: BindingId) {
        self.scopes
            .last_mut()
            .expect("the body scope is always active")
            .insert(sym, binding);
    }

    /// Look up a binding within this body.
    fn lookup(&self, sym: SymId) -> Option<BindingId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&sym).copied())
    }

    /// Enter an arm-local lexical scope.
    fn enter_arm(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave the innermost arm-local lexical scope.
    fn leave_arm(&mut self) {
        assert!(
            self.scopes.len() > 1,
            "leave_arm requires an active arm scope"
        );
        let _ = self.scopes.pop();
    }

    /// Allocate one local in this body.
    const fn add_local(&mut self) -> crate::resolution::LocalId {
        self.layout.add_local()
    }

    /// Intern one capture source in this body.
    fn add_capture(&mut self, source: CaptureSource) -> CaptureId {
        self.layout.add_capture(source)
    }

    /// Finish this body and return its durable layout.
    fn finish(self) -> BodyLayout {
        assert_eq!(
            self.scopes.len(),
            1,
            "a finished body must have no active arm scopes"
        );
        self.layout
    }
}

/// In-progress lexical environment for binding resolution.
pub(super) struct Environment {
    /// Primordial names
    primordial: HashMap<SymId, Target>,
    /// Metadata for bindings minted in any body.
    bindings: BindingTable,
    /// Active bodies from entry body to innermost closure.
    bodies: Vec<BodyBuilder>,
}

impl Environment {
    /// Construct an environment with an active entry body.
    pub(super) fn new(
        var_registry: &RuntimeVariableRegistry,
        prim_registry: &PrimitiveRegistry,
    ) -> Self {
        let mut primordial = prim_registry
            .iter_syms()
            .map(|(id, sym)| (sym, Target::Primitive(id)))
            .collect::<HashMap<SymId, Target>>();

        primordial.extend(
            var_registry
                .iter_syms()
                .map(|(id, sym)| (sym, Target::Variable(id))),
        );
        Self {
            primordial,
            bindings: BindingTable::new(),
            bodies: vec![BodyBuilder::new()],
        }
    }

    /// Open a nested closure body.
    pub(super) fn open_body(&mut self) {
        self.bodies.push(BodyBuilder::new());
    }

    /// Close a nested closure body, returning the complete [`BodyLayout`].
    pub(super) fn close_body(&mut self) -> BodyLayout {
        assert!(
            self.bodies.len() > 1,
            "leave_body cannot remove the entry body"
        );
        self.bodies.pop().expect("a nested body is active").finish()
    }

    /// Enter an arm-local scope in the current body.
    pub(super) fn enter_arm(&mut self) {
        self.current_body().enter_arm();
    }

    /// Leave the current arm-local scope.
    pub(super) fn leave_arm(&mut self) {
        self.current_body().leave_arm();
    }

    /// Mint and bind a local in the current body.
    pub(super) fn bind(&mut self, origin: SyntaxId, sym: SymId) -> BindingId {
        let local = self.current_body().add_local();
        let binding = self.bindings.add(origin, local);
        self.current_body().bind(sym, binding);
        binding
    }

    /// Resolve a symbol to a specific [`Target`] given the current state of the
    /// [`Environment`].  This may mutate as captures may ripple through all the
    /// environments between the point where the symbol is first established as
    /// a local and the current top-level scope.
    pub(super) fn resolve(&mut self, sym: SymId) -> Option<Target> {
        // TODO(oreo)[2026-08-28 01:20]: Can this be faster?
        let binding = self.bodies.iter().rev().enumerate().find_map(
            |(distance, body)| {
                body.lookup(sym).map(|binding| (binding, distance))
            },
        );

        match binding {
            Some((binding, 0)) => Some(Target::Local(binding)),
            Some((binding, distance)) => {
                Some(Target::Captured(self.capture(binding, distance)))
            }
            // We try to resolve to primitives as a last resort.
            _ => self.primordial.get(&sym).copied(),
        }
    }

    /// Check if a symbol is a primordial binding within the current
    /// [`Environment`] i.e. has not been lexically bound within one of the
    /// scopes.
    pub(super) fn is_primordial(&self, sym: SymId) -> bool {
        !self
            .bodies
            .iter()
            .rev()
            .any(|body| body.lookup(sym).is_some())
            && self.primordial.contains_key(&sym)
    }

    /// Finish the entry body and return all durable environment output.
    pub(super) fn finish(mut self) -> (BindingTable, BodyLayout) {
        assert_eq!(
            self.bodies.len(),
            1,
            "all nested bodies must finish before the entry body"
        );
        let entry = self
            .bodies
            .pop()
            .expect("the entry body is always active")
            .finish();
        (self.bindings, entry)
    }

    /// Return the active body.
    fn current_body(&mut self) -> &mut BodyBuilder {
        self.bodies
            .last_mut()
            .expect("the entry body is always active")
    }

    /// Forward an enclosing binding through every intervening closure.
    fn capture(
        &mut self,
        binding: BindingId,
        body_distance: usize,
    ) -> CaptureId {
        assert!(
            body_distance < self.bodies.len(),
            "body distance must name an active enclosing body"
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Compilation,
        source::{SourceId, SourceTable, Span},
    };

    /// Origins for synthetic bindings.
    struct Origins {
        table: SourceTable,
        source: SourceId,
    }

    impl Origins {
        /// Construct an origin source.
        fn new() -> Self {
            let mut table = SourceTable::new();
            let source = table
                .add_source_raw("environment-test", "x".into())
                .expect("test source should be valid");
            Self { table, source }
        }

        /// Mint one origin.
        fn next(&mut self) -> SyntaxId {
            self.table.add_origin(self.source, Span::new(0, 1))
        }
    }

    fn local(environment: &mut Environment, sym: SymId) -> BindingId {
        environment
            .resolve(sym)
            .and_then(|target| match target {
                Target::Local(binding) => Some(binding),
                Target::Captured(_)
                | Target::Primitive(_)
                | Target::Variable(_) => None,
            })
            .expect("expected a local binding")
    }

    fn assert_captured(environment: &mut Environment, sym: SymId) {
        assert!(matches!(
            environment.resolve(sym),
            Some(Target::Captured(_))
        ));
    }

    #[test]
    fn locals_shadow() {
        let mut origins = Origins::new();
        let mut ctx = Compilation::new();
        let prim_sym = ctx.interner.intern("+");
        let var_sym = ctx.interner.intern("*stdout*");
        let mut environment = Environment::new(&ctx.variables, &ctx.primitives);

        assert!(matches!(
            environment.resolve(prim_sym),
            Some(Target::Primitive(_))
        ));
        assert!(matches!(
            environment.resolve(var_sym),
            Some(Target::Variable(_))
        ));

        let binding = environment.bind(origins.next(), prim_sym);
        assert_eq!(local(&mut environment, prim_sym), binding);

        let binding = environment.bind(origins.next(), var_sym);
        assert_eq!(local(&mut environment, var_sym), binding);
    }

    #[test]
    fn body_scopes_shadow_and_restore() {
        let mut origins = Origins::new();
        let mut ctx = Compilation::new();
        let mut environment = Environment::new(&ctx.variables, &ctx.primitives);
        let x = ctx.interner.intern("x");
        let y = ctx.interner.intern("y");

        // The newest binding is visible in its body.
        let _ = environment.bind(origins.next(), x);
        let visible = environment.bind(origins.next(), x);
        assert_eq!(local(&mut environment, x), visible);

        // A child captures that binding, then shadows it locally.
        environment.open_body();
        assert_captured(&mut environment, x);
        let inner_x = environment.bind(origins.next(), x);
        let inner_y = environment.bind(origins.next(), y);
        assert_eq!(local(&mut environment, x), inner_x);
        assert_eq!(local(&mut environment, y), inner_y);
        assert_eq!(environment.close_body().local_count(), 2);

        // Body exit restores the parent's scope.
        assert_eq!(local(&mut environment, x), visible);
        assert!(environment.resolve(y).is_none());
    }

    #[test]
    fn arm_scopes_share_the_body_layout() {
        let mut origins = Origins::new();

        let mut ctx = Compilation::new();
        let mut environment = Environment::new(&ctx.variables, &ctx.primitives);
        let outer_name = ctx.interner.intern("outer");
        let arm_name = ctx.interner.intern("arm");

        // Arm bindings use locals from the enclosing body.
        let outer = environment.bind(origins.next(), outer_name);
        environment.enter_arm();
        let arm = environment.bind(origins.next(), arm_name);
        assert_eq!(local(&mut environment, outer_name), outer);
        assert_eq!(local(&mut environment, arm_name), arm);

        // A nested body captures body and arm locals alike.
        environment.open_body();
        assert_captured(&mut environment, outer_name);
        assert_captured(&mut environment, arm_name);
        assert_eq!(environment.close_body().captures().len(), 2);

        // Arm exit removes its names but keeps their frame slots.
        environment.leave_arm();
        assert!(environment.resolve(arm_name).is_none());
        assert_eq!(environment.finish().1.local_count(), 2);
    }

    #[test]
    #[should_panic(expected = "leave_body cannot remove the entry body")]
    fn entry_body_cannot_be_left() {
        let ctx = Compilation::new();
        let mut environment = Environment::new(&ctx.variables, &ctx.primitives);
        let _ = environment.close_body();
    }
}
