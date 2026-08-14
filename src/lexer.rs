//! Phase 1 - lexer
//!
//! The first stage of the compiler translates raw textual source code from the
//! [`SourceTable`][crate::source::SourceTable] into tokens, which are simply
//! [`Span`][crate::source::Span]s within a specific
//! [`Source`][crate::source::Source] with a type attached.

mod charclass;

mod token;
pub use token::{Token, TokenKind};

mod error;
pub use error::{LexError, LexErrorKind};

mod tokeniser;
pub use tokeniser::tokenise;
