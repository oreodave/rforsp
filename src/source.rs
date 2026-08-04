//! Source generation and management.
//!
//! The first stage to the compiler is slurping source code.  This stage handles
//! the raw text, and manages the tagging of

/// Module defining positions within a [Source], by byte and by character.
mod span;
/// Module for defining the [`SymbolTable`]: managing a collection of [Sources]
mod table;
/// Module defining a singular [Source]
mod text;

pub use span::{Position, Span};
pub use table::{Location, SourceId, SourceTable, SyntaxId, SyntaxOrigin};
pub use text::{MAX_SOURCE_LEN, Source, SourceError};
