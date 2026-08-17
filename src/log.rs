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
    if log.contains(Log::TOKENS) {
        for (&id, lex_stream) in sources.iter().zip(lexes) {
            let source = ctx.table.get_source(id);
            writeln!(
                log_out,
                "{}: {} {} => {} {}",
                source.name,
                source.len(),
                if source.len() == 1 { "byte" } else { "bytes" },
                lex_stream.len(),
                if lex_stream.len() == 1 {
                    "token"
                } else {
                    "tokens"
                },
            )?;
            for token in lex_stream {
                let kind = token.kind;
                let text = source.span_text(token.span);
                write!(log_out, "{kind:?}({text}), ")?;
            }
            writeln!(log_out)?;
        }
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
    let mut result = Ok(());
    if log.contains(Log::HIR) {
        let mut form_count = 0;
        dfs(forms, |_, _| form_count += 1);
        let token_count = token_streams.iter().map(Vec::len).sum::<usize>();

        writeln!(
            log_out,
            "{} {} => {} {}",
            token_count,
            if token_count == 1 { "token" } else { "tokens" },
            form_count,
            if form_count == 1 { "form" } else { "forms" },
        )?;

        dfs(forms, |form, depth| {
            if result.is_err() {
                return;
            }

            result = write!(
                log_out,
                "{:width$}{} ",
                "",
                form.kind.label_str(),
                width = depth * 2
            )
            .and_then(|()| match &form.kind {
                HirKind::Int(_)
                | HirKind::Bind(_)
                | HirKind::Load(_)
                | HirKind::Call(_) => {
                    writeln!(
                        log_out,
                        "`{}`",
                        ctx.table.text_of(ctx.table.get_origin(form.id))
                    )
                }
                HirKind::Vector(xs) | HirKind::List(xs) => {
                    writeln!(log_out, "[{}]", xs.len())
                }
                HirKind::Quote(_) => writeln!(log_out),
            });
        });
    }
    result
}
