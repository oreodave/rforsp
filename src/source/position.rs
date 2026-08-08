//! Character Positions.
//!
//! Defines [`Position`], a character-oriented line-column location.

/// Character Line-Column position in some source text.
///
/// NOTE: Default initialisation sets these to {1, 1}.
///
/// NOTE: These are not byte-oriented, but character oriented.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct Position {
    /// Line count
    pub line: usize,
    /// Column count, in characters.
    pub col: usize,
}

impl Position {
    /// Construct a new postion from the given `line` and `col`.
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
