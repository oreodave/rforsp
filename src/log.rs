//! Compiler phase logging.

use crate::{
    context::Compilation,
    lexer::Token,
    parser::{HirForm, HirKind, dfs},
    source::SourceId,
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
