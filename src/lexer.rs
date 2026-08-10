//! First stage lexer

mod token;
pub use token::{Token, TokenKind};

mod error;
pub use error::{LexError, LexErrorKind};

mod tokeniser;
