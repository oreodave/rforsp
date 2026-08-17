//! Binding resolution.
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and
//! produces a side table for later phases.

mod closure;
pub use closure::{CaptureId, CaptureLayout, CaptureSource, LocalId};

mod types;
pub use types::{ArmRole, BindingId, Resolution, Target};
