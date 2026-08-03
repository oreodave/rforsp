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

    /// Get the length of this Source text.
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// Check if the given byte position points to the end of the source
    /// content.
    pub fn eos(&self, pos: usize) -> bool {
        self.len() <= pos
    }

}

impl From<std::io::Error> for SourceError {
    fn from(e: std::io::Error) -> Self {
        SourceError::Io(e)
    }
}

impl std::error::Error for SourceError {}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::Io(e) => write!(f, "{e}"),
            SourceError::TooLarge { name, len, limit } => {
                write!(f, "{name}: Contains {len} bytes when limit is {limit}")
            }
        }
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
        assert_eq!(Position::default(), Position { line: 1, col: 1 });
    }

    #[test]
    fn span_length() {
        assert_eq!(Span::default().length(), 0);
        assert_eq!(Span::new(100, 100).length(), 0);
        assert_eq!(Span::new(0, 100).length(), 100);
    }

    #[test]
    fn source_construction() {
        const LIMIT: usize = 64;
        assert!(
            Source::from_contents_limited("", "a".repeat(LIMIT), LIMIT).is_ok()
        );
        assert!(
            Source::from_contents_limited("", "".to_string(), LIMIT).is_ok()
        );
    }

    #[test]
    fn source_construction_bad() {
        const LIMIT: usize = 64;
        const OVER_LIMIT: usize = LIMIT + 1;
        let contents = "a".repeat(LIMIT + 1);
        assert!(matches!(
            Source::from_contents_limited("", contents, LIMIT),
            Err(SourceError::TooLarge {
                len: OVER_LIMIT,
                ..
            })
        ))
    }

    #[test]
    fn source_eos() {
        assert!(
            Source::from_contents("", "".to_string())
                .expect("Empty string should not fail source construction.")
                .eos(0)
        );

        let text = "Hello, world!";
        let source = Source::from_contents("", text.to_string())
            .expect("Should not fail construction");
        assert!(source.eos(text.len()));
        assert!((0..text.len()).all(|i| !source.eos(i)));
    }

}
