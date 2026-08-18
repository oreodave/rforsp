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

/// A scope is a map between [`SymId`] and [`ScopeBinding`].
struct Scope(HashMap<SymId, ScopeBinding>);

/// Generalised builder of scopes.
pub struct ScopeBuilder {
    /// Scopes being built.
    scopes: Vec<Scope>,
}

/// The result of lookup in a [`ScopeBuilder`].
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ScopeLookup {
    /// Which [`ScopeBinding`] does the lookup resolve to?
    pub binding: ScopeBinding,
    /// How many scopes outwards from the most recent scope does the
    /// corresponding [`ScopeBinding`] reside?  Is 0 if and only if lookup
    /// resolved within the most recent scope.
    pub lexical_distance: usize,
}

impl ScopeBuilder {
    /// Construct new [`ScopeBuilder`].
    #[must_use]
    pub fn new(registry: &PrimitiveRegistry) -> Self {
        // NOTE: The first scope is pre-initialised with primitives.
        let mut first = Scope::new();
        for (id, sym) in registry.iter_syms() {
            let lookup = &mut first.0;
            lookup.insert(sym, ScopeBinding::Primitive(id));
        }

        Self {
            scopes: vec![first],
        }
    }

    /// Construct a new [`Scope`] in the [`ScopeBuilder`].
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope(HashMap::new()));
    }

    /// Pop a [`Scope`] off the stack.
    ///
    /// # Panics
    /// - If there is only one scope on the stack.
    pub fn pop_scope(&mut self) {
        assert!(
            self.scopes.len() > 1,
            "First [`Scope`] in [`ScopeBuilder`] cannot be popped"
        );
        let _ = self.scopes.pop();
    }

    /// Bind onto the most recent [`Scope`] in the [`ScopeBuilder`].
    ///
    /// # Panics
    /// - If there are no scopes present.
    pub fn bind(&mut self, sym: SymId, binding: BindingId) {
        let scope = self
            .scopes
            .last_mut()
            .expect("Expect at least one [`Scope`] to be in ScopeBuilder");
        let binding = ScopeBinding::Binding(binding);
        scope.0.insert(sym, binding);
    }

    /// Attempt to resolve a name from the most recent scope of this
    /// [`ScopeBuilder`], returning a [`ScopeLookup`] if successful.  Returns
    /// `None` otherwise.
    pub fn lookup(&self, sym: SymId) -> Option<ScopeLookup> {
        for (lexical_distance, scope) in self.scopes.iter().rev().enumerate() {
            if let Some(&binding) = scope.0.get(&sym) {
                return Some(ScopeLookup {
                    binding,
                    lexical_distance,
                });
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

        scopes.push_scope();
        scopes.bind(x, outer);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(outer),
                lexical_distance: 0,
            })
        );
        scopes.bind(x, later);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                lexical_distance: 0,
            })
        );

        scopes.push_scope();
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                lexical_distance: 1,
            })
        );
        scopes.bind(x, inner);
        scopes.bind(y, inner);
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(inner),
                lexical_distance: 0,
            })
        );

        scopes.pop_scope();
        assert_eq!(
            scopes.lookup(x),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(later),
                lexical_distance: 0,
            })
        );
        assert_eq!(scopes.lookup(y), None);
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
                lexical_distance: 0,
            })
        );

        scopes.push_scope();
        scopes.bind(sym, binding);
        assert_eq!(
            scopes.lookup(sym),
            Some(ScopeLookup {
                binding: ScopeBinding::Binding(binding),
                lexical_distance: 0,
            })
        );

        scopes.pop_scope();
        assert_eq!(
            scopes.lookup(sym),
            Some(ScopeLookup {
                binding: ScopeBinding::Primitive(primitive),
                lexical_distance: 0,
            })
        );
    }

    #[test]
    #[should_panic(expected = "cannot be popped")]
    fn primordial_scope_cannot_be_popped() {
        let registry = PrimitiveRegistry::new();
        ScopeBuilder::new(&registry).pop_scope();
    }
}
