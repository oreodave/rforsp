//! Block - a sequence of [`Instruction`]s that exit using some [`Terminator`].

use crate::{lowering::Instruction, source::SyntaxId};

/// ID for a block within a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BlockId(u32);

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
