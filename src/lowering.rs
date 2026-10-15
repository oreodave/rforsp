//! Lowering
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and produces an
//! equivalent [`sir::Module`].

/// The Stack based Intermediate Representation data model.
pub mod sir {
    mod inst;
    pub use inst::{ClosureMode, Instruction, Target};

    mod block;
    pub use block::{Block, BlockId, Terminator, TerminatorKind};

    mod body;
    pub use body::{Body, CaptureId, CaptureSource, LocalId};

    mod module;
    pub use module::{BodyId, Datum, DatumId, Module};
}

mod env;

mod error;
pub use error::{LoweringError, LoweringErrorKind};

mod lower;
pub use lower::lower;
