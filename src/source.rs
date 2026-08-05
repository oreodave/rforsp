//! Source acquisition and management.
//!
//! The first stage of the compiler: reading raw source text into a [`Source`]
//! registered in a session-global [`SourceTable`]. Byte offsets become
//! [`Span`]s and line-column [`Position`]s, each form is tagged with a
//! [`SyntaxId`], and diagnostics resolve those origins back to text.

mod span;
mod table;
mod text;

pub use span::{Position, Span};
pub use table::{Location, SourceId, SourceTable, SyntaxId, SyntaxOrigin};
pub use text::{MAX_SOURCE_LEN, Source, SourceError};
