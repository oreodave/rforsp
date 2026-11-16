//! The stack/type algebra of rForsp's type system.
//!
//! This algebra is the instrument that permits verification to construct and
//! solve constraint equations on instructions, blocks, and bodies.

use crate::define_id;

define_id!(pub TypeVar);
define_id!(pub RowVar);

/// Encapsulation of types of values.
///
/// We refer to variants that are not [`ValueType::Var`] as "concrete".
#[derive(Debug, PartialEq, Eq, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Row {
    /// Possible stack prefix; None represents an exact valued row.
    pub prefix: Option<RowVar>,
    /// Established suffix types, ordered from bottom to top.
    pub suffix: Vec<ValueType>,
}

/// Stack effect that occurs when executing code.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Effect {
    /// Required input stack.
    pub input: Row,
    /// Expected output stack.
    pub output: Row,
}

impl Row {
    /// A completely empty stack.
    #[must_use]
    pub const fn empty() -> Self {
        Self::closed(Vec::new())
    }

    /// A stack with no unknown prefix.
    #[must_use]
    pub const fn closed(suffix: Vec<ValueType>) -> Self {
        Self {
            prefix: None,
            suffix,
        }
    }

    /// A stack with an unknown prefix.
    #[must_use]
    pub const fn open(prefix: RowVar, suffix: Vec<ValueType>) -> Self {
        Self {
            prefix: Some(prefix),
            suffix,
        }
    }
}
