//! Byte Spans.
//!
//! Defines [`Span`], a byte range within a [`Source`][crate::source::Source].

use crate::u32_index;

/// A byte span, composed of a start and end position.
///
/// The range is half-open: `[start, end)`.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Default)]
pub struct Span {
    /// Byte offset of the first character in the span.
    pub start: u32,
    /// Byte offset one past the last character in the span.
    pub end: u32,
}

impl Span {
    /// Construct a new Span from usize components.
    ///
    /// # Panics
    /// - If either component is greater than
    ///   [`MAX_SOURCE_LEN`][crate::source::MAX_SOURCE_LEN].
    /// - If `start > end`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self::from_u32(u32_index(start), u32_index(end))
    }

    /// Construct a new [Span] from u32 components.
    ///
    /// # Panics
    /// - if `start > end`.
    #[must_use]
    pub const fn from_u32(start: u32, end: u32) -> Self {
        assert!(start <= end, "Span must be an ordered range");
        Self { start, end }
    }

    /// Return the length of this [Span] in bytes.
    #[must_use]
    pub const fn length(&self) -> u32 {
        self.end - self.start
    }

    /// Return the smallest [Span] that covers both.
    ///
    /// Anything between two disjoint spans is covered too.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        Self::from_u32(self.start.min(other.start), self.end.max(other.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(Span::default(), Span::new(0, 0));
    }

    #[test]
    #[should_panic(expected = "ordered range")]
    fn span_new_misordered_start_end() {
        let _ = Span::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "ordered range")]
    fn span_from_u32_misordered_start_end() {
        let _ = Span::from_u32(10, 9);
    }

    #[test]
    fn span_length() {
        assert_eq!(Span::default().length(), 0);
        assert_eq!(Span::new(100, 100).length(), 0);
        assert_eq!(Span::new(0, 100).length(), 100);
    }
}
