//! Module - a sequence of bodies and datums, with the first body being the
//! entry point.  The final output of the lowering phase.
//!
//! [`Datum`]s are owned data translated from [`crate::parser::HirForm`]s in
//! datum position during lowering.

use crate::{interner::SymId, lowering::Body};

/// Complete output of lowering.
///
/// NOTE: [`BodyId`][crate::lowering::BodyId] 0 is the entry body of the module.
#[derive(Debug)]
pub struct Module {
    /// Bodies indexed by [`BodyId`][crate::lowering::BodyId].
    pub bodies: Vec<Body>,
    /// Owned datum nodes indexed by [`DatumId`].
    pub data: Vec<Datum>,
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
