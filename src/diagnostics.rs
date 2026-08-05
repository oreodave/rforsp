//! Unified diagnostic representation.
//!
//! TODO: Finish this.

mod phase;
pub use phase::{Aborted, Phase};

mod class;
pub use class::{Class, Severity};

mod diagnostic;
pub use diagnostic::{Diagnostic, Site};

mod accumulator;
pub use accumulator::{DEFAULT_DIAGNOSTIC_CAP, Diagnostics};

pub mod conv;
