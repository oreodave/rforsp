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

/// Source text for scanning purposes.
pub struct Source {
    name: String,
    contents: String,
    line_starts: Vec<u32>,
}

