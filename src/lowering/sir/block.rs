//! Block - a sequence of [`Instruction`]s that exit using some [`Terminator`].

use crate::{define_id, lowering::sir::Instruction, source::SyntaxId};

define_id!(pub BlockId);

/// Straight-line operations followed by explicit control flow.
#[derive(Debug, Default)]
pub struct Block {
    /// Operations in execution order.
    pub instructions: Vec<Instruction>,
    /// Source origins of instructions, indexed in parallel.
    pub origins: Vec<SyntaxId>,
    /// Control flow after the final instruction.
    pub terminator: Terminator,
}

impl Block {
    /// Push a new [`Instruction`] with given [`SyntaxId`] origin into this
    /// [`Block`].
    pub fn push_instruction(&mut self, inst: Instruction, origin: SyntaxId) {
        debug_assert_eq!(
            self.instructions.len(),
            self.origins.len(),
            "Expected number of origins to match number of instructions"
        );
        self.instructions.push(inst);
        self.origins.push(origin);
    }
}

/// Block termination with an optional source origin.
#[derive(Debug)]
pub struct Terminator {
    /// Source form, or `None` for generated control flow.
    pub origin: Option<SyntaxId>,
    /// Control-flow operation.
    pub kind: TerminatorKind,
}

impl Default for Terminator {
    fn default() -> Self {
        Self {
            origin: None,
            kind: TerminatorKind::End,
        }
    }
}

/// Explicit control flow between blocks or out of a body.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
