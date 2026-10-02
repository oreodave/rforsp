//! Primitive type and registry.

use crate::interner::{Interner, SymId};

/// ID for an entry in the primitive registry.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct PrimitiveId(usize);

/// Type for primitive
pub struct Primitive {
    /// Symbol attached to this primitive.
    sym: SymId,
    // TODO(oreo)[2026-08-17 08:26]: fields for stack signature, heap effect,
    // runtime impl, and lowering.
}

/// Registry of primitives, filled in during session initialisation.
pub struct PrimitiveRegistry {
    /// Raw vector of primitives from which [`PrimitiveId`] is derived.
    primitives: Vec<Primitive>,
}

/// Names of default primitives within the rForsp language.
/// NOTE: ordering is important for the [`PrimitiveRegistry::with_builtins`]
/// method.
pub(super) const PRIMITIVE_NAMES: [&str; 27] = [
    "cons", "car", "cdr", "eq", "cswap", "tag", "read", "write", "exit", "+",
    "-", "*", "/", "&", "|", "~&", "<<", ">>", "rec", "if", "copy", "length",
    "vmake", "vpush", "vpop", "vget", "vset",
];

impl PrimitiveRegistry {
    /// Construct a new [`PrimitiveRegistry`].
    #[must_use]
    const fn new() -> Self {
        Self {
            primitives: Vec::new(),
        }
    }

    /// Construct a new [`PrimitiveRegistry`] pre-initialised with expected
    /// primitives.
    pub fn with_builtins(interner: &mut Interner) -> Self {
        let mut registry = Self::new();
        for name in PRIMITIVE_NAMES {
            let sym = interner.intern(name);
            let _ = registry.add(sym);
        }
        registry
    }

    /// Add a primitive to the registry, returning a [`PrimitiveId`] for the new
    /// addition.
    #[must_use]
    fn add(&mut self, sym: SymId) -> PrimitiveId {
        // TODO(oreo)[2026-08-17 08:31]: propagate fields required for
        // `Primitive` to function signature.
        let id = PrimitiveId(self.primitives.len());
        self.primitives.push(Primitive { sym });
        id
    }

    /// Return an iterator over the [`SymId`] of all primitives within the
    /// registry.
    pub fn iter_syms(
        &self,
    ) -> impl ExactSizeIterator<Item = (PrimitiveId, SymId)> {
        (0..self.primitives.len())
            .map(|i| (PrimitiveId(i), self.primitives[i].sym))
    }
}
