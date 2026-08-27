//! Types for the Resolution pass

use crate::{
    resolution::{BindingId, BodyLayout, CaptureId},
    runtime::PrimitiveId,
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
