//! Modules, Data, and Bodies - the top level types of SIR.
//!
//! [`Datum`]s are owned data translated from [`crate::parser::HirForm`]s in
//! datum position during lowering.
//!
//! [`Body`]s represent the combined program entry and executable closure
//! bodies.  Certain recognised forms (like conditional arms) occupy [`Block`]s.
//!
//! A [`Module`] is the final output of lowering, composed of a sequence of
//! bodies and datums, with one body marked as the entry point.

use crate::{
    interner::SymId,
    lowering::{Block, Terminator, TerminatorKind},
    resolution::BodyLayout,
    source::SyntaxId,
    u32_index,
};

/// Complete output of lowering.
///
/// NOTE: [`BodyId`] 0 is the entry body of the module.
#[derive(Debug)]
pub struct Module {
    /// Bodies indexed by [`BodyId`].
    pub bodies: Vec<Body>,
    /// Owned datum nodes indexed by [`DatumId`].
    pub data: Vec<Datum>,
}

impl Module {
    /// Construct a new [`Module`] with the given entry [`BodyLayout`].
    #[must_use]
    pub(super) fn new(entry_layout: BodyLayout) -> Self {
        Self {
            bodies: vec![Body::entry_body(entry_layout)],
            data: Vec::new(),
        }
    }

    /// Add a [`Datum`] to this [`Module`].
    pub(super) fn add_datum(&mut self, datum: Datum) -> DatumId {
        let id = DatumId(u32_index(self.data.len()));
        self.data.push(datum);
        id
    }

    /// Add a [`Body`] to this [`Module`].
    pub(super) fn add_body(&mut self, body: Body) -> BodyId {
        let id = BodyId(u32_index(self.bodies.len()));
        self.bodies.push(body);
        id
    }
}

/// ID for a datum within a module.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct DatumId(u32);

/// Runtime data with indexed children rather than recursive ownership.
#[derive(Debug)]
pub enum Datum {
    /// Integer value.
    Int(i64),
    /// Symbol value.
    Atom(SymId),
    /// List elements in source order.
    List(Vec<DatumId>),
    /// Vector elements in source order.
    Vector(Vec<DatumId>),
    /// Quotation nested within datum content.
    Quote(DatumId),
}

/// ID for a body within a module.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BodyId(u32);

/// ID for a block within a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BlockId(u32);

/// Executable body with resolution's local and capture layout.
///
/// NOTE: [`BlockId`] 0 is the entry block for any body.
#[derive(Debug)]
pub struct Body {
    /// Vector origin, or `None` for the module entry body.
    pub origin: Option<SyntaxId>,
    /// Local slots and ordered capture sources from resolution.
    pub layout: BodyLayout,
    /// Blocks indexed by body-local [`BlockId`].
    pub blocks: Vec<Block>,
}

impl Body {
    /// Construct a [`Body`] for the program entry point.
    #[must_use]
    pub(super) fn entry_body(entry_layout: BodyLayout) -> Self {
        Self {
            origin: None,
            layout: entry_layout,
            blocks: vec![Block::new(Terminator {
                origin: None,
                kind: TerminatorKind::End,
            })],
        }
    }

    /// Construct a new [`Body`] with the given `origin` and [`BodyLayout`].
    #[must_use]
    pub(super) const fn new(origin: SyntaxId, layout: BodyLayout) -> Self {
        Self {
            origin: Some(origin),
            layout,
            blocks: vec![],
        }
    }

    /// Add a new [`Block`] to this [`Body`].
    pub(super) fn add_block(&mut self, block: Block) -> BlockId {
        let id = BlockId(u32_index(self.blocks.len()));
        self.blocks.push(block);
        id
    }

    /// Get the associated [`Block`] for `id` from this [`Body`].
    ///
    /// # Panics
    /// - If `id` is not valid for this [`Body`].
    #[must_use]
    pub(super) fn get_block(&self, id: BlockId) -> &Block {
        assert!(
            (id.0 as usize) < self.blocks.len(),
            "Expected valid BlockId, got {id:?}"
        );
        &self.blocks[id.0 as usize]
    }
}
