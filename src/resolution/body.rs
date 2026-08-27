//! Body analysis data.

use crate::u32_index;

/// ID for a local within a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct LocalId(u32);

/// ID for a capture within a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CaptureId(u32);

/// Source for a value captured by a body.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CaptureSource {
    /// Capture a local within the enclosing body.
    Local(LocalId),
    /// Capture another capture within the enclosing body.
    Captured(CaptureId),
}

/// Layout of a body after analysis.
pub struct BodyLayout {
    /// Captures for this Body, indexed by [`CaptureId`].
    captures: Vec<CaptureSource>,
    /// Number of locals for this Body.
    local_count: usize,
}

impl BodyLayout {
    /// Construct new [`BodyLayout`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            captures: Vec::new(),
            local_count: 0,
        }
    }

    /// Add a new local to this body layout, returning [`LocalId`].
    #[must_use]
    pub const fn add_local(&mut self) -> LocalId {
        let id = LocalId(u32_index(self.local_count));
        self.local_count += 1;
        id
    }

    /// Add a capture to this body layout, returning [`CaptureId`]
    ///
    /// If the given `source` is already within the capture list then return the
    /// [`CaptureId`] for it, otherwise mint a new one.
    ///
    /// # Panics
    /// - if number of captures exceeds `u32::MAX`.
    #[must_use]
    pub fn add_capture(&mut self, source: CaptureSource) -> CaptureId {
        // TODO(oreo)[2026-08-28 00:00]: Potential backwards lookup HashMap like
        // interner.
        if let Some((id, _)) = self
            .captures
            .iter()
            .enumerate()
            .find(|(_, other)| source == **other)
        {
            CaptureId(u32_index(id))
        } else {
            let id = CaptureId(u32_index(self.captures.len()));
            self.captures.push(source);
            id
        }
    }

    /// Get a count of the number of locals in this Body.
    #[must_use]
    pub const fn local_count(&self) -> usize {
        self.local_count
    }

    /// Get the Captures within this Body.
    #[must_use]
    pub fn captures(&self) -> &[CaptureSource] {
        &self.captures
    }
}

impl Default for BodyLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_assigns_dense_slots() {
        let mut layout = BodyLayout::new();

        assert_eq!(layout.local_count(), 0);
        assert!(layout.captures().is_empty());

        let first_local = layout.add_local();
        let second_local = layout.add_local();
        let first_capture =
            layout.add_capture(CaptureSource::Local(first_local));
        let second_capture =
            layout.add_capture(CaptureSource::Captured(first_capture));

        assert_ne!(first_local, second_local);
        assert_ne!(first_capture, second_capture);
        assert_eq!(layout.local_count(), 2);
        assert_eq!(
            layout.captures(),
            [
                CaptureSource::Local(first_local),
                CaptureSource::Captured(first_capture),
            ]
        );
    }

    #[test]
    fn repeated_capture_sources_reuse_their_slot() {
        let mut layout = BodyLayout::new();
        let local = layout.add_local();
        let source = CaptureSource::Local(local);

        let first = layout.add_capture(source);
        let repeated = layout.add_capture(source);

        assert_eq!(first, repeated);
        assert_eq!(layout.captures(), [source]);
    }
}
