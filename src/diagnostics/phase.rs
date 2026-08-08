//! Phases of the Compiler and Aborted phase gate.

/// Phase of the compiler diagnostics may originate from.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Phase {
    /// Source phase.
    Source,
}

/// Error value for an Aborted [`Phase`].
#[derive(Debug, Copy, Clone)]
pub struct Aborted(Phase);

impl Phase {
    /// Convert Phase to a `str`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "Source",
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
