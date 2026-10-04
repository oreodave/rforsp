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
    interner::SymId, lowering::Block, resolution::BodyLayout, source::SyntaxId,
};

/// Complete output of lowering.
#[derive(Debug)]
pub struct Module {
    /// Entry body for the combined source program.
    pub entry: BodyId,
    /// Bodies indexed by [`BodyId`].
    pub bodies: Vec<Body>,
    /// Owned datum nodes indexed by [`DatumId`].
    pub data: Vec<Datum>,
}

/// ID for a body within a module.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BodyId(u32);

/// ID for a datum within a module.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct DatumId(u32);

/// ID for a block within a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BlockId(u32);

/// Executable body with resolution's local and capture layout.
#[derive(Debug)]
pub struct Body {
    /// Vector origin, or `None` for the module entry body.
    pub origin: Option<SyntaxId>,
    /// Local slots and ordered capture sources from resolution.
    pub layout: BodyLayout,
    /// First block executed on entry.
    pub entry: BlockId,
    /// Blocks indexed by body-local [`BlockId`].
    pub blocks: Vec<Block>,
}

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
