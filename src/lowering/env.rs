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
