//! Phase 2 - parser
//!
//! The second stage of the compiler parses streams of
//! [`Token`][crate::lexer::Token]s into a sequence of [`HirForm`]s, which is
//! the Higher Level Intermediate Representation of the compiler.

mod hir;
pub use hir::{HirForm, HirKind, dfs};

mod error;
pub use error::{ParseError, ParseErrorKind};

mod parse;
pub use parse::parse;
