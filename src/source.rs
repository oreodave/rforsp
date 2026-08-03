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
    /// Construct a new Span from usize components.
    /// NOTE: Will panic if either component is greater than `MAX_SOURCE_LEN`.
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(end <= MAX_SOURCE_LEN);
        assert!(start <= MAX_SOURCE_LEN);
        Self::from_u32(start as u32, end as u32)
    }

    /// Construct a new `Span` from u32 components.
    /// NOTE: Will panic if `start > end`.
    pub const fn from_u32(start: u32, end: u32) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    /// Returns the length of this [`Span`].
    pub const fn length(&self) -> u32 {
        self.end - self.start
    }
}

impl Position {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
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

    /// Get the bytes of the contents of this `Source`.
    pub fn bytes(&self) -> &[u8] {
        self.contents.as_bytes()
    }

    /// Get the characters of the contents of this `Source`.
    pub fn text(&self) -> &str {
        &self.contents
    }

    /// Get the characters of the contents of this `Source` from a byte position
    /// onwards.
    /// NOTE: This will panic if pos is out of bounds for the given `Source`.
    pub fn chars_from(&self, pos: usize) -> std::str::Chars<'_> {
        self.contents[pos..].chars()
    }

    /// Map a span into a string in the content of this `Source`.
    /// NOTE: This will panic if span is out of bounds for the given `Source`.
    pub fn span_text(&self, span: Span) -> &str {
        &self.contents[span.start as usize..span.end as usize]
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
        assert_eq!(Position::default(), Position::new(1, 1));
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

    #[test]
    fn source_destructors() {
        // Test the text destructors for Source as soft-wrappers for the
        // underlying content string.

        let text = "Hello, world!".to_string();
        let source = Source::from_contents("", text.clone())
            .expect("Should not fail construction");

        assert_eq!(source.text(), text);
        assert_eq!(source.bytes(), text.as_bytes());

        assert_eq!(source.chars_from(text.len()).next(), None);
        for (i, c) in text.chars().enumerate() {
            assert_eq!(source.chars_from(i).next(), Some(c));
        }

        {
            let source_iter: Vec<char> =
                source.chars_from(text.len() / 2).collect();
            let text_iter: Vec<char> =
                text.chars().skip(text.len() / 2).collect();
            assert_eq!(source_iter, text_iter);
        }

        let components = ["Hello", "world"];
        let spans = [Span::new(0, 5), Span::new(7, text.len() - 1)];
        for (&component, &span) in components.iter().zip(spans.iter()) {
            assert_eq!(component, source.span_text(span));
        }
    }

    #[test]
    #[should_panic]
    fn source_chars_from_bad() {
        let text = "testing testing".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let _ = source.chars_from(text.len() + 1);
    }

    #[test]
    #[should_panic]
    fn source_span_text_bad() {
        let text = "testing testing".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        source.span_text(Span::new(0, text.len() + 1));
    }
}
