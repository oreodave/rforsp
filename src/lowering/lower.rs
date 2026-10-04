//! Main lowering routine.

use crate::{lowering::Module, parser::HirForm, resolution::ResolutionResult};

/// Lower a complete sequence of [`HirForm`]s with a given [`ResolutionResult`].
///
/// # Panics
/// - If an empty sequence of forms has a non-trivial entry
///   [`crate::resolution::BodyLayout`] within the given [`ResolutionResult`].
#[must_use]
#[expect(
    clippy::todo,
    reason = "Lowering implementation follows the IR sketch"
)]
pub fn lower(forms: &[HirForm], resolution: ResolutionResult) -> Module {
    if forms.is_empty() {
        assert_eq!(
            resolution.entry.local_count(),
            0,
            "Invariant: no locals on empty parse tree."
        );
        assert_eq!(
            resolution.entry.captures().len(),
            0,
            "Invariant: no captures on empty parse tree."
        );
        return Module::new(resolution.entry);
    }

    todo!("Implement")
}

#[cfg(test)]
mod tests {
    use crate::{
        context::Compilation,
        lowering::{TerminatorKind, lower},
        resolution::{BodyLayout, CaptureSource, ResolutionResult, resolve},
    };

    /// Resolve an empty program through the normal phase interface.
    fn resolve_empty() -> ResolutionResult {
        let compilation = Compilation::new();
        let (resolution, diagnostics) =
            resolve(&[], &compilation.variables, &compilation.primitives);
        assert!(
            diagnostics.items().is_empty(),
            "empty program resolves without diagnostics"
        );
        resolution
    }

    #[test]
    fn empty_program_has_one_completed_entry_body() {
        let module = lower(&[], resolve_empty());

        assert_eq!(module.bodies.len(), 1, "empty program has one entry body");
        assert!(module.data.is_empty(), "empty program owns no data");
        let body = &module.bodies[0];
        assert!(body.origin.is_none(), "entry body has no vector origin");
        assert_eq!(body.layout.local_count(), 0, "entry has no locals");
        assert!(body.layout.captures().is_empty(), "entry has no captures");
        assert_eq!(body.blocks.len(), 1, "entry has one block");
        let block = &body.blocks[0];
        assert!(block.instructions.is_empty(), "entry has no instructions");
        assert!(block.origins.is_empty(), "entry has no instruction origins");
        assert!(block.terminator.origin.is_none(), "body end is generated");
        assert!(
            matches!(block.terminator.kind, TerminatorKind::End),
            "empty entry completes execution"
        );
    }

    #[test]
    #[should_panic(expected = "Invariant: no locals on empty parse tree.")]
    fn empty_program_rejects_local_slots() {
        let mut resolution = resolve_empty();
        let _local = resolution.entry.add_local();
        let _module = lower(&[], resolution);
    }

    #[test]
    #[should_panic(expected = "Invariant: no captures on empty parse tree.")]
    fn empty_program_rejects_captures() {
        let mut resolution = resolve_empty();
        let mut enclosing = BodyLayout::new();
        let local = enclosing.add_local();
        let _capture =
            resolution.entry.add_capture(CaptureSource::Local(local));
        let _module = lower(&[], resolution);
    }
}
