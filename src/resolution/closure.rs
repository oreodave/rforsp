//! Closure output results from Resolution.

use crate::u32_index;

/// ID for a local within a closure frame.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct LocalId(u32);

/// ID for a capture within a closure frame.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CaptureId(u32);

/// Source for a value captured by a closure.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CaptureSource {
    /// Capture a local within the enclosing frame.
    Local(LocalId),
    /// Capture another capture within the enclosing frame.
    Captured(CaptureId),
}

/// Layout of a closure after analysis.
pub struct ClosureLayout {
    /// Captures for this closure, indexed by [`CaptureId`].
    captures: Vec<CaptureSource>,
    /// Number of locals for this closure.
    local_count: usize,
}
