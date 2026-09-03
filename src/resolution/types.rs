//! Types for the Resolution pass

use std::collections::BTreeMap;

use crate::{
    resolution::{BindingId, BindingTable, BodyLayout, CaptureId},
    runtime::PrimitiveId,
    source::SyntaxId,
};

/// Target for a reference (call or load).
pub enum Target {
    /// Local binding.
    Local(BindingId),
    /// Captured binding.
    Captured(CaptureId),
    /// Primitive binding.
    Primitive(PrimitiveId),
}

/// Role of an arm in a recognised conditional.
pub enum ArmRole {
    /// True branch.
    Then,
    /// False branch.
    Else,
}

/// Resolution recorded for a HIR form.
pub enum Resolution {
    /// Form is a binding.
    Bound(BindingId),
    /// Form is a reference to some target.
    Ref(Target),
    /// Form makes a closure with the given [`BodyLayout`].
    MakesClosure(BodyLayout),
    /// Form makes a recursive closure with the given [`BodyLayout`].
    MakesRecursiveClosure(BodyLayout),
    /// Form is a branch in a conditional.
    BranchArm(ArmRole),
    /// Form is recognised as the conditional primitive `if`.
    Conditional,
    /// Form is recognised as the primitive `rec`.
    Recursive,
    /// Resolution failed for this form, but the walk continued.
    Poison,
}

/// Side table of [`Resolution`]s indexed by [`SyntaxId`]s.
#[derive(Default)]
pub struct ResolutionMap {
    /// Map between [`SyntaxId`]s and their resultant [`Resolution`].
    mapping: BTreeMap<SyntaxId, Resolution>,
}

/// Complete output of binding resolution.
pub struct ResolutionResult {
    /// Resolutions for forms within the entry body.
    pub map: ResolutionMap,
    /// Metadata for every binding minted during resolution.
    pub bindings: BindingTable,
    /// Frame layout for the entry body.
    pub entry: BodyLayout,
}

impl ResolutionMap {
    /// Get the resolution recorded for a form.
    #[must_use]
    pub fn get(&self, id: SyntaxId) -> Option<&Resolution> {
        self.mapping.get(&id)
    }

    /// Get the number of forms with a resolution entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    /// Whether the map contains no resolution entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Record a form's resolution.
    pub(super) fn insert(&mut self, id: SyntaxId, resolution: Resolution) {
        assert!(
            self.mapping.insert(id, resolution).is_none(),
            "a form must have at most one resolution"
        );
    }
}
