//! Lowering
//!
//! This phase walks parsed [`HirForm`][crate::parser::HirForm]s and produces an
//! equivalent [`Module`] in SIR - the Stack-based Intermediate Representation.

mod inst;
pub use inst::{ClosureMode, Instruction, Target};

mod block;
pub use block::{Block, BlockId, Terminator, TerminatorKind};

mod body;
pub use body::{Body, CaptureId, CaptureSource, LocalId};

mod module;
pub use module::{BodyId, Datum, DatumId, Module};

mod lower;
pub use lower::lower;
