//! Module - a sequence of bodies and datums, with the first body being the
//! entry point.  The final output of the lowering phase.
//!
//! [`Datum`]s are owned data translated from [`crate::parser::HirForm`]s in
//! datum position during lowering.

use crate::{interner::SymId, lowering::Body, u32_index};

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

/// ID for a body within a module.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BodyId(u32);

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

impl Module {
    /// Construct a new [`Module`].
    fn new() -> Self {
        Self::default()
    }

    /// Add a new [`Datum`], returning a [`DatumId`].
    #[must_use]
    pub fn add_datum(&mut self, datum: Datum) -> DatumId {
        let id = DatumId(u32_index(self.data.len()));
        self.data.push(datum);
        id
    }

    /// Retrieve a [`Datum`] by [`DatumId`].
    ///
    /// # Panics
    /// - If given `id` is not valid.
    #[must_use]
    pub fn get_datum(&self, id: DatumId) -> &Datum {
        assert!(
            (id.0 as usize) < self.data.len(),
            "{id:?} is not valid for this module."
        );
        &self.data[id.0 as usize]
    }

    /// Add a new default [`Body`], returning a [`BodyId`].
    #[must_use]
    pub fn add_body(&mut self) -> BodyId {
        let id = BodyId(u32_index(self.bodies.len()));
        self.bodies.push(Body::default());
        id
    }

    /// Retrieve a [`Body`] by [`BodyId`].
    ///
    /// # Panics
    /// - If given `id` is not valid.
    #[must_use]
    pub fn get_body(&self, id: BodyId) -> &Body {
        assert!(
            (id.0 as usize) < self.bodies.len(),
            "{id:?} is not valid for this module."
        );
        &self.bodies[id.0 as usize]
    }

    /// Retrieve a [`Body`] mutably by [`BodyId`].
    ///
    /// # Panics
    /// - If given `id` is not valid.
    #[must_use]
    pub fn get_body_mut(&mut self, id: BodyId) -> &mut Body {
        assert!(
            (id.0 as usize) < self.bodies.len(),
            "{id:?} is not valid for this module."
        );
        &mut self.bodies[id.0 as usize]
    }
}

impl Default for Module {
    fn default() -> Self {
        Self {
            bodies: vec![Body::default()],
            data: Vec::new(),
        }
    }
}
