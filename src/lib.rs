//! The rForsp compiler runtime.
//!
//! This is the top-level crate of the rForsp compiler project.  It links
//! together all the disparate modules of the compiler runtime, which is then
//! driven by the main compiler executable.

#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

pub mod diagnostics;
pub mod interner;

pub mod context;

pub mod lexer;
pub mod parser;
pub mod source;

pub mod drivers;
pub mod log;
