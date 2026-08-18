//! Binding resolution.
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and
//! produces a side table for later phases.

mod capture;
pub use capture::{CaptureId, CaptureLayout, CaptureSource, LocalId};
mod bindings;
pub use bindings::{BindingId, BindingInfo, BindingTable};

mod types;
pub use types::{ArmRole, Resolution, Target};
