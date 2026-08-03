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

/******************************************************************************
 * Implementations                                                            *
 ******************************************************************************/
impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= MAX_SOURCE_LEN);
        debug_assert!(end <= MAX_SOURCE_LEN);
        debug_assert!(start <= end);
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    /// Returns the length of this [`Span`].
    pub const fn length(&self) -> u32 {
        self.end - self.start
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 1, col: 1 }
    }
}

impl Source {
    /// Construct a source from the given `file_name`, reading its contents.
    /// Propagates error from reading the file.
    pub fn from_file(file_name: &str) -> Result<Self, SourceError> {
        Self::from_contents(file_name, std::fs::read_to_string(file_name)?)
    }

    /// Construct a source from the given `contents` string.
    pub fn from_contents(
        stream_name: &str,
        contents: String,
    ) -> Result<Self, SourceError> {
        Self::from_contents_limited(stream_name, contents, MAX_SOURCE_LEN)
    }

    pub fn from_contents_limited(
        stream_name: &str,
        contents: String,
        limit: usize,
    ) -> Result<Self, SourceError> {
        if contents.len() > limit {
            return Err(SourceError::TooLarge {
                name: stream_name.to_string(),
                len: contents.len(),
                limit: limit,
            });
        }

        let line_starts = compute_line_starts(&contents);
        Ok(Self {
            name: stream_name.to_string(),
            contents,
            line_starts,
        })
    }
}
