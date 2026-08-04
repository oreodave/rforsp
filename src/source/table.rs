use crate::source::{Position, Source, SourceError, Span};

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
/// ID for a [Source] in the [`SourceTable`]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SourceId(u32);

/// ID for a [`SyntaxOrigin`] in the [`SourceTable`]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SyntaxId(u32);

/// Special syntactical structure within a [Source], specified by a [Span].
#[derive(Debug, Copy, Clone)]
pub struct SyntaxOrigin {
    /// The [`SourceId`] of the [`Source`] this [`SyntaxOrigin`] is located in.
    pub source: SourceId,
    /// The [`Span`] within the [`Source`] that this [`SyntaxOrigin`] relates
    /// to.
    pub span: Span,
}

/// Session-time Table of [Source]s, with relevant metadata for the compiler.
#[derive(Debug)]
pub struct SourceTable {
    /// Collection of [`Source`]'s managed by this table.
    sources: Vec<Source>,
    /// Collection of [`SyntaxOrigin`]'s pointing at [Source]'s within this
    /// table.
    origins: Vec<SyntaxOrigin>,
}

#[derive(Debug)]
pub enum SourceTableError {
    SourceCreate(SourceError),
    Io(std::io::Error),
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
}

impl Default for SourceTable {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SourceError> for SourceTableError {
    fn from(e: SourceError) -> Self {
        Self::SourceCreate(e)
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
