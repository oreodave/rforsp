//! Phases of the Compiler and Aborted phase gate.

/// Phase of the compiler diagnostics may originate from.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Phase {
    /// Internal Compiler Error.
    ICE,
    /// Source phase.
    Source,
    /// Lexing phase.
    Lex,
    /// Parsing phase.
    Parse,
}

/// Error value for an Aborted [`Phase`].
///
/// Deliberately neither [`Copy`] nor [`Clone`]: one abort is one compilation
/// ending, and a copy of it could outlive that.
#[derive(Debug, PartialEq, Eq)]
pub struct Aborted(pub Phase);

impl Phase {
    /// Convert Phase to a `str`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ICE => "ice",
            Self::Source => "source",
            Self::Lex => "lex",
            Self::Parse => "parse",
        }
    }
}

impl Aborted {
    /// Construct a new instance of `Aborted` for the given [`Phase`].
    #[must_use]
    pub const fn new(phase: Phase) -> Self {
        Self(phase)
    }
}
