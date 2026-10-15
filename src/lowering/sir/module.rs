//! Module - a sequence of bodies and datums, with the first body being the
//! entry point.  The final output of the lowering phase.
//!
//! [`Datum`]s are owned data translated from [`crate::parser::HirForm`]s in
//! datum position during lowering.

use crate::{define_id, helper::Table, interner::SymId, lowering::sir::Body};

define_id!(pub BodyId);
define_id!(pub DatumId);

/// Complete output of lowering.
///
/// NOTE: [`BodyId`] 0 is the entry body of the module.
#[derive(Debug)]
pub struct Module {
    /// Bodies indexed by [`BodyId`].
    pub bodies: Table<BodyId, Body>,
    /// Owned datum nodes indexed by [`DatumId`].
    pub data: Table<DatumId, Datum>,
}

impl Module {
    /// Construct a new [`Module`].
    ///
    /// NOTE: This default initialises a [`Body`] which is considered the
    /// "entry" [`Body`] for this [`Module`].
    pub(crate) fn new() -> Self {
        let mut bodies = Table::new();
        let _ = bodies.push(Body::new(None));
        Self {
            bodies,
            data: Table::new(),
        }
    }

    /// Get the "entry" [`Body`] for this [`Module`].
    #[must_use]
    pub fn get_entry(&self) -> &Body {
        self.bodies.get(BodyId::default())
    }

    /// Get the "entry" [`Body`] for this [`Module`] mutably.
    #[must_use]
    pub fn get_entry_mut(&mut self) -> &mut Body {
        self.bodies.get_mut(BodyId::default())
    }
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
