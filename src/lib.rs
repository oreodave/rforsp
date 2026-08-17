//! The rForsp compiler library.
//!
//! Each phase lives in its own module.  [`drivers`] threads them together and
//! the executable drives that.

pub mod diagnostics;
pub mod interner;

pub mod context;

pub mod lexer;
pub mod parser;
pub mod resolution;
pub mod source;

pub mod drivers;
pub mod log;
