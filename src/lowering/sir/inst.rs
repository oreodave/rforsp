//! Instructions - operative vocabulary for execution in SIR.  The smallest unit
//! representing executable code.

use crate::{
    lowering::sir::{BodyId, CaptureId, DatumId, LocalId},
    runtime::{PrimitiveId, RuntimeVariableId},
};

/// Instructions within SIR.
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

/// Slot or primordial reference used by a load, call, or bind.
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

/// Calling mode attached to a constructed closure.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ClosureMode {
    /// Ordinary closure entry.
    Normal,
    /// Recursive closure entry.
    Recursive,
}
