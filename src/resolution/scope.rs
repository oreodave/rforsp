//! Scopes for closure analysis.

use std::collections::HashMap;

use crate::{
    interner::SymId,
    resolution::BindingId,
    runtime::{PrimitiveId, PrimitiveRegistry},
};

/// Binding within a scope.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ScopeBinding {
    /// It's a primitive.
    Primitive(PrimitiveId),
    /// It's a general binding.
    Binding(BindingId),
}

/// Types of scopes that may be created.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ScopeKind {
    /// Primordial scope - initialised at the start, cannot be popped.  Should
    /// not be constructed by hand.
    Primordial,
    /// Body scope - part of a genuine new body.
    Body,
    /// Arm scope - part of a conditional branch.
    Arm,
}

/// A scope is a map between [`SymId`] and [`ScopeBinding`], lexically closed
/// under itself and its parents.
struct Scope(HashMap<SymId, ScopeBinding>);

/// Generalised builder of scopes.
pub struct ScopeBuilder {
    /// Scopes being built.
    scopes: Vec<Scope>,
    /// Kinds of scope.
    kinds: Vec<ScopeKind>,
}

/// The result of lookup in a [`ScopeBuilder`].
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ScopeLookup {
    /// Which [`ScopeBinding`] does the lookup resolve to?
    pub binding: ScopeBinding,
    /// Number of scopes of kind [`ScopeKind::Body`] from the most recent scope
    /// where [`ScopeBinding`] was found.
    /// Is 0 for arm/local lookup within an enclosing body, and positive for a
    /// genuine enclosing closure.
    pub body_distance: usize,
}

impl ScopeBuilder {
    /// Construct new [`ScopeBuilder`].
    #[must_use]
    pub fn new(registry: &PrimitiveRegistry) -> Self {
        // NOTE: The first scope is pre-initialised with primitives.
        let mut first = Scope::new();
        let names = &mut first.0;
        for (id, sym) in registry.iter_syms() {
            names.insert(sym, ScopeBinding::Primitive(id));
        }

        Self {
            scopes: vec![first],
            kinds: vec![ScopeKind::Primordial],
        }
    }

    /// Bind onto the most recent [`Scope`] in the [`ScopeBuilder`].
    ///
    /// # Panics
    /// - If there are no non-primordial scopes present.
    pub fn bind(&mut self, sym: SymId, binding: BindingId) {
        assert!(
            self.scopes.len() > 1,
            "Expected at least one actual scope on the stack"
        );
        let scope = self.scopes.last_mut().expect("Checked via assert!");
        let binding = ScopeBinding::Binding(binding);
        scope.0.insert(sym, binding);
    }

    /// Construct a new [`Scope`] of kind [`ScopeKind::Body`] in the
    /// [`ScopeBuilder`].
    pub fn push_body(&mut self) {
        self.scopes.push(Scope::new());
        self.kinds.push(ScopeKind::Body);
    }

    /// Construct a new [`Scope`] of kind [`ScopeKind::Arm`] in the
    /// [`ScopeBuilder`].
    pub fn push_arm(&mut self) {
        self.scopes.push(Scope::new());
        self.kinds.push(ScopeKind::Arm);
    }

    /// Pop a [`Scope`] of kind [`ScopeKind::Body`] off the stack.
    ///
    /// # Panics
    /// - If the most recent scope on the stack is not of kind
    ///   [`ScopeKind::Body`].
    pub fn pop_body(&mut self) {
        assert!(
            self.kinds.last().is_some_and(|&x| x == ScopeKind::Body),
            "pop_body expected ScopeKind::Body as last scope"
        );
        let _ = self.scopes.pop();
        let _ = self.kinds.pop();
    }

    /// Pop a [`Scope`] of kind [`ScopeKind::Arm`] off the stack.
    ///
    /// # Panics
    /// - If the most recent scope on the stack is not of kind
    ///   [`ScopeKind::Arm`].
    pub fn pop_arm(&mut self) {
        assert!(
            self.kinds.last().is_some_and(|&x| x == ScopeKind::Arm),
            "pop_arm expected ScopeKind::Arm as last scope"
        );
        let _ = self.scopes.pop();
        let _ = self.kinds.pop();
    }

    /// Attempt to resolve a name from the most recent scope of this
    /// [`ScopeBuilder`], returning a [`ScopeLookup`] if successful.  Returns
    /// `None` otherwise.
    pub fn lookup(&self, sym: SymId) -> Option<ScopeLookup> {
        let mut body_distance = 0;
        for (scope, &kind) in
            self.scopes.iter().rev().zip(self.kinds.iter().rev())
        {
            if let Some(&binding) = scope.0.get(&sym) {
                return Some(ScopeLookup {
                    binding,
                    body_distance,
                });
            }
            if kind == ScopeKind::Body {
                body_distance += 1;
            }
        }
        None
    }
}

impl Scope {
    /// Construct new scope.
    fn new() -> Self {
        Self(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        interner::Interner,
        resolution::{BindingTable, BodyLayout},
        source::{SourceTable, Span},
    };

    fn binding(table: &mut BindingTable, layout: &mut BodyLayout) -> BindingId {
        let mut sources = SourceTable::new();
        let source = sources
            .add_source_raw("test", "x".into())
            .expect("test source should be valid");
        let origin = sources.add_origin(source, Span::new(0, 1));
        table.add(origin, layout.add_local())
    }

    #[test]
    fn lexical_scopes_shadow_and_restore_bindings() {
        let mut interner = Interner::new();
        let x = interner.intern("x");
        let y = interner.intern("y");
        let registry = PrimitiveRegistry::new();
        let mut scopes = ScopeBuilder::new(&registry);
        let mut bindings = BindingTable::new();
        let mut layout = BodyLayout::new();
        let outer = binding(&mut bindings, &mut layout);
        let later = binding(&mut bindings, &mut layout);
        let inner = binding(&mut bindings, &mut layout);

        scopes.push_body();
        scopes.bind(x, outer);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(outer),
                body_distance: 0
            })
        );
        scopes.bind(x, later);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                body_distance: 0,
            })
        );

        scopes.push_body();
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                body_distance: 1,
            })
        );
        scopes.bind(x, inner);
        scopes.bind(y, inner);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(inner),
                body_distance: 0,
            })
        );

        scopes.pop_body();
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                body_distance: 0,
            })
        );
        assert_eq!(scopes.lookup(y), None);
    }

    #[test]
    fn arm_scopes_preserve_body_distance() {
        let mut interner = Interner::new();
        let outer_name = interner.intern("outer");
        let arm_name = interner.intern("arm");
        let registry = PrimitiveRegistry::new();
        let mut scopes = ScopeBuilder::new(&registry);
        let mut bindings = BindingTable::new();
        let mut outer_layout = BodyLayout::new();
        let outer = binding(&mut bindings, &mut outer_layout);
        let arm = binding(&mut bindings, &mut outer_layout);

        scopes.push_body();
        scopes.bind(outer_name, outer);
        scopes.push_arm();
        scopes.bind(arm_name, arm);
        assert_eq!(
            scopes.lookup(outer_name),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(outer),
                body_distance: 0,
            })
        );

        scopes.push_body();
        assert_eq!(
            scopes.lookup(arm_name),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(arm),
                body_distance: 1,
            })
        );
        assert_eq!(
            scopes.lookup(outer_name),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(outer),
                body_distance: 1,
            })
        );

        scopes.pop_body();
        scopes.pop_arm();
        assert_eq!(scopes.lookup(arm_name), None);
    }

    #[test]
    fn bindings_shadow_primordial_primitives() {
        let mut interner = Interner::new();
        let sym = interner.intern("primitive");
        let mut registry = PrimitiveRegistry::new();
        let primitive = registry.add(sym);
        let mut scopes = ScopeBuilder::new(&registry);
        let mut bindings = BindingTable::new();
        let mut layout = BodyLayout::new();
        let binding = binding(&mut bindings, &mut layout);

        assert_eq!(
            scopes.lookup(sym),
            Some(ScopeLookup {
                binding: ScopeBinding::Primitive(primitive),
                body_distance: 0,
            })
        );

        scopes.push_body();
        scopes.bind(sym, binding);
        assert_eq!(
            scopes.lookup(sym),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(binding),
                body_distance: 0,
            })
        );

        scopes.pop_body();
        assert_eq!(
            scopes.lookup(sym),
            Some(ScopeLookup {
                binding: ScopeBinding::Primitive(primitive),
                body_distance: 0,
            })
        );
    }

    #[test]
    #[should_panic(expected = "expected ScopeKind::Body")]
    fn primordial_scope_cannot_be_popped() {
        let registry = PrimitiveRegistry::new();
        ScopeBuilder::new(&registry).pop_body();
    }
}
