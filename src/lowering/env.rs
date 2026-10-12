//! Lexical scopes and environments.
//!
//! This helper provides binding information for [`Body`]s as well as
//! construction helpers for capturing bindings across several nested [`Body`]s.

use std::collections::HashMap;

use crate::{
    interner::SymId,
    lowering::{
        Body, BodyId, CaptureId, CaptureSource, LocalId, Module, Target,
    },
    runtime::{PrimitiveRegistry, RuntimeVariableRegistry},
};

/// A Scope is a mapping between [`SymId`] and [`LocalId`] within the [`Body`]
/// being processed.
type Scope = HashMap<SymId, LocalId>;

/// A frame within the [`Environment`].
/// This correlates to a single [`Body`] being processed.
struct Frame {
    /// [`BodyId`] that this frame is attached to.
    body: BodyId,
    /// Sequence of [`Scope`]s, with the last one being the currently active
    /// [`Scope`].
    scopes: Vec<Scope>,
}

impl Frame {
    /// Construct an empty [`Frame`] with only the top-most scope active.
    fn new(body: BodyId) -> Self {
        Self {
            body,
            scopes: vec![Scope::new()],
        }
    }

    /// Bind a symbol with given [`SymId`] to given [`LocalId`] within the
    /// current active [`Scope`] of this [`Frame`].
    fn bind(&mut self, sym: SymId, local: LocalId) {
        self.scopes
            .last_mut()
            .expect("top-most scope should always be active")
            .insert(sym, local);
    }

    /// Look up a binding within this body.
    fn lookup(&self, sym: SymId) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&sym).copied())
    }

    /// Allocate a new capture within this [`Frame`]'s body relative to the
    /// given [`Module`].
    fn allocate_capture(
        &self,
        source: CaptureSource,
        module: &mut Module,
    ) -> CaptureId {
        module.bodies.get_mut(self.body).add_capture(source)
    }

    /// Construct a nested lexical scope
    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave the top most lexical scope
    fn leave_scope(&mut self) {
        assert!(
            self.scopes.len() > 1,
            "leave_arm requires an active arm scope"
        );
        let _ = self.scopes.pop();
    }
}

/// Environment structure for lexical resolution during lowering.
pub struct Environment {
    /// Primordial environment using the [`Target::Primitive`] and
    /// [`Target::Variable`] variants.
    primordial: HashMap<SymId, Target>,
    /// Currently active frames.
    frames: Vec<Frame>,
}

impl Environment {
    /// Construct an [`Environment`] with one active [`Frame`] representing the
    /// "entry".
    pub fn new(
        var_registry: &RuntimeVariableRegistry,
        prim_registry: &PrimitiveRegistry,
    ) -> Self {
        let primordial = prim_registry
            .iter_syms()
            .map(|(id, sym)| (sym, Target::Primitive(id)))
            .chain(
                var_registry
                    .iter_syms()
                    .map(|(id, sym)| (sym, Target::Variable(id))),
            )
            .collect::<HashMap<SymId, Target>>();

        Self {
            primordial,
            frames: vec![Frame::new(BodyId::default())],
        }
    }

    /// Return the [`BodyId`] of the currently active [`Frame`].
    pub fn current_body(&self) -> BodyId {
        self.frames
            .last()
            .expect("the entry frame is always active")
            .body
    }

    /// Open a nested [`Frame`].
    pub fn open_frame(&mut self, id: BodyId) {
        self.frames.push(Frame::new(id));
    }

    /// Close a nested [`Frame`], returning its associated [`BodyId`].
    pub fn close_frame(&mut self) -> BodyId {
        assert!(
            !self.frames.is_empty(),
            "close_frame requires a frame available"
        );
        self.frames.pop().expect("a nested frame is active").body
    }

    /// Enter an arm-local scope in the current body.
    pub fn enter_scope(&mut self) {
        self.current_frame().enter_scope();
    }

    /// Leave the current arm-local scope.
    pub fn leave_scope(&mut self) {
        self.current_frame().leave_scope();
    }

    /// Bind [`SymId`] to a [`LocalId`] within the current [`Frame`].
    pub fn bind(&mut self, sym: SymId, local: LocalId) {
        self.current_frame().bind(sym, local);
    }

    /// Resolve a symbol to a specific [`Target`] within the current
    /// [`Environment`].
    ///
    /// This may mutate the layouts of the associated bodies of nested frames
    /// between the point where the symbol is last bound and the current frame,
    /// ensuring each layout captures that binding.
    pub fn resolve(&self, sym: SymId, module: &mut Module) -> Option<Target> {
        // TODO(oreo)[2026-08-28 01:20]: Can this be faster?
        let binding = self.frames.iter().rev().enumerate().find_map(
            |(distance, frame)| {
                frame.lookup(sym).map(|local| (local, distance))
            },
        );

        match binding {
            Some((local, 0)) => Some(Target::Local(local)),
            Some((local, distance)) => {
                Some(Target::Captured(self.capture(local, distance, module)))
            }
            // We try to resolve to primitives as a last resort.
            _ => self.primordial.get(&sym).copied(),
        }
    }

    /// Check if a symbol is a primordial binding within the current
    /// [`Environment`] i.e. has not been lexically bound within one of the
    /// scopes.
    pub fn is_primordial(&self, sym: SymId) -> bool {
        self.primordial.contains_key(&sym)
            && !self
                .frames
                .iter()
                .rev()
                .any(|frame| frame.lookup(sym).is_some())
    }

    /// Return the active [`Frame`].
    fn current_frame(&mut self) -> &mut Frame {
        self.frames
            .last_mut()
            .expect("the entry frame is always active")
    }

    /// Forward an enclosing binding through every intervening [`Frame`].
    fn capture(
        &self,
        local: LocalId,
        frame_distance: usize,
        module: &mut Module,
    ) -> CaptureId {
        assert!(
            frame_distance > 0,
            "frame_distance must be nontrivial to require a capture waterfall"
        );

        assert!(
            frame_distance < self.frames.len(),
            "frame distance must name an active enclosing body"
        );

        let start = self.frames.len() - frame_distance;
        // Relative to the starting frame, the binding is a local and is not a
        // capture.
        let mut source = CaptureSource::Local(local);
        let mut capture = None;

        for frame in &self.frames[start..] {
            // For each frame following the starting frame, allocate a capture.
            let slot = frame.allocate_capture(source, module);
            // mutate `source` and `capture` to propagate captures across the
            // interleaved frames.
            source = CaptureSource::Captured(slot);
            capture = Some(slot);
        }

        capture.expect("a captured binding crosses at least one body")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Compilation;

    /// Compare resolved targets by variant and slot without changing the IR API.
    fn assert_target(actual: Option<Target>, expected: Option<Target>) {
        let equal = match (actual, expected) {
            (None, None) => true,
            (Some(Target::Local(a)), Some(Target::Local(b))) => a == b,
            (Some(Target::Captured(a)), Some(Target::Captured(b))) => a == b,
            (Some(Target::Primitive(a)), Some(Target::Primitive(b))) => a == b,
            (Some(Target::Variable(a)), Some(Target::Variable(b))) => a == b,
            _ => false,
        };
        assert!(equal, "expected {expected:?}, got {actual:?}");
    }

    /// Resolve an enclosing binding and extract its capture slot.
    fn captured(
        env: &Environment,
        sym: SymId,
        module: &mut Module,
    ) -> CaptureId {
        match env.resolve(sym, module) {
            Some(Target::Captured(id)) => id,
            other => panic!("expected a captured reference, got {other:?}"),
        }
    }

    #[test]
    fn scopes_shadow_locals_and_restore_visibility() {
        let mut compilation = Compilation::new();
        let mut env =
            Environment::new(&compilation.variables, &compilation.primitives);
        let mut module = Module::new();
        let x = compilation.interner.intern("x");
        let y = compilation.interner.intern("y");
        let outer = module.get_entry_mut().locals.allocate();
        let inner = module.get_entry_mut().locals.allocate();
        let scoped = module.get_entry_mut().locals.allocate();

        assert_target(env.resolve(x, &mut module), None);
        env.bind(x, outer);
        assert_target(env.resolve(x, &mut module), Some(Target::Local(outer)));
        env.enter_scope();
        assert_target(env.resolve(x, &mut module), Some(Target::Local(outer)));
        env.bind(x, inner);
        env.bind(y, scoped);
        assert_target(env.resolve(x, &mut module), Some(Target::Local(inner)));
        assert_target(env.resolve(y, &mut module), Some(Target::Local(scoped)));
        env.leave_scope();
        assert_target(env.resolve(x, &mut module), Some(Target::Local(outer)));
        assert_target(env.resolve(y, &mut module), None);
        assert_eq!(module.get_entry().locals.len(), 3);
        assert_eq!(module.get_entry().captures().len(), 0);
    }

    #[test]
    fn equal_local_slots_in_distinct_bodies_do_not_alias() {
        let mut compilation = Compilation::new();
        let mut env =
            Environment::new(&compilation.variables, &compilation.primitives);
        let mut module = Module::new();
        let x = compilation.interner.intern("x");
        let y = compilation.interner.intern("y");
        let outer = module.get_entry_mut().locals.allocate();
        env.bind(x, outer);
        let child = module.bodies.push(Body::new(None));
        let inner = module.bodies.get_mut(child).locals.allocate();
        assert_eq!(outer, inner);

        env.open_frame(child);
        env.bind(y, inner);
        assert_target(env.resolve(y, &mut module), Some(Target::Local(inner)));
        let capture = captured(&env, x, &mut module);
        assert_eq!(captured(&env, x, &mut module), capture);
        assert_eq!(
            module.bodies.get(child).captures().collect::<Vec<_>>(),
            [CaptureSource::Local(outer)]
        );
        env.bind(x, inner);
        assert_target(env.resolve(x, &mut module), Some(Target::Local(inner)));
        assert_eq!(env.close_frame(), child);
        assert_target(env.resolve(x, &mut module), Some(Target::Local(outer)));
        assert_target(env.resolve(y, &mut module), None);
        assert_eq!(module.get_entry().captures().len(), 0);
        assert_eq!(env.close_frame(), BodyId::default());
        assert!(
            env.frames.is_empty(),
            "closing entry exhausts active frames"
        );
    }

    #[test]
    fn nested_demand_forwards_captures_through_an_unused_intermediary() {
        let mut compilation = Compilation::new();
        let mut env =
            Environment::new(&compilation.variables, &compilation.primitives);
        let mut module = Module::new();
        let x = compilation.interner.intern("x");
        let y = compilation.interner.intern("y");
        let x_local = module.get_entry_mut().locals.allocate();
        let y_local = module.get_entry_mut().locals.allocate();
        env.bind(x, x_local);
        env.bind(y, y_local);
        let middle = module.bodies.push(Body::new(None));
        env.open_frame(middle);
        let middle_y = captured(&env, y, &mut module);
        let leaf = module.bodies.push(Body::new(None));
        env.open_frame(leaf);

        let leaf_x = captured(&env, x, &mut module);
        let leaf_y = captured(&env, y, &mut module);
        assert_eq!(captured(&env, x, &mut module), leaf_x);
        assert_eq!(captured(&env, y, &mut module), leaf_y);
        assert_ne!(leaf_x, leaf_y);
        assert_eq!(env.close_frame(), leaf);
        let middle_x = captured(&env, x, &mut module);
        assert_ne!(middle_x, middle_y);
        assert_eq!(
            module.bodies.get(middle).captures().collect::<Vec<_>>(),
            [CaptureSource::Local(y_local), CaptureSource::Local(x_local)]
        );
        assert_eq!(
            module.bodies.get(leaf).captures().collect::<Vec<_>>(),
            [
                CaptureSource::Captured(middle_x),
                CaptureSource::Captured(middle_y)
            ]
        );
        assert_eq!(module.get_entry().captures().len(), 0);
    }

    #[test]
    fn capture_starts_at_the_nearest_defining_frame() {
        let mut compilation = Compilation::new();
        let mut env =
            Environment::new(&compilation.variables, &compilation.primitives);
        let mut module = Module::new();
        let x = compilation.interner.intern("x");
        let outer = module.get_entry_mut().locals.allocate();
        env.bind(x, outer);
        let middle = module.bodies.push(Body::new(None));
        let inner = module.bodies.get_mut(middle).locals.allocate();
        env.open_frame(middle);
        env.bind(x, inner);
        let leaf = module.bodies.push(Body::new(None));
        env.open_frame(leaf);

        let _ = captured(&env, x, &mut module);
        assert_eq!(
            module.bodies.get(leaf).captures().collect::<Vec<_>>(),
            [CaptureSource::Local(inner)]
        );
        assert_eq!(module.bodies.get(middle).captures().len(), 0);
        assert_eq!(module.get_entry().captures().len(), 0);
    }

    #[test]
    fn primordial_bindings_are_captured_only_when_lexically_shadowed() {
        let mut compilation = Compilation::new();
        let mut env =
            Environment::new(&compilation.variables, &compilation.primitives);
        let mut module = Module::new();
        let (primitive, primitive_sym) = compilation
            .primitives
            .iter_syms()
            .next()
            .expect("builtins include a primitive");
        let (variable, variable_sym) = compilation
            .variables
            .iter_syms()
            .next()
            .expect("builtins include a runtime variable");
        let missing = compilation.interner.intern("missing");
        assert!(
            !env.is_primordial(missing),
            "unbound names are not primordial"
        );
        assert_target(env.resolve(missing, &mut module), None);

        for (sym, target) in [
            (primitive_sym, Target::Primitive(primitive)),
            (variable_sym, Target::Variable(variable)),
        ] {
            let child = module.bodies.push(Body::new(None));
            env.open_frame(child);
            assert!(env.is_primordial(sym), "the builtin is initially visible");
            assert_target(env.resolve(sym, &mut module), Some(target));
            assert_eq!(module.bodies.get(child).captures().len(), 0);
            assert_eq!(env.close_frame(), child);

            env.enter_scope();
            let local = module.get_entry_mut().locals.allocate();
            env.bind(sym, local);
            assert!(!env.is_primordial(sym), "a local shadows the builtin");
            assert_target(
                env.resolve(sym, &mut module),
                Some(Target::Local(local)),
            );
            env.open_frame(child);
            assert!(
                !env.is_primordial(sym),
                "an enclosing local shadows the builtin"
            );
            let _ = captured(&env, sym, &mut module);
            assert_eq!(
                module.bodies.get(child).captures().collect::<Vec<_>>(),
                [CaptureSource::Local(local)]
            );
            assert_eq!(env.close_frame(), child);
            env.leave_scope();
            assert!(
                env.is_primordial(sym),
                "leaving the scope restores the builtin"
            );
            assert_target(env.resolve(sym, &mut module), Some(target));
        }
        assert_eq!(module.get_entry().captures().len(), 0);
    }
}
