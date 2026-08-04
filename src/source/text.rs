use crate::source::{Position, Span};

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
/// Source text for scanning purposes.
#[derive(Debug)]
pub struct Source {
    pub name: String,
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
impl Source {
    /// Construct a `Source` from the given `contents`.
    /// Returns Err if `from_contents_limited` fails with
    /// `limit=MAX_SOURCE_LEN`.
    pub(super) fn from_contents(
        source_name: &str,
        contents: String,
    ) -> Result<Self, SourceError> {
        Self::from_contents_limited(source_name, contents, MAX_SOURCE_LEN)
    }

    /// Construct a `Source` from the given `contents`.
    /// Returns Err if `contents.len()` > `limit`.
    pub(super) fn from_contents_limited(
        source_name: &str,
        contents: String,
        limit: usize,
    ) -> Result<Self, SourceError> {
        let name = source_name.to_string();
        if contents.len() > limit {
            return Err(SourceError::TooLarge {
                name,
                len: contents.len(),
                limit,
            });
        }

        let line_starts = compute_line_starts(&contents);
        Ok(Self {
            name,
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

    pub fn valid_span(&self, span: Span) -> bool {
        span.start <= (self.len() as u32) && span.end <= (self.len() as u32)
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
    /// NOTE: Will panic if pos is out of bounds for this source.
    pub fn chars_from(&self, pos: usize) -> std::str::Chars<'_> {
        self.contents[pos..].chars()
    }

    /// Map a span into a string in the content of this `Source`.
    /// NOTE: Will panic if span is out of bounds for this source.
    pub fn span_text(&self, span: Span) -> &str {
        &self.contents[span.start as usize..span.end as usize]
    }

    /// Converts a byte position to a `Position` within the contents of this
    /// `Source`.
    /// NOTE: Will panic if either:
    /// - `byte` is out of bounds for this source.
    /// - `byte` is not at a char boundary for this source.
    pub fn position_at(&self, byte: usize) -> Position {
        assert!(
            byte <= self.contents.len(),
            "position_at: byte {byte} out of bounds"
        );

        assert!(
            self.contents.is_char_boundary(byte),
            "position_at: byte {byte} is not in a char boundary"
        );

        let line = self
            .line_starts
            .binary_search(&(byte as u32))
            .unwrap_or_else(|i| i - 1);

        let line_start = self.line_starts[line];
        let col = self
            .span_text(Span::new(line_start as usize, byte))
            .chars()
            .count()
            + 1;

        Position::new(line + 1, col)
    }

    /// Converts a `Span` into a tuple of two `Position`s (p1, p2)
    ///
    /// p1 and p2 match the inclusive-exclusive nature of `Span` itself: p2 is
    /// *one past* the span's last character.
    ///
    /// This means span.start == span.end <=> p1 == p2.  A span covering a
    /// single line covers `p2.col - p1.col` characters.
    ///
    /// NOTE: A span with an ending position at the EOF of `self.content` will
    /// produce a p2 pointing to the position just past the last character.
    ///
    /// NOTE: Will panic based on `Source::position_at` conditions for
    /// `Span::start` _and_ `Span::end`
    pub fn span_positions(&self, span: Span) -> (Position, Position) {
        (
            self.position_at(span.start as usize),
            self.position_at(span.end as usize),
        )
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

    const SAMPLE_TEXT: &str = concat!(
        "Hello, world!\n",
        "Do Shinigami's like \u{1f34e}'s?\n",
        "I like \u{1f350}'s personally.\n"
    );

    const SAMPLE_NEWLINES: [usize; 3] = [13, 41, 67];
    const SAMPLE_EMOJIS: [usize; 2] = [34, 49];

    #[test]
    fn source_position_at() {
        let text = SAMPLE_TEXT.to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");

        // The position at the first byte is simply a default position.
        assert_eq!(source.position_at(0), Position::default());

        for newline in SAMPLE_NEWLINES {
            let at_newline = source.position_at(newline);
            let ahead_newline = source.position_at(newline + 1);

            // Position.line at a newline is 1 less than Position.line ahead of
            // the newline.
            assert_eq!(at_newline.line + 1, ahead_newline.line);
            // Position.col just after a newline is always 1.
            assert_eq!(ahead_newline.col, 1);
        }

        for emoji_position in SAMPLE_EMOJIS {
            let pos = source.position_at(emoji_position);

            // By construction, the character before an emoji won't be another
            // unicode codepoint, so this will is safe to do.
            let previous_pos = source.position_at(emoji_position - 1);

            assert_eq!(previous_pos, Position::new(pos.line, pos.col - 1));

            // By construction, exactly 4 bytes ahead of each emoji position is
            // the next "character".
            let next_pos = source.position_at(emoji_position + 4);
            assert_eq!(next_pos, Position::new(pos.line, pos.col + 1))
        }

        // Exhaustive checking of every character byte position
        let mut col = 1;
        for (pos, character) in text
            .bytes()
            .enumerate()
            // ensure we're not mid char
            .filter(|(i, _)| text.is_char_boundary(*i))
        {
            let line = text[..pos].matches('\n').count() + 1;
            assert_eq!(source.position_at(pos), Position::new(line, col));
            if character == b'\n' {
                col = 1;
            } else {
                col += 1;
            }
        }
    }

    #[test]
    #[should_panic]
    fn source_position_at_bad() {
        let text = SAMPLE_TEXT.to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");

        // By construction, the next byte after an emoji position should be
        // within the codepoint.  Thus, Source::position_at should fail.
        source.position_at(SAMPLE_EMOJIS[0] + 1);
    }

    #[test]
    fn source_span_positions() {
        // A span going to the end of the text should have a position that is
        // one past the last character.

        // When text ends in a a newline, the EOF will be on this phantom
        // newline.
        let text = "Hello, world!\n".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let span = Span::new(0, text.len());
        let (start, end) = source.span_positions(span);
        assert_eq!(start, Position::default());
        assert_eq!(end, Position::new(2, 1));

        // When text doesn't have a newline, the EOF will be placed in the
        // column just after the last character of the last line.
        let text = "Hello, world!".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let span = Span::new(0, text.len());
        let (start, end) = source.span_positions(span);
        assert_eq!(start, Position::default());
        assert_eq!(end, Position::new(1, text.len() + 1));

        // An empty span should have equivalent positions for start and end.
        let span = Span::new(2, 2);
        let (start, end) = source.span_positions(span);
        assert_eq!(span.length(), 0);
        assert_eq!(start, end);

        let text = SAMPLE_TEXT.to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let lines = [
            Span::new(0, SAMPLE_NEWLINES[0]),
            Span::new(SAMPLE_NEWLINES[0] + 1, SAMPLE_NEWLINES[1]),
            Span::new(SAMPLE_NEWLINES[1] + 1, SAMPLE_NEWLINES[2]),
        ];

        for (i, &line) in lines.iter().enumerate() {
            let (start, end) = source.span_positions(line);

            // We expect the line to mirror the indices of `lines`.
            assert_eq!(start.line, i + 1);
            // We expect the start and end to be on the same line.
            assert_eq!(start.line, end.line);
            // We expect the difference between the start and end columns to be
            // exactly the number of characters in the `span_text`.
            assert_eq!(
                source.span_text(line).chars().count(),
                end.col - start.col
            );
            // NOTE: We cannot do:
            //   assert_eq!(end.col - start.col, line.length());
            // ... since spans are bytes, and Position.col is characters.

            // Constructing a span where the end is past the newline.
            let (start, end) =
                source.span_positions(Span::from_u32(line.start, line.end + 1));

            // We still expect the line to mirror the indices of `lines`
            assert_eq!(start.line, i + 1);
            // We expect end to be on the line after start, but on the same
            // column.
            assert_eq!(end, Position::new(start.line + 1, start.col));
        }
    }
}
