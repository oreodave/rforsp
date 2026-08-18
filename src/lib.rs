//! The rForsp compiler library.
//!
//! Each phase lives in its own module.  [`drivers`] threads them together and
//! the executable drives that.

pub mod diagnostics;
pub mod interner;
pub mod runtime;

pub mod lexer;
pub mod parser;
pub mod resolution;
pub mod source;

pub mod context;
pub mod drivers;
pub mod log;

/// Return `pos` as a u32 (usize -> u32).
///
/// # Panics
/// - if `pos > u32::MAX`.
#[track_caller]
#[must_use]
pub(crate) const fn u32_index(pos: usize) -> u32 {
    assert!(pos <= u32::MAX as usize, "pos should be at most u32::MAX");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Verified `pos` <= u32::MAX already"
    )]
    {
        pos as u32
    }
}
