//! A single source buffer.
//!
//! [`Source`] is a raw in-memory text buffer, capped at [`MAX_SOURCE_LEN`],
//! with byte-to-[`Position`] mapping used by the rest of the source stage.

use crate::source::{Position, Span, span::offset};

/// Contiguous collection of text for scanning purposes.
#[derive(Debug)]
pub struct Source {
    /// Name of the Source.
    pub name: String,
    /// Contents of a Source.
    contents: String,
    /// Byte position for the starts of lines in the Source.
    line_starts: Vec<u32>,
}

/// Maximum source length in bytes.  [Span]'s fields are u32, so they work
/// happily with content of at most this size.
pub const MAX_SOURCE_LEN: usize = u32::MAX as usize;

/// Error in constructing a Source.
#[derive(Debug)]
pub enum SourceError {
    /// Given contents for Source construction was too large
    TooLarge {
        /// Name of source
        name: String,
        /// Length of source
        len: usize,
        /// Limit for how large the source may be
        limit: usize,
    },
}

/// Compute the starting byte positions of every line within `contents`.
fn compute_line_starts(contents: &str) -> Vec<u32> {
    let mut line_starts: Vec<u32> = vec![0];
    line_starts.extend(
        contents
            .bytes()
            .enumerate()
            .filter(|(_, c)| *c == b'\n')
            .map(|(i, _)| offset(i + 1)),
    );
    line_starts
}

impl Source {
    /// Construct a [Source] from the given `contents`.
    /// Returns Err if [`Source::from_contents_limited`] fails with
    /// `limit`=[`MAX_SOURCE_LEN`].
    pub(super) fn from_contents(
        source_name: &str,
        contents: String,
    ) -> Result<Self, SourceError> {
        Self::from_contents_limited(source_name, contents, MAX_SOURCE_LEN)
    }

    /// Construct a [Source] from the given `contents`.
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

    /// Get the length of the [Source].
    #[must_use]
    pub const fn len(&self) -> usize {
        self.contents.len()
    }

    /// Check if the [Source] is empty. (stupid)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the given byte position points to the end of the source
    /// content.
    #[must_use]
    pub const fn eos(&self, pos: usize) -> bool {
        self.len() <= pos
    }

    /// Check if the given `span` is valid for source.
    #[must_use]
    pub const fn valid_span(&self, span: Span) -> bool {
        (span.start as usize) <= self.len() && (span.end as usize) <= self.len()
    }

    /// Get the bytes of the contents of this [Source].
    #[must_use]
    pub const fn bytes(&self) -> &[u8] {
        self.contents.as_bytes()
    }

    /// Get the characters of the contents of this [Source].
    #[must_use]
    pub fn text(&self) -> &str {
        &self.contents
    }

    /// Get the characters of the contents of this [Source] from a byte position
    /// onwards.
    ///
    /// # Panics
    /// - if pos is out of bounds for this source.
    /// - if pos is in a char boundary.
    pub fn chars_from(&self, pos: usize) -> std::str::Chars<'_> {
        assert!(pos <= self.len(), "{pos} is out of bounds");
        assert!(
            self.contents.is_char_boundary(pos),
            "{pos} is not in a char boundary"
        );
        self.contents[pos..].chars()
    }

    /// Map a span into a string in the content of this [Source].
    ///
    /// # Panics
    /// - if span is out of bounds for this source
    #[must_use]
    pub fn span_text(&self, span: Span) -> &str {
        assert!(self.valid_span(span), "{span:?} is invalid for this source");
        &self.contents[span.start as usize..span.end as usize]
    }

    /// Get the index of the line that contains this `byte` within this Source.
    ///
    /// NOTE: It is presumed that `byte` is within bounds.  But it doesn't need
    /// to be in a char boundary for this to work.
    #[must_use]
    fn line_index(&self, byte: u32) -> usize {
        self.line_starts
            .binary_search(&byte)
            .unwrap_or_else(|i| i - 1)
    }

    /// Converts a byte position to a [Position] within the contents of this
    /// [Source].
    ///
    /// # Panics
    /// - if `byte` is out of bounds for this source.
    /// - if `byte` is not at a char boundary for this source.
    #[must_use]
    pub fn position_at(&self, byte: usize) -> Position {
        assert!(
            byte <= self.contents.len(),
            "position_at: byte {byte} out of bounds"
        );

        assert!(
            self.contents.is_char_boundary(byte),
            "position_at: byte {byte} is not in a char boundary"
        );

        let byte = offset(byte);

        let line = self.line_index(byte);

        let line_start = self.line_starts[line];
        let col = self
            .span_text(Span::from_u32(line_start, byte))
            .chars()
            .count()
            + 1;

        Position::new(line + 1, col)
    }

    /// Converts a [Span] into a tuple of two [Position]'s (p1, p2)
    ///
    /// p1 and p2 match the inclusive-exclusive nature of [Span] itself: p2 is
    /// *one past* the span's last character.
    ///
    /// This means span.start == span.end <=> p1 == p2.  A span covering a
    /// single line covers `p2.col - p1.col` characters.
    ///
    /// NOTE: A span with an ending position at the EOF of `self.content` will
    /// produce a p2 pointing to the position just past the last character.
    ///
    /// # Panics
    /// - Based on [`Source::position_at`] conditions for [`Span::start`] _and_
    ///   [`Span::end`]
    #[must_use]
    pub fn span_positions(&self, span: Span) -> (Position, Position) {
        (
            self.position_at(span.start as usize),
            self.position_at(span.end as usize),
        )
    }

    /// Get the text of a full line given the line number.
    ///
    /// NOTE: `line` is 1-indexed.
    ///
    /// # Panics
    /// - if line is out of bounds
    #[must_use]
    pub fn line_text(&self, line: usize) -> &str {
        assert!(line > 0, "line must be 1-indexed");
        assert!(line <= self.line_starts.len(), "{line} is out of bounds");
        let start = self.line_starts[line - 1];
        let end = {
            if line == self.line_starts.len() {
                u32::try_from(self.len()).expect("self.len() <= MAX_SOURCE_LEN")
            } else {
                self.line_starts[line] - 1
            }
        };
        self.span_text(Span::from_u32(start, end))
    }

    /// Get the text of the full line an offset is located in.
    ///
    /// # Panics
    /// - if the offset is out of bounds
    #[must_use]
    pub fn line_text_of(&self, byte: usize) -> &str {
        assert!(byte <= self.len(), "{byte} is out of bounds");
        let offset = offset(byte);
        let line = self.line_index(offset);
        self.line_text(line + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction() {
        const LIMIT: usize = 64;
        assert!(
            Source::from_contents_limited("", "a".repeat(LIMIT), LIMIT).is_ok()
        );
        assert!(
            Source::from_contents_limited("", String::new(), LIMIT).is_ok()
        );
    }

    #[test]
    fn construction_invalid_contents_length() {
        const LIMIT: usize = 64;
        const OVER_LIMIT: usize = LIMIT + 1;
        let contents = "a".repeat(LIMIT + 1);
        assert!(matches!(
            Source::from_contents_limited("", contents, LIMIT),
            Err(SourceError::TooLarge {
                len: OVER_LIMIT,
                ..
            })
        ));
    }

    #[test]
    fn eos() {
        assert!(
            Source::from_contents("", String::new())
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
    fn destructors() {
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
    #[should_panic(expected = "out of bounds")]
    fn chars_from_invalid_offset() {
        let text = "testing testing".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let _ = source.chars_from(text.len() + 1);
    }

    #[test]
    #[should_panic(expected = "invalid")]
    fn span_text_invalid_end() {
        let text = "testing testing".to_string();
        let source =
            Source::from_contents("", text.clone()).expect("Should not fail");
        let _ = source.span_text(Span::new(0, text.len() + 1));
    }

    const SAMPLE_TEXT: &str = concat!(
        "Hello, world!\n",
        "Do Shinigami's like \u{1f34e}'s?\n",
        "I like \u{1f350}'s personally.\n"
    );

    const SAMPLE_LINES: [&str; 3] = [
        "Hello, world!",
        "Do Shinigami's like \u{1f34e}'s?",
        "I like \u{1f350}'s personally.",
    ];

    const SAMPLE_NEWLINES: [usize; 3] = [13, 41, 67];
    const SAMPLE_EMOJIS: [usize; 2] = [34, 49];

    #[test]
    fn position_at() {
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
            assert_eq!(next_pos, Position::new(pos.line, pos.col + 1));
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
    #[should_panic(expected = "char boundary")]
    fn position_at_in_char_boundary() {
        let text = SAMPLE_TEXT.to_string();
        let source = Source::from_contents("", text).expect("Should not fail");

        // By construction, the next byte after an emoji position should be
        // within the codepoint.  Thus, Source::position_at should fail.
        let _ = source.position_at(SAMPLE_EMOJIS[0] + 1);
    }

    #[test]
    fn span_positions() {
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
        let source = Source::from_contents("", text).expect("Should not fail");
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

    fn sample_no_trailing_newline() -> String {
        String::from(&SAMPLE_TEXT[..SAMPLE_TEXT.len() - 1])
    }

    #[test]
    fn line_text() {
        let text = SAMPLE_TEXT.to_string();
        let source = Source::from_contents("", text).expect("Should not fail");

        for (i, &expected) in SAMPLE_LINES.iter().enumerate() {
            assert_eq!(source.line_text(i + 1), expected);
        }

        // A source ending in a newline has a phantom empty final line.
        assert_eq!(source.line_text(4), "");

        // A text with no trailing newline still can iterate through all lines.
        let text = sample_no_trailing_newline();
        let source = Source::from_contents("", text).expect("Should not fail");
        for (i, &expected) in SAMPLE_LINES.iter().enumerate() {
            assert_eq!(source.line_text(i + 1), expected);
        }

        // An empty source means we have an empty starting line
        let source =
            Source::from_contents("", String::new()).expect("Should not fail");
        assert_eq!(source.line_text(1), "");
    }

    #[test]
    fn line_text_of() {
        let text = SAMPLE_TEXT.to_string();
        let source = Source::from_contents("", text).expect("Should not fail");

        // The first byte of the source is on line 1.
        assert_eq!(source.line_text_of(0), "Hello, world!");
        // A byte at a line start resolves to that line.
        assert_eq!(
            source.line_text_of(SAMPLE_NEWLINES[0] + 1),
            SAMPLE_LINES[1],
        );
        assert_eq!(
            source.line_text_of(SAMPLE_NEWLINES[1] + 1),
            SAMPLE_LINES[2],
        );
        // A mid-line byte resolves to its owning line.
        assert_eq!(source.line_text_of(20), SAMPLE_LINES[1]);
        // A byte at a newline still belongs to the line it terminates.
        assert_eq!(source.line_text_of(SAMPLE_NEWLINES[0]), SAMPLE_LINES[0]);

        // Line membership is a byte fact: a byte inside a multi-byte codepoint
        // still resolves to the owning line (no char-boundary requirement).
        let expected = SAMPLE_LINES[1];
        // `SAMPLE_EMOJIS[0]` is the first byte of a 4-byte emoji in line 2.
        for offset in SAMPLE_EMOJIS[0] + 1..SAMPLE_EMOJIS[0] + 4 {
            assert_eq!(source.line_text_of(offset), expected);
        }

        // The byte one past the end of a newline-terminated source resolves to
        // the phantom final line.
        assert_eq!(source.line_text_of(source.len()), "");

        // For a source without an ending newline, it resolves to the final line
        // of text.
        let source = Source::from_contents("", sample_no_trailing_newline())
            .expect("Should not fail");

        assert_eq!(source.line_text_of(source.len()), SAMPLE_LINES[2]);
    }

    #[test]
    #[should_panic(expected = "1-indexed")]
    fn line_text_zero_line() {
        let source = Source::from_contents("", sample_no_trailing_newline())
            .expect("Should not fail");
        let _ = source.line_text(0);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn line_text_out_of_bounds() {
        let source = Source::from_contents("", sample_no_trailing_newline())
            .expect("Should not fail");
        let _ = source.line_text(4);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn line_text_of_out_of_bounds() {
        let source = Source::from_contents("", sample_no_trailing_newline())
            .expect("Should not fail");
        let _ = source.line_text_of(source.len() + 1);
    }
}
