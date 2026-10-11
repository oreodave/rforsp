//! Module - a sequence of bodies and datums, with the first body being the
//! entry point.  The final output of the lowering phase.
//!
//! [`Datum`]s are owned data translated from [`crate::parser::HirForm`]s in
//! datum position during lowering.

use crate::{define_id, helper::Table, interner::SymId, lowering::Body};

define_id!(pub BodyId);
define_id!(pub DatumId);

/// Complete output of lowering.
///
/// NOTE: [`BodyId`] 0 is the entry body of the module.
#[derive(Debug, Default)]
pub struct Module {
    /// Bodies indexed by [`BodyId`].
    pub bodies: Table<BodyId, Body>,
    /// Owned datum nodes indexed by [`DatumId`].
    pub data: Table<DatumId, Datum>,
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
