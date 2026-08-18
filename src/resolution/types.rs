//! Types for the Resolution pass

use std::collections::BTreeMap;

use crate::{
    resolution::{BindingId, CaptureId, ClosureLayout},
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
    /// Form makes a closure with the given [`CaptureLayout`].
    MakesClosure(CaptureLayout),
    /// Form makes a recursive closure with the given [`CaptureLayout`].
    MakesRecursiveClosure(CaptureLayout),
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
pub struct ResolutionMap {
    /// Map between [`SyntaxId`]s and their resultant [`Resolution`].
    mapping: BTreeMap<SyntaxId, Resolution>,
}
