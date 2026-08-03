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

/// Maximum source length in bytes.  `Span`s fields are u32, so they work
/// happily with content of at most this size.
pub const MAX_SOURCE_LEN: usize = u32::MAX as usize;

/// Error in constructing a Source.
#[derive(Debug)]
pub enum SourceError {
    TooLarge {
        name: String,
        len: usize,
        limit: usize,
    },
    Io(std::io::Error),
}

/******************************************************************************
 * Standalone                                                                 *
 ******************************************************************************/
fn compute_line_starts(contents: &str) -> Vec<u32> {
    let mut line_starts: Vec<u32> = vec![0];
    line_starts.extend(
        contents
            .bytes()
            .enumerate()
            .filter_map(|(i, c)| (c == b'\n').then(|| (i + 1) as u32))
            .collect::<Vec<u32>>(),
    );
    line_starts
}
