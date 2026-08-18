//! Unique IDs for every binding.

use crate::source::SyntaxId;

/// Fresh minted ID for a binding.
pub struct BindingId(usize);

/// Information attached to a binding.
pub struct BindingInfo {
    /// Where did this originate from in the source code?
    origin: SyntaxId,
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
    #[must_use]
    pub fn add(&mut self, origin: SyntaxId) -> BindingId {
        let id = BindingId(self.table.len());
        self.table.push(BindingInfo { origin });
        id
    }
}

impl Default for BindingTable {
    fn default() -> Self {
        Self::new()
    }
}
