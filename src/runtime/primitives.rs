//! Primitive type and registry.

use crate::interner::SymId;

/// ID for an entry in the primitive registry.
pub struct PrimitiveId(u32);

/// Type for primitive
pub struct Primitive {
    /// Symbol attached to this primitive.
    sym: SymId,
    // FIXME(oreo)[2026-08-17 08:26]:: fields for stack signature, heap effect,
    // runtime impl, and lowering.
}

/// Registry of primitives, filled in during session initialisation.
pub struct PrimitiveRegistry {
    /// Raw vector of primitives from which [`PrimitiveId`] is derived.
    primitives: Vec<Primitive>,
}
