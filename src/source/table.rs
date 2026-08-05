//! The session source table.
//!
//! The append-only [`SourceTable`] holds every [`Source`] and assigns each a
//! unique [`SourceId`], registers [`SyntaxOrigin`]s as [`SyntaxId`]s, and
//! resolves them back to [`Location`]s and text for diagnostics.

use crate::source::{Position, Source, SourceError, Span};

/// ID for a [Source] in the [`SourceTable`]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SourceId(u32);

/// ID for a [`SyntaxOrigin`] in the [`SourceTable`]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SyntaxId(u32);

/// Special syntactical structure within a [Source], specified by a [Span].
#[derive(Debug, Copy, Clone)]
pub struct SyntaxOrigin {
    /// The [`SourceId`] of the [Source] this [`SyntaxOrigin`] is located in.
    pub source: SourceId,
    /// The [Span] within the [Source] that this [`SyntaxOrigin`] relates to.
    pub span: Span,
}

/// Collated metadata for the location of code within the [`SourceTable`]
#[derive(Debug, Copy, Clone)]
pub struct Location<'a> {
    /// Name of the [Source] this location relates to.
    pub name: &'a str,
    /// Starting [Position] of this LOC
    pub start: Position,
    /// End [Position] (1 past last character) of this LOC
    pub end: Position,
}

/// Session-time Table of [Source]s, with relevant metadata for the compiler.
#[derive(Debug)]
pub struct SourceTable {
    /// Collection of [Source]'s managed by this table.
    sources: Vec<Source>,
    /// Collection of [`SyntaxOrigin`]'s pointing at [Source]'s within this
    /// table.
    origins: Vec<SyntaxOrigin>,
}

/// Possible errors that may arise during [Source] construction.
#[derive(Debug)]
pub enum SourceTableError {
    /// Error arose when creating the raw [Source]
    SourceCreate(SourceError),
    /// Error arose when doing a IO read operation.
    Io(std::io::Error),
}

impl From<SourceError> for SourceTableError {
    fn from(e: SourceError) -> Self {
        Self::SourceCreate(e)
    }
}

impl SourceTable {
    /// Construct a new source table.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sources: Vec::new(),
            origins: Vec::new(),
        }
    }

    /// Add a new source from the contents of the given `file_name`.
    ///
    /// # Errors
    /// - If there is an IO error when reading `file_name`.
    /// - If there is an error with constructing the source
    pub fn add_source_file(
        &mut self,
        file_name: &str,
    ) -> Result<SourceId, SourceTableError> {
        let contents = std::fs::read_to_string(file_name)?;
        self.add_source_raw(file_name, contents)
    }

    /// Add a new source given `source_name` and `contents`.
    ///
    /// # Panics
    /// - if `self.sources.len()` > [`u32::MAX`].
    ///
    /// # Errors
    /// - If there is an error with constructing the source
    pub fn add_source_raw(
        &mut self,
        source_name: &str,
        contents: String,
    ) -> Result<SourceId, SourceTableError> {
        let source = Source::from_contents(source_name, contents)?;
        let id = SourceId(
            u32::try_from(self.sources.len()).expect("|sources| > u32::MAX"),
        );
        self.sources.push(source);
        Ok(id)
    }

    /// Get the Source related to the given `id`
    ///
    /// # Panics
    /// - if `id` is out of bounds of `self.sources`.
    #[must_use]
    pub fn get_source(&self, id: SourceId) -> &Source {
        let id = id.0 as usize;
        assert!(id < self.sources.len());
        &self.sources[id]
    }

    /// Add a new [`SyntaxOrigin`] given the `id` and `span`.
    ///
    /// # Panics
    /// - if `span` is not valid for the [Source] related to `id`.
    /// - if `self.origins.len()` > [`u32::MAX`].
    pub fn add_origin(&mut self, id: SourceId, span: Span) -> SyntaxId {
        let source = self.get_source(id);
        assert!(source.valid_span(span));
        let syn_id = SyntaxId(
            u32::try_from(self.origins.len()).expect("|origins| > u32::MAX"),
        );
        self.origins.push(SyntaxOrigin { source: id, span });
        syn_id
    }

    /// Get the [`SyntaxOrigin`] associated to the given `id`.
    ///
    /// # Panics
    /// - if `id` is out of bounds of `self.origins`.
    #[must_use]
    pub fn get_origin(&self, id: SyntaxId) -> &SyntaxOrigin {
        let id = id.0 as usize;
        assert!(id < self.origins.len());
        &self.origins[id]
    }

    /// Get the [Location] within the source of a given `id`
    #[must_use]
    pub fn location_of(&self, id: SyntaxId) -> Location<'_> {
        let SyntaxOrigin { source, span } = self.get_origin(id);
        let source = self.get_source(*source);
        let (start, end) = source.span_positions(*span);
        Location {
            name: &source.name,
            start,
            end,
        }
    }

    /// Get the text within the source of a given `id`.
    #[must_use]
    pub fn text_of(&self, id: SyntaxId) -> &str {
        let SyntaxOrigin { source, span } = self.get_origin(id);
        let source = self.get_source(*source);
        source.span_text(*span)
    }
}

impl Default for SourceTable {
    fn default() -> Self {
        Self::new()
    }
}

impl From<std::io::Error> for SourceTableError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for SourceTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceCreate(e) => {
                write!(f, "{e}")
            }
            Self::Io(e) => {
                write!(f, "{e}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCES: [(&str, &str); 2] = [
        ("a", concat!("hello\n", "world!\n")),
        ("b", concat!("Foo\n", "bar\n")),
    ];

    fn add_sources(table: &mut SourceTable) -> [SourceId; SOURCES.len()] {
        SOURCES.map(|(name, text)| {
            table
                .add_source_raw(name, text.into())
                .expect("This should not fail")
        })
    }

    #[test]
    fn sources() {
        let mut table = SourceTable::new();
        let source_ids = add_sources(&mut table);

        assert!(source_ids[0].0 < source_ids[1].0);

        for (&s_id, (name, text)) in source_ids.iter().zip(SOURCES.iter()) {
            let source = table.get_source(s_id);
            assert_eq!(source.name, *name);
            assert_eq!(source.text(), *text);
        }
    }

    #[test]
    fn syntax_ids() {
        let mut table = SourceTable::new();
        let source_ids = add_sources(&mut table);
        let cases = [
            (
                source_ids[0],
                Span::new(0, SOURCES[0].1.len()),
                SOURCES[0].0,
                Position::default(),
                Position { line: 3, col: 1 },
                SOURCES[0].1,
            ),
            (
                source_ids[0],
                Span::new(2, 3),
                SOURCES[0].0,
                Position { line: 1, col: 3 },
                Position { line: 1, col: 4 },
                "l",
            ),
            (
                source_ids[1],
                Span::new(0, SOURCES[1].1.len()),
                SOURCES[1].0,
                Position::default(),
                Position { line: 3, col: 1 },
                SOURCES[1].1,
            ),
            (
                source_ids[1],
                Span::new(2, 5),
                SOURCES[1].0,
                Position { line: 1, col: 3 },
                Position { line: 2, col: 2 },
                "o\nb",
            ),
            (
                source_ids[0],
                Span::new(0, 0),
                SOURCES[0].0,
                Position::default(),
                Position { line: 1, col: 1 },
                "",
            ),
        ];

        let syntax_ids = cases
            .map(|(source, span, _, _, _, _)| table.add_origin(source, span));

        for window in syntax_ids.windows(2) {
            let SyntaxId(a) = window[0];
            let SyntaxId(b) = window[1];
            assert!(a < b);
        }

        for (&syntax_id, (source, span, name, start, end, text)) in
            syntax_ids.iter().zip(cases.iter())
        {
            let origin = table.get_origin(syntax_id);
            // Test that getting the origin yields us the input components.
            assert_eq!(origin.source, *source);
            assert_eq!(origin.span, *span);

            // Test location_of works as we expect, across sources.
            let location = table.location_of(syntax_id);
            assert_eq!(location.name, *name);
            assert_eq!(location.start, *start);
            assert_eq!(location.end, *end);

            let actual_text = table.text_of(syntax_id);
            assert_eq!(*text, actual_text);
        }
    }

    #[test]
    fn sources_invalid_filename() {
        let mut table = SourceTable::new();
        let res = table.add_source_file("/no-way/this-is/real");
        assert!(res.is_err());
    }

    #[test]
    #[should_panic(expected = "id < self.sources.len()")]
    fn get_source_invalid_id() {
        let table = SourceTable::new();
        let bad_id = SourceId(10);
        let _ = table.get_source(bad_id);
    }

    #[test]
    #[should_panic(expected = "valid_span")]
    fn add_origin_invalid_span() {
        let mut table = SourceTable::new();
        let source_ids = add_sources(&mut table);
        let _ = table
            .add_origin(source_ids[0], Span::new(0, SOURCES[0].1.len() + 1));
    }

    #[test]
    #[should_panic(expected = "id < self.sources.len()")]
    fn add_origin_invalid_source() {
        let mut table = SourceTable::new();
        let _ = table.add_origin(SourceId(1024), Span::new(0, 0));
    }

    #[test]
    #[should_panic(expected = "id < self.origins.len()")]
    fn get_origin_invalid_id() {
        let table = SourceTable::new();
        let _ = table.get_origin(SyntaxId(1024));
    }
}
