//! Binding resolution.
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and
//! produces a side table for later phases.

mod bindings;
pub use bindings::{BindingId, BindingInfo, BindingTable};

mod closure;
pub use closure::{CaptureId, CaptureSource, ClosureLayout, LocalId};

mod types;
pub use types::{ArmRole, Resolution, Target};
