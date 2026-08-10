//! Main tokeniser runtime

use crate::{
    diagnostics::{Aborted, Diagnostics, Phase},
    lexer::{LexError, LexErrorKind, Token, TokenKind},
    source::{Source, SourceId, SourceTable, Span, SyntaxOrigin},
};

/// Characters that CANNOT be part of a scalar.
const RESTRICTED_CHARS: &str = "'^$()[];\n\t ";

/// Tokenise a source (given by [`SourceId`]) in a [`SourceTable`].
/// Accumulates [`Diagnostic`]s into the given [`Diagnostics`] object.
///
/// # Errors
/// - If any errors arise during lexing.
fn tokenise(
    source_id: SourceId,
    table: &SourceTable,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<Token>, Aborted> {
    let before = diagnostics.error_count();
    let mut tokeniser = Tokeniser::new(source_id, table, diagnostics);
    let mut tokens = Vec::new();

    tokeniser.skip_whitespace();
    while !tokeniser.source.eos(tokeniser.cursor) {
        if let Ok(token) = tokeniser.lex_singular() {
            tokens.push(token);
        }
        tokeniser.skip_whitespace();
    }

    if before < diagnostics.error_count() {
        Err(Aborted::new(Phase::Lex))
    } else {
        Ok(tokens)
    }
}

/// Tokeniser state structure.
struct Tokeniser<'a> {
    /// [`SourceId`] for source being tokenised.
    source_id: SourceId,
    /// [`Source`] being tokenised.
    source: &'a Source,
    /// [`Diagnostics`] to accumulate in.
    diagnostics: &'a mut Diagnostics,
    /// Current byte-position during tokenisation.
    cursor: usize,
}

impl<'a> Tokeniser<'a> {
    /// Construct a new tokeniser.
    fn new(
        source_id: SourceId,
        table: &'a SourceTable,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        let source = table.get_source(source_id);
        Self {
            source_id,
            source,
            diagnostics,
            cursor: 0,
        }
    }

    /// Peek at the character at the current point in the stream.
    fn peek(&self) -> std::str::Chars<'_> {
        self.source.chars_from(self.cursor)
    }

    /// Skip whitespace from the current cursor, stopping at the first non
    /// whitespace character.
    fn skip_whitespace(&mut self) {
        self.cursor += self.peek().take_while(|c| c.is_whitespace()).count();
    }

    /// Construct a new span of given length `len` starting at
    /// `Tokeniser::cursor`.
    const fn new_span(&self, len: usize) -> Span {
        Span::new(self.cursor, self.cursor + len)
    }

    /// Emit a [`Token`] of the given [`TokenKind`] which is known to consist of
    /// exactly `len` bytes.
    /// Advances the cursor by exactly `len` bytes as well.
    const fn emit_token(&mut self, kind: TokenKind, len: usize) -> Token {
        let token = Token {
            kind,
            span: self.new_span(len),
        };
        self.cursor += len;
        token
    }

    /// Lex a [`TokenKind::Symbol`]
    fn lex_sym(&mut self) -> Option<Token> {
        let items = self
            .peek()
            .take_while(|&c| !RESTRICTED_CHARS.contains(c))
            .collect::<Vec<_>>();

        match items.last() {
            None => None,
            Some(c) if c.is_numeric() => None,
            Some(_) => Some(self.emit_token(TokenKind::Symbol, items.len())),
        }
    }

    /// Lex a scalar value (either [`TokenKind::Number`] or
    /// [`TokenKind::Symbol`]).
    fn lex_scalar(&mut self) -> Option<Token> {
        let items = self
            .peek()
            .take_while(|&c| !RESTRICTED_CHARS.contains(c))
            .collect::<Vec<_>>();

        match items.last() {
            None => None,
            Some(c) if c.is_numeric() => {
                Some(self.emit_token(TokenKind::Number, items.len()))
            }
            Some(_) => Some(self.emit_token(TokenKind::Symbol, items.len())),
        }
    }

    /// Attempt to Lex a [`TokenKind::Bind`] (`$`) operator.
    ///
    /// # Errors:
    /// - If no valid symbol following an occurrence `$`.
    fn lex_bind(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        self.cursor += 1;
        self.lex_sym()
            .map(|tk| Token {
                kind: TokenKind::Bind,
                span: Span::new(start, tk.span.end as usize),
            })
            .ok_or_else(|| LexError {
                origin: SyntaxOrigin {
                    source: self.source_id,
                    span: Span::new(start, start + 1),
                },
                kind: LexErrorKind::BindInvalid,
            })
    }

    /// At
    fn lex_load(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        self.cursor += 1;
        self.lex_sym()
            .map(|tk| Token {
                kind: TokenKind::Load,
                span: Span::new(start, tk.span.end as usize),
            })
            .ok_or_else(|| LexError {
                origin: SyntaxOrigin {
                    source: self.source_id,
                    span: Span::new(start, start + 1),
                },
                kind: LexErrorKind::LoadInvalid,
            })
    }

    /// Attempt to lex a singular [`Token`] from the current source.
    ///
    /// # Errors
    /// - Sentinel value representing a failure in tokenisation.  This is
    ///   accompanied by a [`Diagnostic`] being pushed as well.
    fn lex_singular(&mut self) -> Result<Token, LexError> {
        assert!(
            !self.source.eos(self.cursor),
            "cursor must be within bounds"
        );

        let c = self.peek().next().expect("Checked end-of-source already");
        match c {
            '[' => Ok(self.emit_token(TokenKind::VecStart, 1)),
            ']' => Ok(self.emit_token(TokenKind::VecEnd, 1)),
            '(' => Ok(self.emit_token(TokenKind::ListStart, 1)),
            ')' => Ok(self.emit_token(TokenKind::ListEnd, 1)),
            '\'' => Ok(self.emit_token(TokenKind::Quote, 1)),
            '$' => self.lex_bind(),
            '^' => self.lex_load(),
            _ => Ok(self.lex_scalar().expect("Able to emit scalar")),
        }
    }
}
