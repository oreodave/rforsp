//! Unique IDs for every binding.

use crate::{resolution::LocalId, source::SyntaxId, u32_index};

/// Fresh minted ID for a binding.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BindingId(u32);

/// Information attached to a binding.
#[derive(Debug, Copy, Clone)]
pub struct BindingInfo {
    /// Where did this originate from in the source code?
    pub origin: SyntaxId,
    /// Within the enclosing body, which local is it stored in?
    pub local: LocalId,
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

    /// Get the [`BindingInfo`]s currently available.
    #[must_use]
    pub fn bindings(&self) -> &[BindingInfo] {
        &self.table
    }

    /// Get the [`BindingInfo`] for an ID.
    ///
    /// # Panics
    /// - If [`BindingId`] is invalid for this table.
    #[track_caller]
    #[must_use]
    pub fn get(&self, id: BindingId) -> BindingInfo {
        *self
            .table
            .get(id.0 as usize)
            .expect("Invalid BindingId for this binding table")
    }
}

impl Default for BindingTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        resolution::BodyLayout,
        source::{SourceTable, Span},
    };

    fn origin(table: &mut SourceTable) -> SyntaxId {
        let source = table
            .add_source_raw("test", "x".into())
            .expect("test source should be valid");
        table.add_origin(source, Span::new(0, 1))
    }

    #[test]
    fn additions_preserve_their_metadata() {
        let mut sources = SourceTable::new();
        let first_origin = origin(&mut sources);
        let second_origin = origin(&mut sources);
        let mut layout = BodyLayout::new();
        let first_local = layout.add_local();
        let second_local = layout.add_local();
        let mut bindings = BindingTable::new();

        let first = bindings.add(first_origin, first_local);
        let second = bindings.add(second_origin, second_local);

        assert_ne!(first, second);
        assert_eq!(bindings.get(first).origin, first_origin);
        assert_eq!(bindings.get(first).local, first_local);
        assert_eq!(bindings.get(second).origin, second_origin);
        assert_eq!(bindings.get(second).local, second_local);
        assert_eq!(bindings.bindings().len(), 2);
    }
}
