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
