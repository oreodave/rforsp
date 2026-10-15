//! Compiler phase logging.

use std::collections::HashMap;
use std::fmt::Write;

use crate::{
    context::Compilation,
    lexer::Token,
    lowering::sir::{
        BlockId, BodyId, Datum, Instruction, Module, Target, TerminatorKind,
    },
    parser::{HirForm, HirKind, dfs},
    source::{SourceId, SyntaxId},
};

/// Set of compiler phases to log.
///
/// One bit per stage, so several may be requested at once.  The field is
/// private and the constants below are the only constructors, so a value
/// holding bits no stage owns cannot be built.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Log(u8);

impl Log {
    /// No logs.
    pub const NONE: Self = Self(0);
    /// Log the tokens produced by phase 1.
    pub const TOKENS: Self = Self(1 << 0);
    /// Log the [`HirForm`]s produced by phase 2.
    pub const HIR: Self = Self(1 << 1);
    /// Log the [`Module`] produced by phase 3.
    pub const SIR: Self = Self(1 << 2);
    /// Log all results produced by every phase.
    pub const ALL: Self = Self(0xFF);

    /// Whether every stage in `other` is set in this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Add every stage in `other` to this set.
    pub const fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Log tokens if and only if `log` contains [`Log::TOKENS`].
///
/// # Errors
/// - repeated back from `writeln` calls.
pub fn log_tokens(
    sources: &[SourceId],
    lexes: &[Vec<Token>],
    log: Log,
    ctx: &Compilation,
    log_out: &mut impl std::fmt::Write,
) -> std::fmt::Result {
    if !log.contains(Log::TOKENS) {
        return Ok(());
    }

    writeln!(log_out, "TOKENS:")?;
    for (&id, lex_stream) in sources.iter().zip(lexes) {
        let source = ctx.table.get_source(id);
        write!(log_out, "  {}: ", source.name)?;
        log_count(log_out, source.len(), "byte", "bytes")?;
        write!(log_out, " => ")?;
        log_count(log_out, lex_stream.len(), "token", "tokens")?;
        writeln!(log_out)?;

        write!(log_out, "    ")?;
        for (count, token) in lex_stream.iter().enumerate() {
            let kind = token.kind;
            let text = source.span_text(token.span);
            write!(log_out, "{kind:?}({text}), ")?;
            if count % 8 == 0 && count != 0 {
                write!(log_out, "\n\t")?;
            }
        }
        writeln!(log_out)?;
    }

    Ok(())
}

/// Log the HIR forms if and only if `log` contains [`Log::HIR`].
///
/// # Errors
/// - repeated back from `writeln` calls.
pub fn log_hir(
    token_streams: &[Vec<Token>],
    forms: &[HirForm],
    log: Log,
    ctx: &Compilation,
    log_out: &mut impl std::fmt::Write,
) -> std::fmt::Result {
    if !log.contains(Log::HIR) {
        return Ok(());
    }

    let mut result = Ok(());
    let mut form_count = 0;
    dfs(forms, |_, _| form_count += 1);
    let token_count = token_streams.iter().map(Vec::len).sum::<usize>();

    write!(log_out, "HIR:\n  ")?;
    log_count(log_out, token_count, "token", "tokens")?;
    write!(log_out, " => ")?;
    log_count(log_out, form_count, "form", "forms")?;
    writeln!(log_out)?;

    dfs(forms, |form, depth| {
        if result.is_err() {
            return;
        }

        result = write!(
            log_out,
            "{:width$}{} ",
            "",
            form.kind.label_str(),
            width = (depth + 1) * 2
        )
        .and_then(|()| match &form.kind {
            HirKind::Int(_)
            | HirKind::Bind(_)
            | HirKind::Load(_)
            | HirKind::Call(_) => {
                writeln!(
                    log_out,
                    "{:?}",
                    ctx.table.text_of(ctx.table.get_origin(form.id))
                )
            }
            HirKind::Vector(xs) | HirKind::List(xs) => {
                writeln!(log_out, "[{}]", xs.len())
            }
            HirKind::Quote(_) => writeln!(log_out),
        });
    });

    result
}

/// Log the [`Module`] if and only if `log` contains [`Log::SIR`].
///
/// # Errors
/// - repeated back from `writeln` calls.
pub fn log_sir(
    forms: &[HirForm],
    module: &Module,
    log: Log,
    ctx: &Compilation,
    log_out: &mut impl std::fmt::Write,
) -> std::fmt::Result {
    if !log.contains(Log::SIR) {
        return Ok(());
    }

    // Generate a count of all the forms
    let form_count = {
        let mut form_count = 0;
        dfs(forms, |_, _| form_count += 1);
        form_count
    };

    // Generate a count of all the instructions, blocks, and bodies in the
    // module.
    let body_count = module.bodies.len();
    let mut block_count = 0;
    let mut inst_count = 0;
    for (_, body) in module.bodies.iter() {
        for (_, block) in body.blocks.iter() {
            block_count += 1;
            inst_count += block.instructions.len();
        }
    }

    write!(log_out, "SIR:\n  ")?;
    log_count(log_out, form_count, "form", "forms")?;
    write!(log_out, " => ")?;
    log_count(log_out, body_count, "body", "bodies")?;
    write!(log_out, ", ")?;
    log_count(log_out, block_count, "block", "blocks")?;
    write!(log_out, ", ")?;
    log_count(log_out, inst_count, "instruction", "instructions")?;
    writeln!(log_out)?;

    // Print datums owned by this module
    writeln!(log_out, "  section: data")?;
    for (datum_id, datum) in module.data.iter() {
        write!(log_out, "    {datum_id:?} => ")?;
        match datum {
            Datum::Int(n) => writeln!(log_out, "Int({n})"),
            Datum::Atom(sym) => {
                let s = ctx.interner.resolve(*sym);
                writeln!(log_out, "Atom({s})")
            }
            Datum::List(children) => {
                writeln!(log_out, "List({children:?})")
            }
            Datum::Vector(children) => {
                writeln!(log_out, "Vec({children:?})")
            }
            Datum::Quote(subform) => {
                writeln!(log_out, "Quote({subform:?})")
            }
        }?;
    }

    // Render per body instructions and terminators, ensuring we record the
    // maximum render length.
    let (renders, max_width) = render_blocks(ctx, module)?;

    // Now print the instructions and bodies.
    writeln!(log_out, "  section: code")?;
    for (body_id, body) in module.bodies.iter() {
        writeln!(
            log_out,
            "    body[{body_id:?}, #locals[{}], captures[{:?}]] {{",
            body.locals.len(),
            body.captures().collect::<Vec<_>>()
        )?;

        for (block_id, _) in body.blocks.iter() {
            writeln!(log_out, "      block[{block_id:?}]: {{")?;
            for (string, origin) in &renders[&(body_id, block_id)] {
                write!(log_out, "        {string:<max_width$}")?;
                if let Some(origin) = origin {
                    write!(
                        log_out,
                        "    <-- [{:?}]",
                        ctx.table.text_of(ctx.table.get_origin(*origin))
                    )?;
                }
                writeln!(log_out)?;
            }
            writeln!(log_out, "    }}")?;
        }

        writeln!(log_out, "  }}")?;
    }

    Ok(())
}

/// A completed render of all the [`BlockId`]s within all [`BodyId`]s in a
/// [`Module`].
type BlockRender = HashMap<(BodyId, BlockId), Vec<(String, Option<SyntaxId>)>>;

/// Render all [`BlockId`]s within the [`Module`] given.
/// On `Ok`, returns a [`BlockRender`] and the maximum width of any string
/// rendered.
fn render_blocks(
    ctx: &Compilation,
    module: &Module,
) -> Result<(BlockRender, usize), std::fmt::Error> {
    let mut renders = BlockRender::new();
    let mut max_width = 0;

    for (body_id, body) in module.bodies.iter() {
        for (block_id, block) in body.blocks.iter() {
            // Collection of rendered strings for this specific (Body,Block)
            // combination.
            let mut strings = Vec::new();

            for (inst, origin) in
                block.instructions.iter().zip(block.origins.iter())
            {
                let string = match inst {
                    Instruction::PushData(datum) => {
                        format!("PushData({datum:?})")
                    }
                    Instruction::Load(target) => {
                        let mut x = String::new();
                        write!(x, "Load(")?;
                        log_target(&mut x, ctx, target)?;
                        write!(x, ")")?;
                        x
                    }
                    Instruction::Call(target) => {
                        let mut x = String::new();
                        write!(x, "Call(")?;
                        log_target(&mut x, ctx, target)?;
                        write!(x, ")")?;
                        x
                    }
                    Instruction::Bind(local) => {
                        format!("Bind({local:?})")
                    }
                    Instruction::MakeClosure(body_id, mode) => {
                        format!("MakeClosure({body_id:?}, {mode:?})")
                    }
                };
                strings.push((string, Some(*origin)));
            }

            // Render terminator as well.
            {
                let mut terminator_string = String::new();
                write!(terminator_string, "Terminates[")?;
                match block.terminator.kind {
                    TerminatorKind::End => write!(terminator_string, "End"),
                    TerminatorKind::Jump(block) => {
                        write!(terminator_string, "Jump({block:?})")
                    }
                    TerminatorKind::Branch {
                        then_block,
                        else_block,
                    } => {
                        write!(
                            terminator_string,
                            "Branch(then={then_block:?}, else={else_block:?})"
                        )
                    }
                }?;
                write!(terminator_string, "]")?;
                strings.push((terminator_string, block.terminator.origin));
            }

            // Compute maximum width.
            max_width = std::cmp::max(
                max_width,
                strings
                    .iter()
                    .map(|(s, _)| s.chars().count())
                    .max()
                    .unwrap_or(0),
            );

            renders.insert((body_id, block_id), strings);
        }
    }

    Ok((renders, max_width))
}

/// Log the given lowering [`Target`].
fn log_target(
    log_out: &mut impl std::fmt::Write,
    ctx: &Compilation,
    target: &Target,
) -> std::fmt::Result {
    match target {
        Target::Primitive(id) => {
            let name = ctx.interner.resolve(ctx.primitives.get(*id).sym);
            write!(log_out, "Primitive({name})")
        }
        Target::Variable(id) => {
            let name = ctx.interner.resolve(ctx.variables.get(*id).sym);
            write!(log_out, "RuntimeVariable({name})")
        }
        target @ (Target::Local(_) | Target::Captured(_)) => {
            write!(log_out, "{target:?}")
        }
    }
}

/// Log a count of items, printing the `singular` and `plural` forms depending
/// on `count`.
fn log_count(
    log_out: &mut impl std::fmt::Write,
    count: usize,
    singular: &str,
    plural: &str,
) -> std::fmt::Result {
    write!(
        log_out,
        "{} {}",
        count,
        if count == 1 { singular } else { plural }
    )
}
