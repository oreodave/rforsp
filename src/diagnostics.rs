//! Unified diagnostic representation.
//!
//! Every stage of the compiler reports through the [`Diagnostic`] type.  Stages
//! do not render; they accumulate into a [`Diagnostics`] and rendering happens
//! once, at the driver, where the [`SourceTable`][crate::source::SourceTable]
//! is available to turn a [`Site`] into line/column text.

mod phase;
pub use phase::{Aborted, Phase};

mod class;
pub use class::{Class, Severity};

mod diagnostic;
pub use diagnostic::{Diagnostic, Site};

mod accumulator;
pub use accumulator::{DEFAULT_DIAGNOSTIC_CAP, Diagnostics};

pub mod conv;
