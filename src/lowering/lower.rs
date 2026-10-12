//! Main lowering algorithm.

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    lowering::{
        Block, BlockId, Body, BodyId, Datum, DatumId, Instruction,
        LoweringError, LoweringErrorKind, Module, env::Environment,
    },
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
    primitives: &PrimitiveRegistry,
    variables: &RuntimeVariableRegistry,
) -> (Option<Module>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let lowerer = Lowerer::new(forms, &mut diagnostics, primitives, variables);
    let module = lowerer.complete();

    (module, diagnostics)
}

/// Traversal work that needs to be completed in order to lower.
enum Work<'forms> {
    /// A body of forms to lower.
    Body(&'forms [HirForm]),
    /// Sentinel to finish a complete [`Body`][crate::lowering::Body] of forms.
    FinishBody {
        /// Expected [`BodyId`] that this is closing.
        expected_body: BodyId,
        /// Previous [`BlockId`] being processed.
        prev_block: Option<BlockId>,
    },
}

/// Lowerer state machine.
struct Lowerer<'diags, 'forms> {
    /// [`Module`] being built.
    module: Module,
    /// [`Diagnostics`] for [`Lowerer`].
    diagnostics: &'diags mut Diagnostics,
    /// Lexical environment helper.
    env: Environment,
    /// [`Work`] to be completed.
    work: Vec<Work<'forms>>,
    /// Current [`BlockId`], within `body`.
    block: BlockId,
}

impl<'diags, 'forms> Lowerer<'diags, 'forms> {
    /// Construct a new [`Lowerer`].
    fn new(
        forms: &'forms [HirForm],
        diagnostics: &'diags mut Diagnostics,
        primitives: &PrimitiveRegistry,
        variables: &RuntimeVariableRegistry,
    ) -> Self {
        Self {
            module: Module::new(),
            env: Environment::new(variables, primitives),
            diagnostics,
            work: vec![
                Work::FinishBody {
                    expected_body: BodyId::default(),
                    prev_block: None,
                },
                Work::Body(forms),
            ],
            block: BlockId::default(),
        }
    }

    /// Lower the initial set of [`HirForm`]s given, by completing all movements
    /// of the [`Work`] state machine.
    ///
    /// Return Some([`Module`]) if and only if no errors arise during this
    /// process.
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
                Work::FinishBody {
                    expected_body,
                    prev_block,
                } => {
                    debug_assert_eq!(
                        expected_body,
                        self.env.current_body(),
                        "Expected alignment on current body"
                    );
                    let _ = self.env.close_frame();
                    if let Some(block) = prev_block {
                        self.block = block;
                    }
                }
            }
        }

        (!self.diagnostics.has_errors()).then_some(self.module)
    }

    /// Lower a singular [`HirForm`], adjusting the [`Module`] as required.
    fn lower_form(&mut self, form: &'forms HirForm) {
        match form {
            // Numbers, quoted forms, and lists, all lower to compile time data
            // pushes.
            HirForm {
                id,
                kind: HirKind::Int(n),
            } => {
                let datum = self.module.data.push(Datum::Int(*n));
                self.current_block()
                    .push_instruction(Instruction::PushData(datum), *id);
            }

            HirForm {
                id,
                kind: HirKind::Quote(subform),
            } => {
                let datum = lower_datum(&mut self.module, subform);
                self.current_block()
                    .push_instruction(Instruction::PushData(datum), *id);
            }

            HirForm {
                id,
                kind: HirKind::List(_),
            } => {
                let datum = lower_datum(&mut self.module, form);
                self.current_block()
                    .push_instruction(Instruction::PushData(datum), *id);
            }

            HirForm {
                id,
                kind: HirKind::Bind(sym),
            } => {
                // Adjust the current body's layout.
                let local = self.current_body().locals.allocate();
                // Inform lexical scope handler.
                self.env.bind(*sym, local);
                // Push instruction.
                self.current_block()
                    .push_instruction(Instruction::Bind(local), *id);
            }

            HirForm {
                id,
                kind: kind @ (HirKind::Load(sym) | HirKind::Call(sym)),
            } => {
                // Resolve the symbol.
                let Some(target) = self.env.resolve(*sym, &mut self.module)
                else {
                    self.error(*id, LoweringErrorKind::UnresolvedSymbol);
                    return;
                };

                // Construct instruction based on kind.
                let inst = match kind {
                    HirKind::Load(_) => Instruction::Load(target),
                    HirKind::Call(_) => Instruction::Call(target),
                    _ => unreachable!(
                        "outer pattern restricts kind to Load or Call"
                    ),
                };
                // Push instruction.
                self.current_block().push_instruction(inst, *id);
            }

            _ => todo!("implement"),
        }
    }

    /// Construct new error diagnostic of `kind` at `origin`.
    fn error(&mut self, origin: SyntaxId, kind: LoweringErrorKind) {
        self.diagnostics
            .push(Diagnostic::from(LoweringError { origin, kind }));
    }

    /// Retrieve the current [`Body`] that is being lowered.
    fn current_body(&mut self) -> &mut Body {
        self.module.bodies.get_mut(self.env.current_body())
    }

    /// Retrieve the current [`Block`] that is being lowered.
    fn current_block(&mut self) -> &mut Block {
        self.module
            .bodies
            .get_mut(self.env.current_body())
            .blocks
            .get_mut(self.block)
    }
}

/// Traversal work regarding [`HirForm`]s in Datum position.
enum DatumWork<'forms> {
    /// A singular form to make into data.
    Form(&'forms HirForm),
    /// Finish lowering the subform of a [`HirKind::Quote`].
    FinishQuote,
    /// Finish lowering a [`HirKind::Vector`] of some finite number of forms.
    FinishVector(usize),
    /// Finish lowering a [`HirKind::List`] of some finite number of forms.
    FinishList(usize),
}

/// Lower a [`HirForm`] in datum position within a [`Module`], returning the
/// associated [`DatumId`].
fn lower_datum<'forms>(module: &mut Module, form: &'forms HirForm) -> DatumId {
    let mut work_stack: Vec<DatumWork<'forms>> = vec![DatumWork::Form(form)];
    let mut results: Vec<DatumId> = Vec::new();

    // Invariant: |results| = 1 when the loop terminates.
    //
    // if `form` is singular, this is obviously true - see DatumWork::Form.
    // Otherwise, it's some container - quote, vector, list.
    //
    // But multiple forms are pushed onto `results` if and only if they are part
    // of some container (Vector, List, Quote).  Once those forms are lowered,
    // they are consumed by the `DatumWork::Finish<variant>`, leaving only a
    // single `DatumId` in `results`.
    while let Some(work) = work_stack.pop() {
        match work {
            DatumWork::Form(f) => match f {
                // This can be translated into a Datum directly.
                HirForm {
                    kind: HirKind::Int(n),
                    ..
                } => {
                    let datum = module.data.push(Datum::Int(*n));
                    results.push(datum);
                }

                // This can be translated into a Datum directly.
                HirForm {
                    kind: HirKind::Call(sym),
                    ..
                } => {
                    let datum = module.data.push(Datum::Atom(*sym));
                    results.push(datum);
                }

                // This requires lowering a subform.
                HirForm {
                    kind: HirKind::Quote(subform),
                    ..
                } => work_stack
                    .extend([DatumWork::FinishQuote, DatumWork::Form(subform)]),

                // This requires lowering subforms.
                HirForm {
                    kind: HirKind::List(xs),
                    ..
                } => work_stack.extend(
                    xs.iter()
                        .map(DatumWork::Form)
                        .chain(std::iter::once(DatumWork::FinishList(xs.len())))
                        .rev(),
                ),

                // This requires lowering subforms.
                HirForm {
                    kind: HirKind::Vector(xs),
                    ..
                } => work_stack.extend(
                    xs.iter()
                        .map(DatumWork::Form)
                        .chain(std::iter::once(DatumWork::FinishVector(
                            xs.len(),
                        )))
                        .rev(),
                ),

                // Invalid forms to be in Datum position, should have been
                // caught by the parser.
                HirForm {
                    kind: HirKind::Bind(_) | HirKind::Load(_),
                    ..
                } => {
                    unreachable!(
                        "HirForm's of kind `HirKind::Bind(_)` or `HirKind::Load(_)` are invalid in Datum position."
                    );
                }
            },

            // Quote finished lowering.
            DatumWork::FinishQuote => {
                let Some(subdatum) = results.pop() else {
                    unreachable!(
                        "FinishQuote on WorkStack => subform is lowered."
                    );
                };
                let datum = module.data.push(Datum::Quote(subdatum));
                results.push(datum);
            }

            // Vector finished lowering.
            DatumWork::FinishVector(size) => {
                let items = results.split_off(results.len() - size);
                let datum = module.data.push(Datum::Vector(items));
                results.push(datum);
            }

            // List finished lowering.
            DatumWork::FinishList(size) => {
                let items = results.split_off(results.len() - size);
                let datum = module.data.push(Datum::List(items));
                results.push(datum);
            }
        }
    }

    assert_eq!(results.len(), 1, "Expected |results|=1.");
    results[0]
}

#[cfg(test)]
mod tests {
    use crate::{
        context::Compilation,
        diagnostics::{Class, Diagnostics, Site},
        interner::SymId,
        lowering::{
            BlockId, BodyId, Datum, DatumId, Instruction, Module, Target,
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

    #[test]
    fn straight_lexical_binding_load_and_call_share_a_local() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("x");
        let forms = [
            fixture.form(HirKind::Int(1)),
            fixture.form(HirKind::Bind(x)),
            fixture.form(HirKind::Load(x)),
            fixture.form(HirKind::Call(x)),
        ];

        let (module, diagnostics) = fixture.lower(&forms);
        assert!(diagnostics.items().is_empty());
        let module = module.expect("resolved lexical references lower");
        assert_eq!(module.data.len(), 1);

        let entry = module.bodies.get(BodyId::default());
        assert_eq!(entry.locals.len(), 1);
        assert_eq!(entry.captures().len(), 0);
        let block = entry.blocks.get(BlockId::default());
        assert_eq!(block.instructions.len(), 4);
        let datum = match block.instructions[0] {
            Instruction::PushData(datum) => datum,
            ref other => panic!("expected data push, got {other:?}"),
        };
        assert!(matches!(module.data.get(datum), Datum::Int(1)));
        let local = match block.instructions[1] {
            Instruction::Bind(local) => local,
            ref other => panic!("expected bind, got {other:?}"),
        };
        assert!(
            matches!(block.instructions[2], Instruction::Load(Target::Local(got)) if got == local)
        );
        assert!(
            matches!(block.instructions[3], Instruction::Call(Target::Local(got)) if got == local)
        );
        assert_eq!(block.origins, forms.map(|form| form.id));
        assert_eq!(block.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn unresolved_lexical_references_report_origins_and_withhold_module() {
        let mut fixture = Fixture::new();
        let load_sym = fixture.sym("missing-load");
        let call_sym = fixture.sym("missing-call");
        let forms = [
            fixture.form(HirKind::Load(load_sym)),
            fixture.form(HirKind::Call(call_sym)),
            fixture.form(HirKind::Int(1)),
        ];

        let (module, diagnostics) = fixture.lower(&forms);
        assert!(module.is_none(), "errors withhold the partial module");
        let items = diagnostics.items();
        assert_eq!(items.len(), 2, "lowering accumulates both errors");
        for (diagnostic, form) in items.iter().zip(&forms[..2]) {
            assert_eq!(diagnostic.class, Class::LoweringUnresolvedSymbol);
            assert_eq!(diagnostic.site, Site::Syntax(form.id));
            assert_eq!(diagnostic.message, "Unresolved symbol");
        }
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
