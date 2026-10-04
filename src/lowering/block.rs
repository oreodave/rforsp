//! Blocks and instructions - executable SIR code within a body.
//!
//! These are the components that make up the vast majority of translation from
//! our higher level representation.  [`crate::parser::HirForm`]s, with added
//! context in the form of a [`crate::resolution::ResolutionMap`], are
//! translated into sequences of [`Instruction`]s arranged in [`Block`]s.

use crate::{
    lowering::{BlockId, BodyId, DatumId},
    resolution::{CaptureId, LocalId},
    runtime::{PrimitiveId, RuntimeVariableId},
    source::SyntaxId,
};

/// Straight-line operations followed by explicit control flow.
#[derive(Debug)]
pub struct Block {
    /// Operations in execution order.
    pub instructions: Vec<Instruction>,
    /// Source origins of instructions, indexed in parallel.
    pub origins: Vec<SyntaxId>,
    /// Control flow after the final instruction.
    pub terminator: Terminator,
}

impl Block {
    /// Construct a new [`Block`] with the given [`Terminator`].
    #[must_use]
    pub(super) const fn new(terminator: Terminator) -> Self {
        Self {
            instructions: Vec::new(),
            origins: Vec::new(),
            terminator,
        }
    }

    /// Add a new [`Instruction`] at the given `origin` to this [`Block`].
    pub(super) fn add_inst(&mut self, inst: Instruction, origin: SyntaxId) {
        debug_assert_eq!(
            self.instructions.len(),
            self.origins.len(),
            "Expected instruction length to match origin length"
        );

        self.instructions.push(inst);
        self.origins.push(origin);
    }
}

/// Straight-line operation vocabulary.
#[derive(Debug)]
pub enum Instruction {
    /// Push a module-owned datum.
    PushData(DatumId),
    /// Push the value of a reference without dispatching it.
    Load(Target),
    /// Read a reference and apply normal call dispatch.
    Call(Target),
    /// Pop a value into a local slot.
    Bind(LocalId),
    /// Construct a closure using the referenced body's capture layout.
    MakeClosure(BodyId, ClosureMode),
}

/// Slot or primordial reference used by a load or call.
#[derive(Debug, Copy, Clone)]
pub enum Target {
    /// Local slot in the current body.
    Local(LocalId),
    /// Capture slot in the current body.
    Captured(CaptureId),
    /// Registered primitive.
    Primitive(PrimitiveId),
    /// Registered runtime variable.
    Variable(RuntimeVariableId),
}

/// Block termination with an optional source origin.
#[derive(Debug)]
pub struct Terminator {
    /// Source form, or `None` for generated control flow.
    pub origin: Option<SyntaxId>,
    /// Control-flow operation.
    pub kind: TerminatorKind,
}

/// Explicit control flow between blocks or out of a body.
#[derive(Debug)]
pub enum TerminatorKind {
    /// Complete execution of the current body.
    End,
    /// Continue at another block in the current body.
    Jump(BlockId),
    /// Pop a condition and select a block in the current body.
    Branch {
        /// Destination for a true condition.
        then_block: BlockId,
        /// Destination for a false condition.
        else_block: BlockId,
    },
}

/// Calling mode attached to a constructed closure.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ClosureMode {
    /// Ordinary closure entry.
    Normal,
    /// Recursive closure entry.
    Recursive,
}
