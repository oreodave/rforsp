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

/// Executable body, composed of a sequence of executable [`Block`]s and a
/// layout of bindings within that block.
///
/// NOTE: [`BlockId`] 0 is the entry block for any body.
#[derive(Debug, Default)]
pub struct Body {
    /// Vector origin, or `None` for the module entry body.
    pub origin: Option<SyntaxId>,
    /// Blocks indexed by body-local [`BlockId`].
    pub blocks: Table<BlockId, Block>,
    /// Local bindings.
    pub locals: IdCounter<LocalId>,
    /// Captured bindings.
    captures: Table<CaptureId, CaptureSource>,
}

impl Body {
    /// Construct a new [`Body`] with the given [`SyntaxId`] origin.
    fn new(origin: SyntaxId) -> Self {
        Self {
            origin: Some(origin),
            ..Self::default()
        }
    }

    /// Get the Captures within this Body.
    pub fn captures(&self) -> impl ExactSizeIterator<Item = CaptureSource> {
        self.captures.iter().map(|(_, y)| *y)
    }

    /// Add a capture to this [`Body`]s binding layout, returning [`CaptureId`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_assigns_dense_slots() {
        let mut body = Body::default();

        assert_eq!(body.locals.len(), 0);
        assert_eq!(body.captures().len(), 0);

        let first_local = body.locals.allocate();
        let second_local = body.locals.allocate();
        let first_capture = body.add_capture(CaptureSource::Local(first_local));
        let second_capture =
            body.add_capture(CaptureSource::Captured(first_capture));

        assert_ne!(first_local, second_local);
        assert_ne!(first_capture, second_capture);
        assert_eq!(body.locals.len(), 2);
        let captures: Vec<CaptureSource> = body.captures().collect();
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
        let mut body = Body::default();
        let local = body.locals.allocate();
        let source = CaptureSource::Local(local);

        let first = body.add_capture(source);
        let repeated = body.add_capture(source);

        assert_eq!(first, repeated);
        let captures: Vec<CaptureSource> = body.captures().collect();
        assert_eq!(captures, [source]);
    }
}
