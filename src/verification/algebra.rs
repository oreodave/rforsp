//! The stack/type algebra of rForsp's type system.
//!
//! This algebra is the instrument that permits verification to construct and
//! solve constraint equations on instructions, blocks, and bodies.

use crate::define_id;

define_id!(pub TypeVar);
define_id!(pub RowVar);

/// Encapsulation of types of values.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueType {
    /// Type that is to-be-resolved.
    Var(TypeVar),
    /// Integers.
    Integer,
    /// Lists.
    List,
    /// Vectors (datum position).
    Vector,
    /// Some closure with an attached effect when called.
    Closure(Box<Effect>),
}

/// Representation of a stack argument ordered from bottom to top.
#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    /// Possible stack prefix; None represents an exact valued row.
    pub prefix: Option<RowVar>,
    /// Established suffix types, ordered from bottom to top.
    pub values: Vec<ValueType>,
}

/// Stack effect that occurs when executing code.
#[derive(Debug, PartialEq, Eq)]
pub struct Effect {
    /// Required input stack.
    pub input: Row,
    /// Expected output stack.
    pub output: Row,
}

impl Row {
    /// A stack with no unknown prefix.
    #[must_use]
    pub const fn closed(values: Vec<ValueType>) -> Self {
        Self {
            prefix: None,
            values,
        }
    }

    /// A stack with an unknown prefix.
    #[must_use]
    pub const fn open(prefix: RowVar, values: Vec<ValueType>) -> Self {
        Self {
            prefix: Some(prefix),
            values,
        }
    }
}
