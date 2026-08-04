use crate::source::{Position, Source, SourceError, Span};

/******************************************************************************
 * Structures                                                                 *
 ******************************************************************************/
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SourceId(u32);

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct SyntaxId(u32);

#[derive(Debug, Copy, Clone)]
pub struct SyntaxOrigin {
    pub source: SourceId,
    pub span: Span,
}

#[derive(Debug)]
pub struct SourceTable {
    sources: Vec<Source>,
    origins: Vec<SyntaxOrigin>,
}

#[derive(Debug)]
pub enum SourceTableError {
    SourceCreate(SourceError),
    Io(std::io::Error),
}

#[derive(Debug, Copy, Clone)]
pub struct Location<'a> {
    pub file: &'a str,
    pub start: Position,
    pub end: Position,
}

impl SourceTable {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            origins: Vec::new(),
        }
    }

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
    /// - if `self.sources.len()` > `u32::MAX`.
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

    /// Add a new `SyntaxOrigin` given the `id` and `span`.
    ///
    /// # Panics
    /// - if `span` is not valid for the `Source` related to `id`.
    /// - if `self.origins.len()` > `u32::MAX`.
    pub fn add_origin(&mut self, id: SourceId, span: Span) -> SyntaxId {
        let source = self.get_source(id);
        assert!(source.valid_span(span));
        let syn_id = SyntaxId(
            u32::try_from(self.origins.len()).expect("|origins| > u32::MAX"),
        );
        self.origins.push(SyntaxOrigin { source: id, span });
        syn_id
    }

    /// Get the `SyntaxOrigin` associated to the given `id`.
    ///
    /// # Panics
    /// - if `id` is out of bounds of `self.origins`.
    #[must_use]
    pub fn get_origin(&self, id: SyntaxId) -> &SyntaxOrigin {
        let id = id.0 as usize;
        assert!(id < self.origins.len());
        &self.origins[id]
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
            SourceTableError::SourceCreate(e) => {
                write!(f, "{e}")
            }
            SourceTableError::Io(e) => {
                write!(f, "{e}")
            }
        }
    }
}
