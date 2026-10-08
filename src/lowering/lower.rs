//! Main lowering algorithm.

use crate::{
    diagnostics::Diagnostics,
    lowering::{Block, BlockId, BodyId, Datum, Instruction, Module},
    parser::{HirForm, HirKind},
    runtime::{PrimitiveRegistry, RuntimeVariableRegistry},
    source::SyntaxId,
};

/// Lower the given [`HirForm`]s, returning an SIR [`Module`] if no errors
/// arise, otherwise [`None`].
///
/// This constructs its own [`Diagnostics`] which is always passed back to the
/// caller.
#[must_use]
pub fn lower(
    forms: &[HirForm],
    _primitives: &PrimitiveRegistry,
    _variables: &RuntimeVariableRegistry,
) -> (Option<Module>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let lowerer = Lowerer::new(forms, &mut diagnostics);
    let module = lowerer.complete();

    (module, diagnostics)
}

/// Traversal work that needs to be completed in order to lower.
enum Work<'forms> {
    /// A body of forms to lower.
    Body(&'forms [HirForm]),
    /// Sentinel to finish a complete [`Body`][crate::lowering::Body] of forms.
    FinishBody(Option<SyntaxId>),
}

/// Lowerer state machine.
struct Lowerer<'diags, 'forms> {
    /// [`Module`] being built.
    module: Module,
    /// [`Diagnostics`] for [`Lowerer`].
    diagnostics: &'diags mut Diagnostics,
    /// [`Work`] to be completed.
    work: Vec<Work<'forms>>,
    /// Current [`BodyId`] within the [`Module`] being constructed.
    body: BodyId,
    /// Current [`BlockId`], within `body`.
    block: BlockId,
}

impl<'diags, 'forms> Lowerer<'diags, 'forms> {
    /// Construct a new [`Lowerer`].
    fn new(
        forms: &'forms [HirForm],
        diagnostics: &'diags mut Diagnostics,
    ) -> Self {
        Self {
            module: Module::default(),
            diagnostics,
            work: vec![Work::FinishBody(None), Work::Body(forms)],
            block: BlockId::default(),
            body: BodyId::default(),
        }
    }

    /// Perform all actions of state machine and return Some([`Module`]) if and
    /// only if no errors arised during the work.
    fn complete(mut self) -> Option<Module> {
        while let Some(work) = self.work.pop() {
            match work {
                Work::Body(forms) => {
                    let Some((form, remaining)) = forms.split_first() else {
                        continue;
                    };
                    self.work.push(Work::Body(remaining));
                    self.lower_form(form);
                }
                Work::FinishBody(_) => {
                    // TODO(oreo)[2026-10-08 06:36]: Close up any environments,
                    // copy [`BodyLayout`] over to the current working [`Body`].
                }
            }
        }

        (!self.diagnostics.has_errors()).then_some(self.module)
    }

    /// Lower a singular [`HirForm`], adjusting the [`Module`] as required.
    fn lower_form(&mut self, form: &'forms HirForm) {
        match form {
            &HirForm {
                id,
                kind: HirKind::Int(n),
            } => {
                let datum = self.module.data.push(Datum::Int(n));
                let inst = Instruction::PushData(datum);
                self.current_block().push_instruction(inst, id);
            }
            _ => todo!("implement"),
        }
    }

    /// Retrieve the current [`Block`] that is being built.
    fn current_block(&mut self) -> &mut Block {
        self.module
            .bodies
            .get_mut(self.body)
            .blocks
            .get_mut(self.block)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::Compilation,
        diagnostics::Diagnostics,
        interner::SymId,
        lowering::{
            BlockId, BodyId, Datum, Instruction, Module, TerminatorKind,
        },
        parser::{HirForm, HirKind},
        source::{SourceId, Span, SyntaxId},
    };

    struct Fixture {
        compilation: Compilation,
        source: SourceId,
    }

    impl Fixture {
        fn new() -> Self {
            let mut compilation = Compilation::new();
            let source = compilation
                .table
                .add_source_raw("resolver-test", "x".into())
                .expect("test source should be valid");

            Self {
                compilation,
                source,
            }
        }

        fn id(&mut self) -> SyntaxId {
            self.compilation
                .table
                .add_origin(self.source, Span::new(0, 1))
        }

        fn sym(&mut self, name: &str) -> SymId {
            self.compilation.interner.intern(name)
        }

        fn lower(&self, forms: &[HirForm]) -> (Option<Module>, Diagnostics) {
            super::lower(
                forms,
                &self.compilation.primitives,
                &self.compilation.variables,
            )
        }
    }

    #[test]
    fn empty_program() {
        let fixture = Fixture::new();
        let (module, diagnostics) = fixture.lower(&[]);
        assert!(diagnostics.items().is_empty());

        let module = module.expect("empty program lowers successfully");
        assert!(module.data.is_empty());
        assert_eq!(module.bodies.len(), 1);

        let entry = module.bodies.get(BodyId::default());
        assert_eq!(entry.origin, None);
        assert_eq!(entry.layout.locals.len(), 0);
        assert_eq!(entry.layout.captures().len(), 0);
        assert_eq!(entry.blocks.len(), 1);

        let block = &entry.blocks.get(BlockId::default());
        assert_eq!(block.instructions.len(), 0);
        assert_eq!(block.origins, []);
        assert_eq!(block.terminator.origin, None);
        assert_eq!(block.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn program_of_ints() {
        let mut fixture = Fixture::new();
        let expected_data = [0, 1, 2, 3, 4];
        let forms = expected_data
            .map(HirKind::Int)
            .map(|k| HirForm::new(fixture.id(), k));

        let (module, diagnostics) = fixture.lower(&forms);
        assert!(diagnostics.items().is_empty());

        let module = module.expect("empty program lowers successfully");
        assert_eq!(module.bodies.len(), 1);
        assert_eq!(module.data.len(), expected_data.len());

        for (got, expected) in module
            .data
            .iter()
            .map(|(_, data)| data)
            .zip(expected_data.iter())
        {
            let Datum::Int(got) = got else {
                panic!("Expected Datum::Int, got {got:?}");
            };
            assert_eq!(got, expected);
        }

        let entry = module.bodies.get(BodyId::default());
        assert_eq!(entry.origin, None);
        assert_eq!(entry.layout.locals.len(), 0);
        assert_eq!(entry.layout.captures().len(), 0);
        assert_eq!(entry.blocks.len(), 1);

        let block = &entry.blocks.get(BlockId::default());
        // Each instruction pushes some data
        assert_eq!(block.instructions.len(), 5);
        for (inst, expected) in
            block.instructions.iter().zip(expected_data.iter())
        {
            let Instruction::PushData(got_datum) = inst else {
                panic!("Expected Instruction::PushData, got {inst:?}");
            };
            let got_datum = module.data.get(*got_datum);
            let &Datum::Int(got) = got_datum else {
                panic!("Expected Datum::Int, got {got_datum:?}");
            };
            assert_eq!(got, *expected);
        }
        assert_eq!(block.origins, forms.map(|form| form.id));
        assert_eq!(block.terminator.origin, None);
        assert_eq!(block.terminator.kind, TerminatorKind::End);
    }
}
