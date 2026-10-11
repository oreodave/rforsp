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

/// Traversal work regarding [`HirForm`]s in Datum position.
enum DatumWork<'forms> {
    /// A singular form to make into data.
    Form(&'forms HirForm),
    /// Finish making into data a [`HirKind`] quote.
    FinishQuote,
    /// Finish making into data a [`HirKind`] vector of some finite number of
    /// forms.
    FinishVector(usize),
    /// Finish making into data a [`HirKind`] list of some finite number of
    /// forms.
    FinishList(usize),
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
        // Initialise a module with the first, entry-level body, with a default
        // empty block.
        let module = {
            let mut module = Module::default();
            let entry_body = module.bodies.push(Body::default());
            let _entry_block = module
                .bodies
                .get_mut(entry_body)
                .blocks
                .push(Block::default());
            module
        };

        Self {
            module,
            diagnostics,
            work: vec![
                Work::FinishBody(None, BodyId::default()),
                Work::Body(forms),
            ],
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
            HirForm {
                id,
                kind: HirKind::Int(n),
            } => {
                let datum = self.module.data.push(Datum::Int(*n));
                let inst = Instruction::PushData(datum);
                self.current_block().push_instruction(inst, *id);
            }

            HirForm {
                id,
                kind: HirKind::Quote(subform),
            } => {
                let datum = self.lower_datum(subform);
                let inst = Instruction::PushData(datum);
                self.current_block().push_instruction(inst, *id);
            }

            HirForm {
                id,
                kind: HirKind::List(_),
            } => {
                let datum = self.lower_datum(form);
                let inst = Instruction::PushData(datum);
                self.current_block().push_instruction(inst, *id);
            }

            _ => todo!("implement"),
        }
    }

    /// Lower a form as a [`Datum`], returning its corresponding [`DatumId`]
    /// within the being-built [`Module`].
    fn lower_datum(&mut self, form: &'forms HirForm) -> DatumId {
        let mut work_stack: Vec<DatumWork<'forms>> =
            vec![DatumWork::Form(form)];
        let mut results: Vec<DatumId> = Vec::new();

        while let Some(work) = work_stack.pop() {
            match work {
                DatumWork::FinishQuote => {
                    let Some(subdatum) = results.pop() else {
                        unreachable!(
                            "There should be data on the results stack"
                        );
                    };
                    let datum = self.module.data.push(Datum::Quote(subdatum));
                    results.push(datum);
                }

                DatumWork::FinishVector(size) => {
                    let items = results.split_off(results.len() - size);
                    let datum = self.module.data.push(Datum::Vector(items));
                    results.push(datum);
                }

                DatumWork::FinishList(size) => {
                    let items = results.split_off(results.len() - size);
                    let datum = self.module.data.push(Datum::List(items));
                    results.push(datum);
                }

                DatumWork::Form(f) => match f {
                    HirForm {
                        kind: HirKind::Int(n),
                        ..
                    } => {
                        let datum = self.module.data.push(Datum::Int(*n));
                        results.push(datum);
                    }

                    HirForm {
                        kind: HirKind::Call(sym),
                        ..
                    } => {
                        let datum = self.module.data.push(Datum::Atom(*sym));
                        results.push(datum);
                    }

                    HirForm {
                        kind: HirKind::Quote(subform),
                        ..
                    } => work_stack.extend([
                        DatumWork::FinishQuote,
                        DatumWork::Form(subform),
                    ]),

                    HirForm {
                        kind: HirKind::List(xs),
                        ..
                    } => {
                        work_stack.push(DatumWork::FinishList(xs.len()));
                        work_stack.extend(xs.iter().rev().map(DatumWork::Form));
                    }

                    HirForm {
                        kind: HirKind::Vector(xs),
                        ..
                    } => {
                        work_stack.push(DatumWork::FinishVector(xs.len()));
                        work_stack.extend(xs.iter().rev().map(DatumWork::Form));
                    }

                    HirForm {
                        kind: HirKind::Bind(_) | HirKind::Load(_),
                        ..
                    } => {
                        unreachable!(
                            "Impossible for these [`HirForm`] variants to be here."
                        );
                    }
                },
            }
        }

        assert_eq!(results.len(), 1, "Expected |results|=1.");
        results[0]
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
            BlockId, BodyId, Datum, DatumId, Instruction, Module,
            TerminatorKind,
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

        /// Construct a form with a distinct source origin.
        fn form(&mut self, kind: HirKind) -> HirForm {
            HirForm::new(self.id(), kind)
        }

        /// Lower data forms and check their instruction origins and body shape.
        fn lower_data(&self, forms: &[HirForm]) -> (Module, Vec<DatumId>) {
            let (module, diagnostics) = self.lower(forms);
            assert!(
                diagnostics.items().is_empty(),
                "data produces no diagnostics"
            );
            let module = module.expect("data forms lower successfully");
            assert_eq!(
                module.bodies.len(),
                1,
                "datum vectors allocate no bodies"
            );
            let entry = module.bodies.get(BodyId::default());
            assert_eq!(entry.blocks.len(), 1, "data is straight-line code");
            let block = entry.blocks.get(BlockId::default());
            assert_eq!(
                block.instructions.len(),
                forms.len(),
                "one push per form"
            );
            assert_eq!(
                block.origins,
                forms.iter().map(|form| form.id).collect::<Vec<_>>(),
                "pushes retain top-level origins in source order"
            );
            assert_eq!(
                block.terminator.kind,
                TerminatorKind::End,
                "entry completes"
            );
            let ids = block
                .instructions
                .iter()
                .map(|inst| match inst {
                    Instruction::PushData(id) => *id,
                    other => panic!("expected data push, got {other:?}"),
                })
                .collect();
            (module, ids)
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
        assert_eq!(entry.locals.len(), 0);
        assert_eq!(entry.captures().len(), 0);
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
        assert_eq!(entry.locals.len(), 0);
        assert_eq!(entry.captures().len(), 0);
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

    /// Retrieve a list datum's children without depending on their ID numbering.
    fn list_items(module: &Module, id: DatumId) -> &[DatumId] {
        match module.data.get(id) {
            Datum::List(items) => items,
            other => panic!("expected list, got {other:?}"),
        }
    }

    /// Retrieve a vector datum's children without depending on their ID numbering.
    fn vector_items(module: &Module, id: DatumId) -> &[DatumId] {
        match module.data.get(id) {
            Datum::Vector(items) => items,
            other => panic!("expected vector, got {other:?}"),
        }
    }

    /// Retrieve the child of a retained quotation.
    fn quote_item(module: &Module, id: DatumId) -> DatumId {
        match module.data.get(id) {
            Datum::Quote(child) => *child,
            other => panic!("expected quotation, got {other:?}"),
        }
    }

    /// Check an integer leaf through its actual datum reference.
    fn assert_int(module: &Module, id: DatumId, expected: i64) {
        assert!(
            matches!(module.data.get(id), Datum::Int(got) if *got == expected),
            "expected integer {expected}, got {:?}",
            module.data.get(id)
        );
    }

    #[test]
    fn quoted_atom_does_not_resolve_its_name() {
        let mut fixture = Fixture::new();
        let symbol = fixture.sym("unbound-datum-name");
        let operand = fixture.form(HirKind::Call(symbol));
        let form = fixture.form(HirKind::Quote(Box::new(operand)));
        let (module, ids) = fixture.lower_data(&[form]);

        assert_eq!(
            module.data.len(),
            1,
            "outer quotation allocates no wrapper"
        );
        assert!(
            matches!(module.data.get(ids[0]), Datum::Atom(got) if *got == symbol),
            "quoted name becomes the same symbol atom"
        );
    }

    #[test]
    fn nested_quotes_retain_only_inner_wrappers() {
        let mut fixture = Fixture::new();
        let symbol = fixture.sym("nested-atom");
        let mut form = fixture.form(HirKind::Call(symbol));
        for _ in 0..3 {
            form = fixture.form(HirKind::Quote(Box::new(form)));
        }
        let (module, ids) = fixture.lower_data(&[form]);

        assert_eq!(module.data.len(), 3, "atom and two inner quotations");
        let inner_quote = quote_item(&module, ids[0]);
        let atom = quote_item(&module, inner_quote);
        assert!(
            matches!(module.data.get(atom), Datum::Atom(got) if *got == symbol),
            "nested quotation preserves its atom"
        );
    }

    #[test]
    fn mixed_containers_preserve_children_and_sibling_order() {
        let mut fixture = Fixture::new();
        let symbol = fixture.sym("list-atom");
        let one = fixture.form(HirKind::Int(1));
        let two = fixture.form(HirKind::Int(2));
        let three = fixture.form(HirKind::Int(3));
        let atom = fixture.form(HirKind::Call(symbol));
        let inner_list = fixture.form(HirKind::List(vec![three, atom]));
        let vector = fixture.form(HirKind::Vector(vec![two, inner_list]));
        let four = fixture.form(HirKind::Int(4));
        let list = fixture.form(HirKind::List(vec![one, vector, four]));
        let following = fixture.form(HirKind::Int(5));
        let (module, ids) = fixture.lower_data(&[list, following]);

        assert_eq!(
            module.data.len(),
            9,
            "five leaves, three containers, following integer"
        );
        let outer = list_items(&module, ids[0]);
        assert_eq!(outer.len(), 3, "outer list retains all siblings");
        assert_int(&module, outer[0], 1);
        assert_int(&module, outer[2], 4);
        let vector = vector_items(&module, outer[1]);
        assert_eq!(vector.len(), 2, "vector retains both children");
        assert_int(&module, vector[0], 2);
        let inner = list_items(&module, vector[1]);
        assert_eq!(inner.len(), 2, "inner list retains both children");
        assert_int(&module, inner[0], 3);
        assert!(
            matches!(module.data.get(inner[1]), Datum::Atom(got) if *got == symbol),
            "list names become atoms without resolution"
        );
        assert_int(&module, ids[1], 5);
    }

    #[test]
    fn empty_containers_survive_in_code_and_datum_positions() {
        let mut fixture = Fixture::new();
        let list = fixture.form(HirKind::List(vec![]));
        let vector = fixture.form(HirKind::Vector(vec![]));
        let quoted_vector = fixture.form(HirKind::Quote(Box::new(vector)));
        let inner_list = fixture.form(HirKind::List(vec![]));
        let quoted_list = fixture.form(HirKind::Quote(Box::new(inner_list)));
        let nested_vector = fixture.form(HirKind::Vector(vec![]));
        let nested_list = fixture.form(HirKind::List(vec![]));
        let containers =
            fixture.form(HirKind::List(vec![nested_vector, nested_list]));
        let (module, ids) =
            fixture.lower_data(&[list, quoted_vector, quoted_list, containers]);

        assert_eq!(module.data.len(), 6, "every container has one datum node");
        assert_eq!(list_items(&module, ids[0]), [], "empty executable list");
        assert_eq!(vector_items(&module, ids[1]), [], "empty quoted vector");
        assert_eq!(list_items(&module, ids[2]), [], "empty quoted list");
        let children = list_items(&module, ids[3]);
        assert_eq!(children.len(), 2, "empty children still occupy positions");
        assert_eq!(
            vector_items(&module, children[0]),
            [],
            "nested empty vector"
        );
        assert_eq!(list_items(&module, children[1]), [], "nested empty list");
    }

    #[test]
    fn deeply_nested_data_lowers_and_drops_iteratively() {
        let mut fixture = Fixture::new();
        let depth = 12_000;
        let mut form = fixture.form(HirKind::Int(-1));
        for level in 0..depth {
            let kind = match level % 3 {
                0 => HirKind::List(vec![form]),
                1 => HirKind::Vector(vec![form]),
                _ => HirKind::Quote(Box::new(form)),
            };
            form = fixture.form(kind);
        }
        let form = fixture.form(HirKind::Quote(Box::new(form)));
        let (module, ids) = fixture.lower_data(&[form]);

        assert_eq!(
            module.data.len(),
            depth + 1,
            "each retained wrapper has one node"
        );
        let mut id = ids[0];
        for level in (0..depth).rev() {
            id = match level % 3 {
                0 => {
                    let children = list_items(&module, id);
                    assert_eq!(children.len(), 1, "nested list has one child");
                    children[0]
                }
                1 => {
                    let children = vector_items(&module, id);
                    assert_eq!(
                        children.len(),
                        1,
                        "nested vector has one child"
                    );
                    children[0]
                }
                _ => quote_item(&module, id),
            };
        }
        assert_int(&module, id, -1);
        drop(module);
    }
}
