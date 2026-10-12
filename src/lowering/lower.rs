//! Main lowering algorithm.

use crate::{
    diagnostics::{Class, Diagnostic, Diagnostics, Site, conv::ice},
    interner::{SYM_IF, SYM_REC},
    lowering::{
        Block, BlockId, Body, BodyId, ClosureMode, Datum, DatumId, Instruction,
        LoweringError, LoweringErrorKind, Module, Terminator, TerminatorKind,
        env::Environment,
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
                Work::Body {
                    forms,
                    exit: TerminatorKind::End,
                },
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
                Work::Body { forms, exit } => {
                    let forms = match self.lower_recognised_prefix(forms, exit)
                    {
                        Recognition::Unrecognised => forms,
                        Recognition::Scheduled => continue,
                        Recognition::Continue(xs) => {
                            self.work.push(Work::Body { forms: xs, exit });
                            continue;
                        }
                    };

                    let Some((form, remaining)) = forms.split_first() else {
                        // no more forms to lower => set the current block's
                        // terminator.
                        self.terminate_block(Terminator {
                            origin: None,
                            kind: exit,
                        });
                        continue;
                    };

                    self.work.push(Work::Body {
                        forms: remaining,
                        exit,
                    });
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

                Work::Branch { block, forms, exit } => {
                    // Initialise lowerer state machine to lower this branch.
                    self.block = block;
                    self.env.enter_scope();
                    // Push actual work onto the stack.
                    self.work.extend([
                        Work::FinishBranch,
                        Work::Body { forms, exit },
                    ]);
                }

                Work::FinishBranch => {
                    self.env.leave_scope();
                }

                Work::ContinueAt { block, forms, exit } => {
                    // Setup state machine to continue lowering `forms` in the
                    // given `block`.
                    self.block = block;
                    self.work.push(Work::Body { forms, exit });
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
                self.emit_inst(Instruction::PushData(datum), *id);
            }

            HirForm {
                id,
                kind: HirKind::Quote(subform),
            } => {
                self.lower_push_data(subform, *id);
            }

            HirForm {
                id,
                kind: HirKind::List(_),
            } => {
                self.lower_push_data(form, *id);
            }

            HirForm {
                id,
                kind: HirKind::Bind(sym),
            } => {
                // Adjust the current body's layout.
                let local = self.current_body().locals.allocate();
                // Inform lexical scope handler.
                self.env.bind(*sym, local);

                self.emit_inst(Instruction::Bind(local), *id);
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
                let inst = if matches!(kind, HirKind::Load(_)) {
                    Instruction::Load(target)
                } else {
                    Instruction::Call(target)
                };

                self.emit_inst(inst, *id);
            }

            // A vector in executable position makes a closure.
            HirForm {
                id,
                kind: HirKind::Vector(xs),
            } => {
                self.lower_make_closure(*id, xs, ClosureMode::Normal);
            }
        }
    }

    /// Attempt to recognise a prefix of `forms`.
    fn lower_recognised_prefix(
        &mut self,
        forms: &'forms [HirForm],
        exit: TerminatorKind,
    ) -> Recognition<'forms> {
        let is_prim_call = |index, sym| {
            forms.get(index).is_some_and(|form: &HirForm| {
                matches!(form.kind, HirKind::Call(item) if item == sym)
                    && self.env.is_primordial(sym)
            })
        };

        if is_prim_call(1, SYM_REC) {
            self.lower_recognised_recursion(&forms[0..2], &forms[2..], exit)
        } else if is_prim_call(2, SYM_IF) {
            self.lower_recognised_conditional(&forms[0..3], &forms[3..], exit)
        } else {
            Recognition::Unrecognised
        }
    }

    /// Attempt to lower a potentially recognised recursive closure.
    fn lower_recognised_recursion(
        &mut self,
        prefix: &'forms [HirForm],
        suffix: &'forms [HirForm],
        exit: TerminatorKind,
    ) -> Recognition<'forms> {
        // This function is called <=> prefix[1] == CALL(SYM_REC).
        match &prefix[0].kind {
            // Ideal case - the form is a vector in executable position.
            HirKind::Vector(body) => {
                // Schedule the suffix for after we process this vector.
                self.work.push(Work::Body {
                    forms: suffix,
                    exit,
                });
                self.lower_make_closure(
                    prefix[0].id,
                    body,
                    ClosureMode::Recursive,
                );
                Recognition::Scheduled
            }

            // A `Load`, `Call`, or `Bind` cannot be recognised recursion, since
            // we can't be sure what data is on the stack following it.
            HirKind::Load(_) | HirKind::Call(_) | HirKind::Bind(_) => {
                Recognition::Unrecognised
            }

            // Otherwise this is data - certainly NOT allowed for the recursion
            // operator - so we need to record an error.  But we should also
            // make sure to pass back the suffix to the caller.
            HirKind::Int(_) | HirKind::Quote(_) | HirKind::List(_) => {
                self.error(
                    prefix[0].id,
                    LoweringErrorKind::RecursionDataOperand,
                );
                Recognition::Continue(suffix)
            }
        }
    }

    /// Attempt to lower a potentially recognised conditional.
    fn lower_recognised_conditional(
        &mut self,
        prefix: &'forms [HirForm],
        suffix: &'forms [HirForm],
        exit: TerminatorKind,
    ) -> Recognition<'forms> {
        for form in &prefix[..2] {
            match form.kind {
                // Invalid forms for a recognised conditional.
                HirKind::Bind(_) | HirKind::Load(_) | HirKind::Call(_) => {
                    return Recognition::Unrecognised;
                }
                HirKind::Int(_)
                | HirKind::Quote(_)
                | HirKind::List(_)
                | HirKind::Vector(_) => {}
            }
        }

        // Mint blocks for the branches and a potential continuation block.
        let b_then = self.new_block();
        let b_else = self.new_block();
        let b_after = (!suffix.is_empty()).then(|| self.new_block());

        // Determine the type of exit required for the branches.
        let branch_exit = b_after.map_or(exit, TerminatorKind::Jump);

        // Add work for the continuation block if required.
        if let Some(block) = b_after {
            self.work.push(Work::ContinueAt {
                block,
                forms: suffix,
                exit,
            });
        }

        // Add work for the branches.
        for (form, block) in prefix[0..2].iter().zip([b_then, b_else]).rev() {
            let forms = match &form.kind {
                HirKind::Vector(body) => body,
                _ => std::slice::from_ref(form),
            };
            self.work.push(Work::Branch {
                block,
                forms,
                exit: branch_exit,
            });
        }

        // Terminate the current block with a `Branch` into the new blocks.
        self.terminate_block(Terminator {
            origin: Some(prefix[2].id),
            kind: TerminatorKind::Branch {
                then_block: b_then,
                else_block: b_else,
            },
        });

        Recognition::Scheduled
    }

    /// Lower a known instance of closure construction.
    ///
    /// This involves constructing a new [`Body`] within the [`Module`], pushing
    /// [`Instruction::MakeClosure`] within the block which first instantiated
    /// the closure with the given `mode`, as well as setting up [`Lowerer`]
    /// state machine to lower the `forms` within that closure.
    fn lower_make_closure(
        &mut self,
        origin: SyntaxId,
        forms: &'forms [HirForm],
        mode: ClosureMode,
    ) {
        // Mint new BodyId.
        let body_id = self.module.bodies.push(Body::new(Some(origin)));

        // Setup the closure creation instruction within the current scope.
        self.emit_inst(Instruction::MakeClosure(body_id, mode), origin);

        // Push work onto the stack regarding the actual forms of this closure.
        self.work.extend([
            Work::FinishBody {
                expected_body: body_id,
                prev_block: Some(self.block),
            },
            Work::Body {
                forms,
                exit: TerminatorKind::End,
            },
        ]);

        // Setup environment and block relative to the new [`Body`].
        self.env.open_frame(body_id);
        self.block = BlockId::default();
    }

    /// Construct a new [`Block`] within the current active [`Body`], returning
    /// the associated [`BlockId`].
    fn new_block(&mut self) -> BlockId {
        self.current_body().blocks.push(Block::default())
    }

    /// Terminate the current [`Block`] using the given [`Terminator`].
    fn terminate_block(&mut self, terminator: Terminator) {
        self.current_block().terminator = terminator;
    }

    /// Emit an [`Instruction`] within the active [`Block`].
    fn emit_inst(&mut self, inst: Instruction, origin: SyntaxId) {
        self.current_block().push_instruction(inst, origin);
    }

    /// Emit a datum push or report an invalid datum form from HIR.
    fn lower_push_data(&mut self, form: &HirForm, origin: SyntaxId) {
        match lower_datum(&mut self.module, form) {
            Ok(datum) => self.emit_inst(Instruction::PushData(datum), origin),
            Err(invalid_origin) => self.diagnostics.push(ice(
                Class::ICEInvalidHIRDatum,
                Site::Syntax(invalid_origin),
                "lowering received a bind or load in datum position",
            )),
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

/// Result of running recongition on some prefix of forms.
enum Recognition<'forms> {
    /// Prefix of forms were unrecognised.
    Unrecognised,
    /// Prefix of forms were recognised and work has been scheduled.
    Scheduled,
    /// Prefix of forms were recognised and work needs to be continued by
    /// caller.
    Continue(&'forms [HirForm]),
}

/// Traversal work that needs to be completed in order to lower.
#[derive(Debug)]
enum Work<'forms> {
    /// A body of forms to lower.
    Body {
        /// [`HirForm`]s to lower.
        forms: &'forms [HirForm],
        /// Terminator for this body.
        exit: TerminatorKind,
    },

    /// Sentinel to finish a complete [`Body`] of forms.
    FinishBody {
        /// Expected [`BodyId`] that this is closing.
        expected_body: BodyId,
        /// Previous [`BlockId`] being processed.
        prev_block: Option<BlockId>,
    },

    /// A Branch that needs lowering.
    Branch {
        /// Block that this Branch lowers into.
        block: BlockId,
        /// [`HirForm`]s belonging to this Branch.
        forms: &'forms [HirForm],
        /// Terminator for this Branch.
        exit: TerminatorKind,
    },

    /// Sentinel to clean up following the lowering of a Branch.
    FinishBranch,

    /// Continuation following some [`Work`] that restores [`Block`],
    /// [`HirForm`]s, and the [`TerminatorKind`] before the [`Work`] started.
    ContinueAt {
        /// Block that `forms` should be lowered into.
        block: BlockId,
        /// [`HirForm`]s to lower.
        forms: &'forms [HirForm],
        /// Inherited exit of the overall block.
        exit: TerminatorKind,
    },
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
fn lower_datum<'forms>(
    module: &mut Module,
    form: &'forms HirForm,
) -> Result<DatumId, SyntaxId> {
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
                    id,
                    kind: HirKind::Bind(_) | HirKind::Load(_),
                } => return Err(*id),
            },

            // Quote finished lowering.
            DatumWork::FinishQuote => {
                let subdatum = results
                    .pop()
                    .expect("a quote finisher follows its lowered operand");
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
    Ok(results[0])
}

#[cfg(test)]
#[expect(clippy::panic, reason = "test failures report unexpected IR shapes")]
mod tests {
    use crate::{
        context::Compilation,
        diagnostics::{Class, Diagnostics, Site},
        helper::Id,
        interner::{SYM_IF, SYM_REC, SymId},
        lowering::{
            BlockId, BodyId, CaptureSource, ClosureMode, Datum, DatumId,
            Instruction, Module, Target, TerminatorKind,
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

        fn lower_ok(&self, forms: &[HirForm]) -> Module {
            let (module, diagnostics) = self.lower(forms);
            assert!(diagnostics.items().is_empty(), "unexpected diagnostics");
            module.expect("forms lower successfully")
        }
    }

    fn closure_at(
        module: &Module,
        body: BodyId,
        instruction: usize,
        expected_mode: ClosureMode,
    ) -> BodyId {
        match module
            .bodies
            .get(body)
            .blocks
            .get(BlockId::default())
            .instructions[instruction]
        {
            Instruction::MakeClosure(child, mode) => {
                assert_eq!(mode, expected_mode);
                child
            }
            ref other => panic!("expected closure construction, got {other:?}"),
        }
    }

    #[test]
    fn flat_closure_links_body_and_restores_parent_continuation() {
        let mut fixture = Fixture::new();
        let body_value = fixture.form(HirKind::Int(20));
        let vector = fixture.form(HirKind::Vector(vec![body_value]));
        let tail = fixture.form(HirKind::Int(30));
        let vector_id = vector.id;
        let tail_id = tail.id;
        let module = fixture.lower_ok(&[vector, tail]);

        let entry = module.get_entry();
        let entry_block = entry.blocks.get(BlockId::default());
        assert_eq!(entry_block.origins, [vector_id, tail_id]);
        let child =
            closure_at(&module, BodyId::default(), 0, ClosureMode::Normal);
        assert_eq!(module.bodies.get(child).origin, Some(vector_id));
        assert!(matches!(
            module
                .bodies
                .get(child)
                .blocks
                .get(BlockId::default())
                .instructions[0],
            Instruction::PushData(_)
        ));
        assert!(matches!(
            entry_block.instructions[1],
            Instruction::PushData(_)
        ));
        assert_eq!(entry_block.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn empty_closure_has_an_empty_ending_entry_block() {
        let mut fixture = Fixture::new();
        let vector = fixture.form(HirKind::Vector(vec![]));
        let vector_id = vector.id;
        let module = fixture.lower_ok(&[vector]);
        let child =
            closure_at(&module, BodyId::default(), 0, ClosureMode::Normal);
        let body = module.bodies.get(child);
        assert_eq!(body.origin, Some(vector_id));
        assert_eq!(body.blocks.len(), 1);
        let block = body.blocks.get(BlockId::default());
        assert!(block.instructions.is_empty());
        assert_eq!(block.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn nested_closures_without_bindings_restore_each_parent() {
        let mut fixture = Fixture::new();
        let leaf_value = fixture.form(HirKind::Int(7));
        let inner = fixture.form(HirKind::Vector(vec![leaf_value]));
        let middle_tail = fixture.form(HirKind::Int(8));
        let inner_id = inner.id;
        let middle_tail_id = middle_tail.id;
        let outer = fixture.form(HirKind::Vector(vec![inner, middle_tail]));
        let outer_id = outer.id;
        let entry_tail = fixture.form(HirKind::Int(9));
        let entry_tail_id = entry_tail.id;
        let module = fixture.lower_ok(&[outer, entry_tail]);

        let middle_id =
            closure_at(&module, BodyId::default(), 0, ClosureMode::Normal);
        let middle = module.bodies.get(middle_id);
        assert_eq!(middle.origin, Some(outer_id));
        assert_eq!(
            middle.blocks.get(BlockId::default()).origins,
            [inner_id, middle_tail_id]
        );
        let leaf_id = closure_at(&module, middle_id, 0, ClosureMode::Normal);
        assert_eq!(module.bodies.get(leaf_id).origin, Some(inner_id));
        assert!(matches!(
            module
                .bodies
                .get(leaf_id)
                .blocks
                .get(BlockId::default())
                .instructions[0],
            Instruction::PushData(_)
        ));
        assert_eq!(
            module.get_entry().blocks.get(BlockId::default()).origins,
            [outer_id, entry_tail_id]
        );
    }

    #[test]
    fn nested_closure_demand_forwards_and_deduplicates_captures() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("transitive-capture-x");
        let bind = fixture.form(HirKind::Bind(x));
        let load = fixture.form(HirKind::Load(x));
        let call = fixture.form(HirKind::Call(x));
        let load_id = load.id;
        let call_id = call.id;
        let inner = fixture.form(HirKind::Vector(vec![load, call]));
        let inner_id = inner.id;
        let middle = fixture.form(HirKind::Vector(vec![inner]));
        let middle_id = middle.id;
        let module = fixture.lower_ok(&[bind, middle]);

        let local = match module
            .get_entry()
            .blocks
            .get(BlockId::default())
            .instructions[0]
        {
            Instruction::Bind(local) => local,
            ref other => panic!("expected bind, got {other:?}"),
        };
        let mid_id =
            closure_at(&module, BodyId::default(), 1, ClosureMode::Normal);
        let child_id = closure_at(&module, mid_id, 0, ClosureMode::Normal);
        let mid = module.bodies.get(mid_id);
        let child = module.bodies.get(child_id);
        assert_eq!(mid.origin, Some(middle_id));
        assert_eq!(
            mid.captures().collect::<Vec<_>>(),
            [CaptureSource::Local(local)]
        );
        assert_eq!(child.origin, Some(inner_id));
        assert_eq!(child.captures().len(), 1);
        let middle_capture = crate::lowering::CaptureId::from_index(0);
        assert_eq!(
            child.captures().collect::<Vec<_>>(),
            [CaptureSource::Captured(middle_capture)]
        );
        let child_block = child.blocks.get(BlockId::default());
        assert_eq!(child_block.origins, [load_id, call_id]);
        let load_capture = match child_block.instructions[0] {
            Instruction::Load(Target::Captured(capture)) => capture,
            ref other => panic!("expected captured load, got {other:?}"),
        };
        let call_capture = match child_block.instructions[1] {
            Instruction::Call(Target::Captured(capture)) => capture,
            ref other => panic!("expected captured call, got {other:?}"),
        };
        assert_eq!(load_capture, middle_capture);
        assert_eq!(call_capture, load_capture);
    }

    #[test]
    fn recognised_conditional_uses_inline_arms_and_continuation() {
        let mut fixture = Fixture::new();
        let one = fixture.form(HirKind::Int(1));
        let then_value = fixture.form(HirKind::Int(11));
        let then_value_id = then_value.id;
        let then_arm = fixture.form(HirKind::Vector(vec![then_value]));
        let else_value = fixture.form(HirKind::Int(12));
        let else_value_id = else_value.id;
        let else_arm = fixture.form(HirKind::Vector(vec![else_value]));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let suffix = fixture.form(HirKind::Int(13));
        let one_id = one.id;
        let operator_id = operator.id;
        let suffix_id = suffix.id;
        let module =
            fixture.lower_ok(&[one, then_arm, else_arm, operator, suffix]);

        let entry = module.get_entry();
        assert_eq!(
            entry.blocks.len(),
            4,
            "entry, two arms, and non-empty continuation"
        );
        let entry_block = entry.blocks.get(BlockId::default());
        assert_eq!(entry_block.origins, [one_id]);
        assert_eq!(entry_block.terminator.origin, Some(operator_id));
        let (then_block, else_block) = match entry_block.terminator.kind {
            TerminatorKind::Branch {
                then_block,
                else_block,
            } => (then_block, else_block),
            other => panic!("expected recognized branch, got {other:?}"),
        };
        assert!(
            entry_block
                .instructions
                .iter()
                .all(|inst| !matches!(inst, Instruction::MakeClosure(..)))
        );
        for (arm, value) in
            [(then_block, then_value_id), (else_block, else_value_id)]
        {
            let block = entry.blocks.get(arm);
            assert_eq!(block.origins, [value]);
            assert!(matches!(block.instructions[0], Instruction::PushData(_)));
        }
        let then_exit = entry.blocks.get(then_block).terminator.kind;
        let else_exit = entry.blocks.get(else_block).terminator.kind;
        let continuation_id = match then_exit {
            TerminatorKind::Jump(id) => id,
            other => panic!(
                "expected then arm to jump to continuation, got {other:?}"
            ),
        };
        assert_eq!(else_exit, TerminatorKind::Jump(continuation_id));
        let continuation = entry.blocks.get(continuation_id);
        assert_eq!(continuation.origins, [suffix_id]);
        assert_eq!(continuation.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn recognized_conditional_with_empty_vector_arms_has_no_join_block() {
        let mut fixture = Fixture::new();
        let empty_then = fixture.form(HirKind::Vector(vec![]));
        let empty_else = fixture.form(HirKind::Vector(vec![]));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let operator_id = operator.id;
        let module = fixture.lower_ok(&[empty_then, empty_else, operator]);
        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 3);
        let branch = entry.blocks.get(BlockId::default());
        assert_eq!(branch.terminator.origin, Some(operator_id));
        let (then_block, else_block) = match branch.terminator.kind {
            TerminatorKind::Branch {
                then_block,
                else_block,
            } => (then_block, else_block),
            other => panic!("expected branch, got {other:?}"),
        };
        assert_eq!(
            entry.blocks.get(then_block).terminator.kind,
            TerminatorKind::End
        );
        assert_eq!(
            entry.blocks.get(else_block).terminator.kind,
            TerminatorKind::End
        );
    }

    #[test]
    fn conditional_data_arms_emit_pushes_with_arm_origins() {
        let mut fixture = Fixture::new();
        let atom = fixture.sym("data-arm-atom");
        let int_arm = fixture.form(HirKind::Int(21));
        let quote_child = fixture.form(HirKind::Call(atom));
        let quote_arm = fixture.form(HirKind::Quote(Box::new(quote_child)));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let int_id = int_arm.id;
        let quote_id = quote_arm.id;
        let operator_id = operator.id;
        let module = fixture.lower_ok(&[int_arm, quote_arm, operator]);
        let entry = module.get_entry();
        let branch = entry.blocks.get(BlockId::default());
        assert_eq!(branch.terminator.origin, Some(operator_id));
        let (then_block, else_block) = match branch.terminator.kind {
            TerminatorKind::Branch {
                then_block,
                else_block,
            } => (then_block, else_block),
            other => panic!("expected data-arm conditional, got {other:?}"),
        };
        let then = entry.blocks.get(then_block);
        let els = entry.blocks.get(else_block);
        assert_eq!(then.origins, [int_id]);
        assert_eq!(els.origins, [quote_id]);
        assert!(matches!(then.instructions[0], Instruction::PushData(_)));
        assert!(matches!(els.instructions[0], Instruction::PushData(_)));
        assert_eq!(
            module.bodies.len(),
            1,
            "data arms do not allocate closures"
        );
    }

    #[test]
    fn nested_conditionals_keep_inner_exits_on_outer_join() {
        let mut fixture = Fixture::new();
        let inner_then = fixture.form(HirKind::Vector(vec![]));
        let inner_else = fixture.form(HirKind::Vector(vec![]));
        let inner_if = fixture.form(HirKind::Call(SYM_IF));
        let outer_then = fixture
            .form(HirKind::Vector(vec![inner_then, inner_else, inner_if]));
        let outer_else = fixture.form(HirKind::Vector(vec![]));
        let outer_if = fixture.form(HirKind::Call(SYM_IF));
        let suffix = fixture.form(HirKind::Int(44));
        let outer_if_id = outer_if.id;
        let suffix_id = suffix.id;
        let module =
            fixture.lower_ok(&[outer_then, outer_else, outer_if, suffix]);

        let entry = module.get_entry();
        assert_eq!(
            entry.blocks.len(),
            6,
            "two levels of branches and one join"
        );
        let top = entry.blocks.get(BlockId::default());
        assert_eq!(top.terminator.origin, Some(outer_if_id));
        let (outer_then_block, outer_else_block) = match top.terminator.kind {
            TerminatorKind::Branch {
                then_block,
                else_block,
            } => (then_block, else_block),
            other => panic!("expected outer branch, got {other:?}"),
        };
        let outer_join =
            match entry.blocks.get(outer_else_block).terminator.kind {
                TerminatorKind::Jump(id) => id,
                other => panic!(
                    "expected else arm to jump to outer join, got {other:?}"
                ),
            };
        let inner = entry.blocks.get(outer_then_block);
        let (inner_then_block, inner_else_block) = match inner.terminator.kind {
            TerminatorKind::Branch {
                then_block,
                else_block,
            } => (then_block, else_block),
            other => panic!("expected nested branch, got {other:?}"),
        };
        assert_eq!(
            entry.blocks.get(inner_then_block).terminator.kind,
            TerminatorKind::Jump(outer_join)
        );
        assert_eq!(
            entry.blocks.get(inner_else_block).terminator.kind,
            TerminatorKind::Jump(outer_join)
        );
        assert_eq!(entry.blocks.get(outer_join).origins, [suffix_id]);
    }

    #[test]
    fn mixed_nested_conditional_recognises_outer_and_keeps_inner_dynamic() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("mixed-if-x");
        let bind = fixture.form(HirKind::Bind(x));
        let dynamic_load = fixture.form(HirKind::Load(x));
        let dynamic_then = fixture.form(HirKind::Int(1));
        let dynamic_if = fixture.form(HirKind::Call(SYM_IF));
        let dynamic_operator = dynamic_if.id;
        let dynamic_arm = fixture.form(HirKind::Vector(vec![
            dynamic_load,
            dynamic_then,
            dynamic_if,
        ]));
        let static_arm = fixture.form(HirKind::Vector(vec![]));
        let outer_if = fixture.form(HirKind::Call(SYM_IF));
        let module =
            fixture.lower_ok(&[bind, dynamic_arm, static_arm, outer_if]);

        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 3, "only the outer form is a branch");
        let (then_block, else_block) =
            match entry.blocks.get(BlockId::default()).terminator.kind {
                TerminatorKind::Branch {
                    then_block,
                    else_block,
                } => (then_block, else_block),
                other => {
                    panic!("expected recognized outer branch, got {other:?}")
                }
            };
        let dynamic = entry.blocks.get(then_block);
        assert_eq!(dynamic.origins.last(), Some(&dynamic_operator));
        assert!(matches!(
            dynamic.instructions.last(),
            Some(Instruction::Call(Target::Primitive(_)))
        ));
        assert_eq!(entry.blocks.get(else_block).instructions.len(), 0);
    }

    #[test]
    fn wholly_unrecognized_conditional_is_lowered_as_ordinary_calls() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("dynamic-if-x");
        let bind = fixture.form(HirKind::Bind(x));
        let load = fixture.form(HirKind::Load(x));
        let arm = fixture.form(HirKind::Int(2));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let operator_id = operator.id;
        let module = fixture.lower_ok(&[bind, load, arm, operator]);
        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 1);
        let block = entry.blocks.get(BlockId::default());
        assert_eq!(block.origins.last(), Some(&operator_id));
        assert!(matches!(
            block.instructions.last(),
            Some(Instruction::Call(Target::Primitive(_)))
        ));
        assert_eq!(block.terminator.kind, TerminatorKind::End);
    }

    #[test]
    fn call_and_bind_operands_keep_conditional_prefixes_dynamic() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("dynamic-if-call");
        let y = fixture.sym("dynamic-if-bind");
        let bind_x = fixture.form(HirKind::Bind(x));
        let call_operand = fixture.form(HirKind::Call(x));
        let int_arm = fixture.form(HirKind::Int(3));
        let first_if = fixture.form(HirKind::Call(SYM_IF));
        let int_arm_two = fixture.form(HirKind::Int(4));
        let bind_operand = fixture.form(HirKind::Bind(y));
        let second_if = fixture.form(HirKind::Call(SYM_IF));
        let module = fixture.lower_ok(&[
            bind_x,
            call_operand,
            int_arm,
            first_if,
            int_arm_two,
            bind_operand,
            second_if,
        ]);
        let entry = module.get_entry();
        assert_eq!(
            entry.blocks.len(),
            1,
            "neither conditional prefix branches"
        );
        let block = entry.blocks.get(BlockId::default());
        assert!(matches!(
            block.instructions[1],
            Instruction::Call(Target::Local(_))
        ));
        assert!(matches!(
            block.instructions[3],
            Instruction::Call(Target::Primitive(_))
        ));
        assert!(matches!(block.instructions[5], Instruction::Bind(_)));
        assert!(matches!(
            block.instructions[6],
            Instruction::Call(Target::Primitive(_))
        ));
    }

    #[test]
    fn conditional_arm_bindings_are_isolated_and_outer_binding_is_restored() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("branch-local-x");
        let outer_bind = fixture.form(HirKind::Bind(x));
        let then_bind = fixture.form(HirKind::Bind(x));
        let then_load = fixture.form(HirKind::Load(x));
        let then_arm =
            fixture.form(HirKind::Vector(vec![then_bind, then_load]));
        let else_bind = fixture.form(HirKind::Bind(x));
        let else_load = fixture.form(HirKind::Load(x));
        let else_arm =
            fixture.form(HirKind::Vector(vec![else_bind, else_load]));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let after = fixture.form(HirKind::Load(x));
        let module = fixture
            .lower_ok(&[outer_bind, then_arm, else_arm, operator, after]);

        let entry = module.get_entry();
        assert_eq!(entry.locals.len(), 3);
        let (then_id, else_id) =
            match entry.blocks.get(BlockId::default()).terminator.kind {
                TerminatorKind::Branch {
                    then_block,
                    else_block,
                } => (then_block, else_block),
                other => panic!("expected branch, got {other:?}"),
            };
        let then = entry.blocks.get(then_id);
        let els = entry.blocks.get(else_id);
        let local = |inst: &Instruction| match inst {
            Instruction::Bind(local) => *local,
            other => panic!("expected bind, got {other:?}"),
        };
        assert!(
            matches!(then.instructions[1], Instruction::Load(Target::Local(id)) if id == local(&then.instructions[0]))
        );
        assert!(
            matches!(els.instructions[1], Instruction::Load(Target::Local(id)) if id == local(&els.instructions[0]))
        );
        assert_ne!(local(&then.instructions[0]), local(&els.instructions[0]));
        let continuation = match then.terminator.kind {
            TerminatorKind::Jump(id) => id,
            other => panic!("expected branch join, got {other:?}"),
        };
        let outer_local =
            match entry.blocks.get(BlockId::default()).instructions[0] {
                Instruction::Bind(local) => local,
                ref other => panic!("expected outer binding, got {other:?}"),
            };
        assert!(
            matches!(entry.blocks.get(continuation).instructions[0], Instruction::Load(Target::Local(local)) if local == outer_local)
        );
    }

    #[test]
    fn closure_inside_arm_captures_arm_binding_then_restores_context() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("closure-in-arm-x");
        let outer_bind = fixture.form(HirKind::Bind(x));
        let arm_bind = fixture.form(HirKind::Bind(x));
        let captured_load = fixture.form(HirKind::Load(x));
        let closure = fixture.form(HirKind::Vector(vec![captured_load]));
        let arm_load = fixture.form(HirKind::Load(x));
        let then_arm =
            fixture.form(HirKind::Vector(vec![arm_bind, closure, arm_load]));
        let else_arm = fixture.form(HirKind::Vector(vec![]));
        let operator = fixture.form(HirKind::Call(SYM_IF));
        let after = fixture.form(HirKind::Load(x));
        let module = fixture
            .lower_ok(&[outer_bind, then_arm, else_arm, operator, after]);

        let entry = module.get_entry();
        let (then_id, _) =
            match entry.blocks.get(BlockId::default()).terminator.kind {
                TerminatorKind::Branch {
                    then_block,
                    else_block,
                } => (then_block, else_block),
                other => panic!("expected branch, got {other:?}"),
            };
        let then = entry.blocks.get(then_id);
        let arm_local = match then.instructions[0] {
            Instruction::Bind(local) => local,
            ref other => panic!("expected arm binding, got {other:?}"),
        };
        let child_id = match then.instructions[1] {
            Instruction::MakeClosure(body, ClosureMode::Normal) => body,
            ref other => panic!("expected closure in arm, got {other:?}"),
        };
        assert_eq!(
            module.bodies.get(child_id).captures().collect::<Vec<_>>(),
            [CaptureSource::Local(arm_local)]
        );
        assert!(
            matches!(then.instructions[2], Instruction::Load(Target::Local(local)) if local == arm_local)
        );
        let continuation = match then.terminator.kind {
            TerminatorKind::Jump(id) => id,
            other => panic!("expected arm continuation, got {other:?}"),
        };
        assert!(
            matches!(entry.blocks.get(continuation).instructions[0], Instruction::Load(Target::Local(local)) if local != arm_local)
        );
    }

    #[test]
    fn shadowed_if_and_rec_are_ordinary_local_references() {
        let mut fixture = Fixture::new();
        let bind_if = fixture.form(HirKind::Bind(SYM_IF));
        let bind_rec = fixture.form(HirKind::Bind(SYM_REC));
        let first = fixture.form(HirKind::Int(1));
        let second = fixture.form(HirKind::Int(2));
        let if_call = fixture.form(HirKind::Call(SYM_IF));
        let vector = fixture.form(HirKind::Vector(vec![]));
        let rec_call = fixture.form(HirKind::Call(SYM_REC));
        let module = fixture.lower_ok(&[
            bind_if, bind_rec, first, second, if_call, vector, rec_call,
        ]);
        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 1);
        assert_eq!(
            module.bodies.len(),
            2,
            "shadowed recursive suffix is a normal closure"
        );
        let block = entry.blocks.get(BlockId::default());
        let if_local = match block.instructions[0] {
            Instruction::Bind(id) => id,
            ref other => panic!("expected bind, got {other:?}"),
        };
        let rec_local = match block.instructions[1] {
            Instruction::Bind(id) => id,
            ref other => panic!("expected bind, got {other:?}"),
        };
        assert!(
            matches!(block.instructions[4], Instruction::Call(Target::Local(id)) if id == if_local)
        );
        assert!(matches!(
            block.instructions[5],
            Instruction::MakeClosure(_, ClosureMode::Normal)
        ));
        assert!(
            matches!(block.instructions[6], Instruction::Call(Target::Local(id)) if id == rec_local)
        );
    }

    #[test]
    fn recursive_closure_consumes_operator_and_preserves_parent_remainder() {
        let mut fixture = Fixture::new();
        let body_form = fixture.form(HirKind::Call(SYM_REC));
        let vector = fixture.form(HirKind::Vector(vec![body_form]));
        let operator = fixture.form(HirKind::Call(SYM_REC));
        let tail = fixture.form(HirKind::Int(55));
        let vector_id = vector.id;
        let tail_id = tail.id;
        let module = fixture.lower_ok(&[vector, operator, tail]);
        let entry = module.get_entry();
        let block = entry.blocks.get(BlockId::default());
        assert_eq!(
            block.origins,
            [vector_id, tail_id],
            "rec operator is consumed"
        );
        let child =
            closure_at(&module, BodyId::default(), 0, ClosureMode::Recursive);
        assert_eq!(module.bodies.get(child).origin, Some(vector_id));
        assert!(matches!(
            module
                .bodies
                .get(child)
                .blocks
                .get(BlockId::default())
                .instructions[0],
            Instruction::Call(Target::Primitive(_))
        ));
        assert!(matches!(block.instructions[1], Instruction::PushData(_)));
    }

    #[test]
    fn invalid_recursive_data_reports_and_recovers_through_suffix() {
        let mut fixture = Fixture::new();
        let missing = fixture.sym("missing-after-invalid-rec");
        let int_operand = fixture.form(HirKind::Int(5));
        let rec_one = fixture.form(HirKind::Call(SYM_REC));
        let quote_value = fixture.form(HirKind::Int(6));
        let quote_operand = fixture.form(HirKind::Quote(Box::new(quote_value)));
        let rec_two = fixture.form(HirKind::Call(SYM_REC));
        let list_child = fixture.form(HirKind::Int(7));
        let list_operand = fixture.form(HirKind::List(vec![list_child]));
        let rec_three = fixture.form(HirKind::Call(SYM_REC));
        let unresolved = fixture.form(HirKind::Load(missing));
        let operand_ids = [int_operand.id, quote_operand.id, list_operand.id];
        let unresolved_id = unresolved.id;
        let (module, diagnostics) = fixture.lower(&[
            int_operand,
            rec_one,
            quote_operand,
            rec_two,
            list_operand,
            rec_three,
            unresolved,
        ]);
        assert!(module.is_none());
        assert_eq!(diagnostics.items().len(), 4);
        for (diagnostic, origin) in
            diagnostics.items()[..3].iter().zip(operand_ids)
        {
            assert_eq!(diagnostic.class, Class::LoweringRecursionDataOperand);
            assert_eq!(diagnostic.site, Site::Syntax(origin));
        }
        assert_eq!(
            diagnostics.items()[3].class,
            Class::LoweringUnresolvedSymbol
        );
        assert_eq!(diagnostics.items()[3].site, Site::Syntax(unresolved_id));
    }

    #[test]
    fn dynamic_recursion_operand_keeps_ordinary_rec_call_path() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("dynamic-rec-x");
        let bind = fixture.form(HirKind::Bind(x));
        let load = fixture.form(HirKind::Load(x));
        let rec = fixture.form(HirKind::Call(SYM_REC));
        let module = fixture.lower_ok(&[bind, load, rec]);
        let block = module.get_entry().blocks.get(BlockId::default());
        assert!(matches!(
            block.instructions[2],
            Instruction::Call(Target::Primitive(_))
        ));
        assert_eq!(module.bodies.len(), 1);
    }

    #[test]
    fn recursion_inside_closure_keeps_recursive_mode() {
        let mut fixture = Fixture::new();
        let recursive_body = fixture.form(HirKind::Int(1));
        let recursive_vector =
            fixture.form(HirKind::Vector(vec![recursive_body]));
        let rec = fixture.form(HirKind::Call(SYM_REC));
        let enclosing =
            fixture.form(HirKind::Vector(vec![recursive_vector, rec]));
        let module = fixture.lower_ok(&[enclosing]);
        let outer_id =
            closure_at(&module, BodyId::default(), 0, ClosureMode::Normal);
        let inner_id = closure_at(&module, outer_id, 0, ClosureMode::Recursive);
        assert!(module.bodies.get(inner_id).origin.is_some());
    }

    #[test]
    fn deeply_nested_executable_closures_lower_iteratively() {
        let mut fixture = Fixture::new();
        let depth = 12_000;
        let mut nested = fixture.form(HirKind::Int(-1));
        for _ in 0..depth {
            nested = fixture.form(HirKind::Vector(vec![nested]));
        }
        let module = fixture.lower_ok(&[nested]);
        assert_eq!(module.bodies.len(), depth + 1);

        let mut body_id = BodyId::default();
        for _ in 0..depth {
            body_id = match module
                .bodies
                .get(body_id)
                .blocks
                .get(BlockId::default())
                .instructions[0]
            {
                Instruction::MakeClosure(child, ClosureMode::Normal) => child,
                ref other => panic!("expected nested closure, got {other:?}"),
            };
        }
        assert!(matches!(
            module
                .bodies
                .get(body_id)
                .blocks
                .get(BlockId::default())
                .instructions[0],
            Instruction::PushData(_)
        ));
    }

    #[test]
    fn deeply_nested_conditionals_inherit_end_without_join_blocks() {
        let mut fixture = Fixture::new();
        let depth = 12_000;
        let mut forms = vec![fixture.form(HirKind::Int(-1))];
        for _ in 0..depth {
            let then_arm = fixture.form(HirKind::Vector(forms));
            let else_arm = fixture.form(HirKind::Vector(vec![]));
            let operator = fixture.form(HirKind::Call(SYM_IF));
            forms = vec![then_arm, else_arm, operator];
        }
        let module = fixture.lower_ok(&forms);
        assert_eq!(module.bodies.len(), 1);
        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 2 * depth + 1);
        let mut block = BlockId::default();
        for _ in 0..depth {
            let TerminatorKind::Branch {
                then_block,
                else_block,
            } = entry.blocks.get(block).terminator.kind
            else {
                panic!("expected nested conditional");
            };
            let otherwise = entry.blocks.get(else_block);
            assert!(otherwise.instructions.is_empty());
            assert_eq!(otherwise.terminator.kind, TerminatorKind::End);
            block = then_block;
        }
        let leaf = entry.blocks.get(block);
        assert_eq!(leaf.terminator.kind, TerminatorKind::End);
        let Instruction::PushData(datum) = leaf.instructions[0] else {
            panic!("expected leaf datum");
        };
        assert!(matches!(module.data.get(datum), Datum::Int(-1)));
    }

    #[test]
    fn dynamic_conditional_operand_closure_contains_recognised_conditional() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("dynamic-outer-if");
        let bind = fixture.form(HirKind::Bind(x));
        let inner_then = fixture.form(HirKind::Vector(vec![]));
        let inner_else = fixture.form(HirKind::Vector(vec![]));
        let inner_if = fixture.form(HirKind::Call(SYM_IF));
        let inner_origin = inner_if.id;
        let closure = fixture
            .form(HirKind::Vector(vec![inner_then, inner_else, inner_if]));
        let load = fixture.form(HirKind::Load(x));
        let outer_if = fixture.form(HirKind::Call(SYM_IF));
        let tail = fixture.form(HirKind::Int(42));
        let origins = [bind.id, closure.id, load.id, outer_if.id, tail.id];
        let module = fixture.lower_ok(&[bind, closure, load, outer_if, tail]);
        let entry = module.get_entry();
        assert_eq!(entry.blocks.len(), 1);
        let block = entry.blocks.get(BlockId::default());
        assert_eq!(block.origins, origins);
        assert!(matches!(
            block.instructions[3],
            Instruction::Call(Target::Primitive(_))
        ));
        assert_eq!(block.terminator.kind, TerminatorKind::End);
        let child =
            closure_at(&module, BodyId::default(), 1, ClosureMode::Normal);
        let body = module.bodies.get(child);
        assert_eq!(body.blocks.len(), 3);
        let branch = &body.blocks.get(BlockId::default()).terminator;
        assert_eq!(branch.origin, Some(inner_origin));
        assert!(matches!(branch.kind, TerminatorKind::Branch { .. }));
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
    fn malformed_hir_datum_reports_ice_and_continues() {
        let mut fixture = Fixture::new();
        let x = fixture.sym("invalid-datum-x");
        let bind = fixture.form(HirKind::Bind(x));
        let bind_id = bind.id;
        let list = fixture.form(HirKind::List(vec![bind]));
        let load = fixture.form(HirKind::Load(x));
        let load_id = load.id;
        let quote = fixture.form(HirKind::Quote(Box::new(load)));
        let missing = fixture.form(HirKind::Call(x));
        let missing_id = missing.id;

        let (module, diagnostics) = fixture.lower(&[list, quote, missing]);
        assert!(module.is_none());
        let items = diagnostics.items();
        assert_eq!(items.len(), 3);
        for (item, origin) in items[..2].iter().zip([bind_id, load_id]) {
            assert_eq!(item.class, Class::ICEInvalidHIRDatum);
            assert_eq!(item.site, Site::Syntax(origin));
        }
        assert_eq!(items[2].class, Class::LoweringUnresolvedSymbol);
        assert_eq!(items[2].site, Site::Syntax(missing_id));
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
