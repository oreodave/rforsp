//! Binding resolution.
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and
//! produces a side table for later phases.

mod bindings;
pub use bindings::{BindingId, BindingInfo, BindingTable};

mod body;
pub use body::{BodyLayout, CaptureId, CaptureSource, LocalId};

mod builder;

mod types;
pub use types::{ArmRole, Resolution, ResolutionMap, ResolutionResult, Target};

mod error;

mod resolver;
pub use resolver::resolve;
