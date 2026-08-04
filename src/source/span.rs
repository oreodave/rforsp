use crate::source::MAX_SOURCE_LEN;

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
/// A byte span, composed of a start and end position.
/// NOTE: This span maps to [start, end) i.e. an exclusive ended range.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// Character Line-Column position in some source text.
/// NOTE: Default initialisation sets these to {1, 1}.
/// NOTE: These are not byte-oriented, but character oriented.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

/******************************************************************************
 * Standalone                                                                 *
 ******************************************************************************/
/// Return `pos` as an offset (usize -> u32).
///
/// # Panics
/// - if `pos > MAX_SOURCE_LEN`.
#[track_caller]
pub(super) const fn offset(pos: usize) -> u32 {
    assert!(pos <= MAX_SOURCE_LEN);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "MUST: `contents.len() <= MAX_SOURCE_LEN`."
    )]
    {
        pos as u32
    }
}

/******************************************************************************
 * Implementations                                                            *
 ******************************************************************************/
impl Span {
    /// Construct a new Span from usize components.
    ///
    /// # Panics
    /// - If either component is greater than `MAX_SOURCE_LEN`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self::from_u32(offset(start), offset(end))
    }

    /// Construct a new `Span` from u32 components.
    ///
    /// # Panics
    /// - if `start > end`.
    #[must_use]
    pub const fn from_u32(start: u32, end: u32) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    /// Returns the length of this [`Span`].
    #[must_use]
    pub const fn length(&self) -> u32 {
        self.end - self.start
    }
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

/******************************************************************************
 * Tests                                                                      *
 ******************************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(Span::default(), Span::new(0, 0));
        assert_eq!(Position::default(), Position::new(1, 1));
    }

    #[test]
    #[should_panic(expected = "start <= end")]
    fn span_new_bad() {
        let _ = Span::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "start <= end")]
    fn span_from_u32_bad() {
        let _ = Span::from_u32(10, 9);
    }

    #[test]
    fn span_length() {
        assert_eq!(Span::default().length(), 0);
        assert_eq!(Span::new(100, 100).length(), 0);
        assert_eq!(Span::new(0, 100).length(), 100);
    }
}
