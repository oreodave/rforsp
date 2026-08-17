//! Closure capture types.

use crate::resolution::PrimitiveId;

/// ID for a local within a closure frame.
pub struct LocalId(u32);
/// ID for a capture within a closure frame.
pub struct CaptureId(u32);

/// Source for a value captured by a closure.
pub enum CaptureSource {
    /// Capture a local within the enclosing frame.
    Local(LocalId),
    /// Capture another capture within the enclosing frame.
    Captured(CaptureId),
    /// Capture a primordial primitive.
    Primitive(PrimitiveId),
}

/// Capture sources indexed by [`CaptureId`].
pub struct CaptureLayout {
    /// Source of each capture slot.
    sources: Vec<CaptureSource>,
}
