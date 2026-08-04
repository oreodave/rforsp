use std::collections::HashMap;

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
/// ID representing an interned symbol - only returnable by the Interner.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SymId(u32);

/// Generic interner structure that maintains a unique collection of Symbols
/// with associated [`SymId`]s.
#[derive(Debug)]
pub struct Interner {
    names: Vec<String>,
    lookup: HashMap<String, SymId>,
}

/******************************************************************************
 * Distinguished symbols                                                      *
 ******************************************************************************/
const DISTINGUISHED: [&str; 3] = ["f", "if", "rec"];

/// Symbol representing "f"
pub const SYM_F: SymId = SymId(0);
/// Symbol representing "if"
pub const SYM_IF: SymId = SymId(1);
/// Symbol representing "rec"
pub const SYM_REC: SymId = SymId(2);

// Build time assertion that we've built around 3 distinguished symbols.
const _: () = assert!(DISTINGUISHED.len() == 3);

/******************************************************************************
 * Implementation                                                             *
 ******************************************************************************/
impl Interner {
    /// Creates a new [Interner] structure.
    /// The interner automatically interns a number of distinguished symbols
    /// which see.
    #[must_use]
    pub fn new() -> Self {
        let mut interner = Self {
            names: Vec::new(),
            lookup: HashMap::new(),
        };

        for name in DISTINGUISHED {
            interner.intern(name);
        }

        interner
    }

    /// Intern a `name`, returning its associated [`SymId`].
    /// NOTE: This will mutate and allocate iff the `name` is not already
    /// present in `self`.
    ///
    /// # Panics
    /// - if `self.names.len()` > [`u32::MAX`] (over 4 billion names...)
    pub fn intern(&mut self, name: &str) -> SymId {
        if let Some(&id) = self.lookup.get(name) {
            id
        } else {
            let id = SymId(
                u32::try_from(self.names.len()).expect("|names| > u32::MAX"),
            );
            let name: String = name.into();
            self.names.push(name.clone());
            self.lookup.insert(name, id);
            id
        }
    }

    /// Get the associated [`SymId`] for a given `name`.
    /// Returns None iff `name` is not present in `self`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<SymId> {
        self.lookup.get(name).copied()
    }

    /// Resolve the given `id` to the string contents.
    ///
    /// # Panics
    /// - if `id` isn't a valid [`SymId`] in `self`.
    #[must_use]
    pub fn resolve(&self, id: SymId) -> &str {
        &self.names[id.0 as usize]
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

/******************************************************************************
 * Tests                                                                      *
 ******************************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction() {
        let interner = Interner::new();
        assert_eq!(interner.get("f"), Some(SYM_F));
        assert_eq!(interner.get("if"), Some(SYM_IF));
        assert_eq!(interner.get("rec"), Some(SYM_REC));
    }

    #[test]
    fn no_collision() {
        let mut interner = Interner::new();
        let a = interner.intern("my-house");
        let b = interner.intern("your-house");
        assert_ne!(a, b);
        assert_eq!(interner.resolve(a), "my-house");
        assert_eq!(interner.resolve(b), "your-house");
    }

    #[test]
    fn interning_idempotent() {
        let mut interner = Interner::new();
        let a = interner.intern("xyz123");
        let b = interner.intern("xyz123");
        assert_eq!(a, b);
        assert_eq!(interner.resolve(b), "xyz123");
        assert_eq!(interner.names.len(), DISTINGUISHED.len() + 1);
    }

    #[test]
    fn get() {
        let mut interner = Interner::new();
        let name = "can't-park-there-mate";
        assert_eq!(interner.get(name), None);
        let id = interner.intern(name);
        assert_eq!(interner.get(name), Some(id));
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn resolve_bad() {
        // Resolve will panic for IDs that are out of range.
        let _ = Interner::new().resolve(SymId(1024));
    }

    #[test]
    fn resolve() {
        const NAMES: [&str; 4] =
            ["hello", "derivative", "->>", "xy871238huashask_;@"];

        // NOTE: Since resolve can panic, we can only do success validation
        // tests here.
        let mut interner = Interner::new();
        let ids = NAMES.map(|n| interner.intern(n));

        for (&id, &name) in ids.iter().zip(NAMES.iter()) {
            assert_eq!(interner.resolve(id), name);
        }
    }
}
