use std::collections::HashMap;

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
/// ID representing an interned symbol - only returnable by the Interner.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct SymId(u32);

/// Generic interner structure that maintains a unique collection of Symbols
/// with associated SymIds.
#[derive(Debug)]
pub struct Interner {
    names: Vec<String>,
    lookup: HashMap<String, SymId>,
}

/******************************************************************************
 * Distinguished symbols                                                      *
 ******************************************************************************/
const DISTINGUISHED: [&str; 3] = ["f", "if", "rec"];
pub const SYM_F: SymId = SymId(0);
pub const SYM_IF: SymId = SymId(1);
pub const SYM_REC: SymId = SymId(2);

// Build time assertion that we've built around 3 distinguished symbols.
const _: () = assert!(DISTINGUISHED.len() == 3);

/******************************************************************************
 * Implementation                                                             *
 ******************************************************************************/
impl Interner {
    /// Creates a new [`Interner`] structure.
    /// The interner automatically interns a number of distinguished symbols
    /// which see.
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

    /// Intern a `name`, returning its associated `SymId`.
    /// NOTE: This will mutate and allocate iff the `name` is not already
    /// present in `self`.
    pub fn intern(&mut self, name: &str) -> SymId {
        if let Some(&id) = self.lookup.get(name) {
            id
        } else {
            let id = SymId(self.names.len() as u32);
            let name: String = name.into();
            self.names.push(name.clone());
            self.lookup.insert(name, id);
            id
        }
    }

    /// Get the associated `SymId` for a given `name`.
    /// Returns None iff `name` is not present in `self`.
    pub fn get(&self, name: &str) -> Option<SymId> {
        self.lookup.get(name).copied()
    }

    /// Resolve the given `id` to the string contents.
    /// NOTE: This will panic iff `id` isn't a valid `SymID` in `self`.
    pub fn resolve(&self, id: SymId) -> &str {
        &self.names[id.0 as usize]
    }
}
