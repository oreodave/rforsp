//! Compiler phase logging
//!
//! This provides functionality to log the output of phases of the compiler.

use crate::{
    context::Compilation,
    lexer::Token,
    parser::{HirForm, HirKind, dfs},
    source::SourceId,
};

/// Set of compiler phases to log.
///
/// One bit per stage, so several may be requested at once.  The field is
/// private and the only constructors are the constants below, which is what
/// keeps a value holding bits no stage owns unconstructible.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Log(u8);

impl Log {
    /// No logs.
    pub const NONE: Self = Self(0);
    /// Log the tokens produced by phase 1.
    pub const TOKENS: Self = Self(1 << 0);

    /// Whether every stage in `other` is set in this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Add every stage in `other` to this set.
    ///
    /// Intended for argument parsing, where each flag accumulates rather than
    /// replacing what came before it.
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
