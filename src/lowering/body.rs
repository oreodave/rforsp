//! Body - executable functions, composed of sequences of [`Block`]s and a
//! layout.

use crate::{
    define_id,
    helper::{IdCounter, Table},
    lowering::{Block, BlockId},
    source::SyntaxId,
};

define_id!(pub LocalId);
define_id!(pub CaptureId);

/// Source of some binding.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CaptureSource {
    /// Capture a local within the enclosing body.
    Local(LocalId),
    /// Capture another capture within the enclosing body.
    Captured(CaptureId),
}

/// The layout of bindings created and used within a [`Body`].
#[derive(Debug)]
pub struct BodyLayout {
    /// Local bindings.
    pub locals: IdCounter<LocalId>,
    /// Captured bindings.
    captures: Table<CaptureId, CaptureSource>,
}

impl BodyLayout {
    /// Construct new [`BodyLayout`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            captures: Table::new(),
            locals: IdCounter::new(),
        }
    }

    /// Get the Captures within this Body.
    pub fn captures(&self) -> impl ExactSizeIterator<Item = CaptureSource> {
        self.captures.iter().map(|(_, y)| *y)
    }

    /// Add a capture to this body layout, returning [`CaptureId`]
    ///
    /// If the given `source` is already within the capture list then return the
    /// [`CaptureId`] for it, otherwise mint a new one.
    ///
    /// # Panics
    /// - if number of captures exceeds `u32::MAX`.
    #[must_use]
    pub(crate) fn add_capture(&mut self, source: CaptureSource) -> CaptureId {
        // TODO(oreo)[2026-08-28 00:00]: Potential backwards lookup HashMap,
        // like interner.
        if let Some(id) = self
            .captures
            .iter()
            .find_map(|(id, other)| (source == *other).then_some(id))
        {
            id
        } else {
            self.captures.push(source)
        }
    }
}

impl Default for BodyLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Executable body with a [`BodyLayout`] and sequence of executable [`Block`]s.
///
/// NOTE: [`BlockId`] 0 is the entry block for any body.
#[derive(Debug)]
pub struct Body {
    /// Vector origin, or `None` for the module entry body.
    pub origin: Option<SyntaxId>,
    /// Local slots and ordered capture sources from resolution.
    pub layout: BodyLayout,
    /// Blocks indexed by body-local [`BlockId`].
    pub blocks: Table<BlockId, Block>,
}

impl Body {
    /// Construct a new [`Body`] with the given [`SyntaxId`] origin.
    fn new(origin: SyntaxId) -> Self {
        let mut blocks = Table::new();
        let _ = blocks.push(Block::default());
        Self {
            origin: Some(origin),
            layout: BodyLayout::default(),
            blocks,
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        let mut blocks = Table::new();
        let _ = blocks.push(Block::default());
        Self {
            origin: None,
            layout: BodyLayout::default(),
            blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_assigns_dense_slots() {
        let mut layout = BodyLayout::new();

        assert_eq!(layout.locals.len(), 0);
        assert_eq!(layout.captures().len(), 0);

        let first_local = layout.locals.allocate();
        let second_local = layout.locals.allocate();
        let first_capture =
            layout.add_capture(CaptureSource::Local(first_local));
        let second_capture =
            layout.add_capture(CaptureSource::Captured(first_capture));

        assert_ne!(first_local, second_local);
        assert_ne!(first_capture, second_capture);
        assert_eq!(layout.locals.len(), 2);
        let captures: Vec<CaptureSource> = layout.captures().collect();
        assert_eq!(
            captures,
            [
                CaptureSource::Local(first_local),
                CaptureSource::Captured(first_capture),
            ]
        );
    }

    #[test]
    fn repeated_capture_sources_reuse_their_slot() {
        let mut layout = BodyLayout::new();
        let local = layout.locals.allocate();
        let source = CaptureSource::Local(local);

        let first = layout.add_capture(source);
        let repeated = layout.add_capture(source);

        assert_eq!(first, repeated);
        let captures: Vec<CaptureSource> = layout.captures().collect();
        assert_eq!(captures, [source]);
    }
}
