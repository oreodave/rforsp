//! Unique IDs for every binding.

use crate::{resolution::LocalId, source::SyntaxId, u32_index};

/// Fresh minted ID for a binding.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BindingId(u32);

/// Information attached to a binding.
#[derive(Debug, Copy, Clone)]
pub struct BindingInfo {
    /// Where did this originate from in the source code?
    origin: SyntaxId,
    /// Within the enclosing body, which local is it stored in?
    local: LocalId,
}

/// Table that mints new Bindings.
pub struct BindingTable {
    /// Raw vector for minting purposes.
    table: Vec<BindingInfo>,
}

impl BindingTable {
    /// Construct new binding table.
    #[must_use]
    pub const fn new() -> Self {
        Self { table: Vec::new() }
    }

    /// Mint new [`BindingId`].
    ///
    /// # Panics
    /// - If table entries exceed `u32::MAX`.
    #[must_use]
    pub fn add(&mut self, origin: SyntaxId, local_id: LocalId) -> BindingId {
        let id = BindingId(u32_index(self.table.len()));
        self.table.push(BindingInfo {
            origin,
            local: local_id,
        });
        id
    }
}

impl Default for BindingTable {
    fn default() -> Self {
        Self::new()
    }
}
