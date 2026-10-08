//! Lowering from resolved HIR to executable bodies and control flow.

mod block;
pub use block::{
    Block, ClosureMode, Instruction, Target, Terminator, TerminatorKind,
};

mod module;
pub use module::{BlockId, Body, BodyId, Datum, DatumId, Module};

mod lower;
pub use lower::lower;
