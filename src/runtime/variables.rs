//! Runtime variable types and registry.

use crate::interner::{Interner, SymId};

/// ID for an entry in the runtime variable registry.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct RuntimeVariableId(usize);

/// Type for runtime variable
pub struct RuntimeVariable {
    /// Symbol attached to this runtime variable.
    sym: SymId,
    // TODO(oreo)[2026-09-30 04:51]: fields for typing.
}

/// Registry of runtime variables, filled during session initialisation.
pub struct RuntimeVariableRegistry {
    /// Raw vector of variables - indexing derives [`RuntimeVariableId`].
    variables: Vec<RuntimeVariable>,
}

/// Names of default runtime variables within the rForsp language.
pub(super) const VARIABLE_NAMES: [&str; 3] =
    ["*stdout*", "*stdin*", "*stderr*"];

impl RuntimeVariableRegistry {
    /// Construct an empty [`RuntimeVariableRegistry`].
    #[must_use]
    const fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    /// Construct a new [`RuntimeVariableRegistry`] pre-initialised with
    /// expected variables.
    pub fn with_builtins(interner: &mut Interner) -> Self {
        let mut registry = Self::new();
        for name in VARIABLE_NAMES {
            let sym = interner.intern(name);
            let _ = registry.add(sym);
        }
        registry
    }

    /// Add a variable to the registry, returning a [`RuntimeVariableId`] for
    /// the new addition.
    #[must_use]
    fn add(&mut self, sym: SymId) -> RuntimeVariableId {
        // TODO(oreo)[2026-08-17 08:31]: propagate fields required for
        // `RuntimeVariable` to function signature.
        let id = RuntimeVariableId(self.variables.len());
        self.variables.push(RuntimeVariable { sym });
        id
    }

    /// Return an iterator over the [`SymId`] of all variables within the
    /// registry.
    pub fn iter_syms(
        &self,
    ) -> impl ExactSizeIterator<Item = (RuntimeVariableId, SymId)> {
        (0..self.variables.len())
            .map(|i| (RuntimeVariableId(i), self.variables[i].sym))
    }
}
