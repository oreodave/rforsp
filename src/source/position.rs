//! Character Positions.
//!
//! Defines [`Position`], a character-oriented line-column location.

/// Character Line-Column position in some source text.
///
/// Both count characters, not bytes, and both start at 1.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct Position {
    /// Line number.
    pub line: usize,
    /// Column number.
    pub col: usize,
}

impl Position {
    /// Construct a new position from the given `line` and `col`.
    #[must_use]
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 1, col: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(Position::default(), Position::new(1, 1));
    }
}
