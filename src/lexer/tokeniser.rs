//! Main tokeniser runtime

use crate::{
    diagnostics::Diagnostics,
    lexer::{LexError, LexErrorKind, Token, TokenKind, charclass::is_format},
    source::{Source, SourceId, SourceTable, Span, SyntaxOrigin},
};

/// Characters that separate scalars, and so CANNOT be part of one.
const RESTRICTED_CHARS: &str = "'^$()[];\r\n\t ";

/// Characters skipped between tokens.  A strict subset of [`RESTRICTED_CHARS`].
const WHITESPACE_CHARS: &str = "\r\n\t ";

/// Character introducing a comment, which runs to end of line.
const COMMENT_START: char = ';';

/// Compile time check that [`WHITESPACE_CHARS`] ⊂ [`RESTRICTED_CHARS`].
const _: () = {
    // TODO(oreo)[2026-08-11 00:06]: This rigamarole is only necessary because
    // iterating and `.contains` aren't const-stable yet in Rust.  Might be
    // worth looking back at this later.
    let whitespace = WHITESPACE_CHARS.as_bytes();
    let restricted = RESTRICTED_CHARS.as_bytes();
    let mut i = 0;
    while i < whitespace.len() {
        let mut found = false;
        let mut j = 0;
        while j < restricted.len() {
            if whitespace[i] == restricted[j] {
                found = true;
            }
            j += 1;
        }
        assert!(
            found,
            "WHITESPACE_CHARS must be a subset of RESTRICTED_CHARS"
        );
        i += 1;
    }
};

/// Check if a given [`char`] is a valid character to be part of a symbol.
///
/// Two classes are excluded beyond the restricted set, both because they
/// render as nothing: control characters (`Cc`) and format characters (`Cf`).
/// A name containing either would report back to the reader as a name they
/// cannot see.
fn is_valid_sym_char(c: char) -> bool {
    !RESTRICTED_CHARS.contains(c) && !c.is_control() && !is_format(c)
}

/// Check if the given [`&str`] contains only numeric digits, excluding a
/// possible sign at the start.
fn is_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Tokenise a source given by [`SourceId`] in a [`SourceTable`], returning the
/// token stream as a [`Vec<Token>`] if no errors arise, otherwise [`None`].
///
/// This constructs its own [`Diagnostics`] which is always passed back to the
/// caller.
#[must_use]
pub fn tokenise(
    source_id: SourceId,
    table: &SourceTable,
) -> (Option<Vec<Token>>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let mut tokeniser = Tokeniser::new(source_id, table, &mut diagnostics);
    let mut tokens = Vec::new();

    tokeniser.skip_trivia();
    while !tokeniser.source.eos(tokeniser.cursor) {
        match tokeniser.lex_singular() {
            Ok(token) => tokens.push(token),
            Err(error) => tokeniser.report(error),
        }
        tokeniser.skip_trivia();
    }

    let tokens = (!diagnostics.has_errors()).then_some(tokens);
    (tokens, diagnostics)
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
            // Phase 0 has already accounted for a leading byte order mark, so
            // every character from here on is program text.
            cursor: source.content_start(),
        }
    }

    /// Remaining text starting from `Tokeniser::cursor`.
    fn rest(&self) -> &str {
        &self.source.text()[self.cursor..]
    }

    /// The character at `Tokeniser::cursor`, or `None` at end-of-source.
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Byte length of the maximal run of `predicate`-satisfying characters
    /// starting at `Tokeniser::cursor`.
    fn run_len(&self, predicate: impl Fn(char) -> bool) -> usize {
        self.rest()
            .chars()
            .take_while(|&c| predicate(c))
            .map(char::len_utf8)
            .sum()
    }

    /// Skip whitespace and comments from the current cursor, stopping at the
    /// first character that begins a token.
    ///
    /// Trivia is liberal in what it swallows, deliberately.  Nothing within it
    /// becomes a name, a token or a span, so a character that could not appear
    /// in a symbol - a control character, say - is skipped happily here rather
    /// than reported.  The exclusions applied to symbol material exist to stop
    /// a diagnostic being corrupted by the very thing it names, and trivia
    /// names nothing, so applying them here would buy nothing.
    ///
    /// NOTE: We do NOT count `\r` as a valid newline starter, only `\n`.  A
    /// carriage return is simply counted as trivia.
    fn skip_trivia(&mut self) {
        loop {
            self.cursor += self.run_len(|c| WHITESPACE_CHARS.contains(c));
            if self.peek() == Some(COMMENT_START) {
                self.cursor += self.run_len(|c| c != '\n');
            } else {
                return;
            }
        }
    }

    /// Construct a new span of given length `len` starting at
    /// `Tokeniser::cursor`.
    const fn new_span(&self, len: usize) -> Span {
        Span::new(self.cursor, self.cursor + len)
    }

    /// Construct a [`LexError`] of the given kind over `span`.
    const fn error(&self, kind: LexErrorKind, span: Span) -> LexError {
        LexError {
            origin: SyntaxOrigin {
                source: self.source_id,
                span,
            },
            kind,
        }
    }

    /// Record a [`LexError`] as a diagnostic and carry on lexing.
    fn report(&mut self, error: LexError) {
        self.diagnostics.push(error.into());
    }

    /// Return a [`Token`] of the given [`TokenKind`] which is known to consist
    /// of exactly `len` bytes, advancing the cursor by that same number of
    /// bytes.
    const fn emit_token(&mut self, kind: TokenKind, len: usize) -> Token {
        let token = Token {
            kind,
            span: self.new_span(len),
        };
        self.cursor += len;
        token
    }

    /// Classify the scalar run at the cursor without consuming it, returning
    /// its [`TokenKind`] and byte length.
    ///
    /// Returns `None` when there is no run at all - the cursor sits on a
    /// restricted character or at end-of-source.
    fn scan_scalar(&self) -> Option<(TokenKind, usize)> {
        let len = self.run_len(is_valid_sym_char);
        if len == 0 {
            None
        } else if is_integer(&self.rest()[..len]) {
            Some((TokenKind::Number, len))
        } else {
            Some((TokenKind::Symbol, len))
        }
    }

    /// Lex a scalar value (either [`TokenKind::Number`] or
    /// [`TokenKind::Symbol`]).
    fn lex_scalar(&mut self) -> Option<Token> {
        let (kind, len) = self.scan_scalar()?;
        Some(self.emit_token(kind, len))
    }

    /// Attempt to lex an operator involving a binding - [`TokenKind::Bind`] or
    /// [`TokenKind::Load`] - whose sigil (`$` and `^` respectively) sits at the
    /// cursor.
    ///
    /// The sigil binds to the scalar IMMEDIATELY following it, which must be a
    /// [`TokenKind::Symbol`] - no whitespace.  The returned token spans the
    /// sigil and the symbol together.
    ///
    /// # Errors
    /// - `error_kind`, if the sigil is not followed by a symbol.
    fn lex_binding(
        &mut self,
        kind: TokenKind,
        error_kind: LexErrorKind,
    ) -> Result<Token, LexError> {
        let start = self.cursor;
        self.cursor += 1;

        match self.scan_scalar() {
            // Nothing follows to consume, so the sigil alone is the span.
            None => Err(self.error(error_kind, Span::new(start, self.cursor))),
            Some((ret_kind, len)) => {
                self.cursor += len;
                if ret_kind == TokenKind::Symbol {
                    // Good path!
                    Ok(Token {
                        kind,
                        span: Span::new(start, self.cursor),
                    })
                } else {
                    // If we get a non-symbol scalar, then we do want to bind it in the
                    // diagnostic span so it doesn't get re-used somewhere else.
                    Err(self.error(error_kind, Span::new(start, self.cursor)))
                }
            }
        }
    }

    /// Attempt to lex a singular [`Token`] from the current source.
    ///
    /// Assumes trivia has already been skipped, so the cursor sits on a
    /// character that we can actually lex.
    ///
    /// # Errors
    /// - Any [`LexError`] arising at this position.
    fn lex_singular(&mut self) -> Result<Token, LexError> {
        assert!(
            !self.source.eos(self.cursor),
            "cursor must be within bounds"
        );

        let c = self.peek().expect("Checked end-of-source already");
        match c {
            '[' => Ok(self.emit_token(TokenKind::VecStart, 1)),
            ']' => Ok(self.emit_token(TokenKind::VecEnd, 1)),
            '(' => Ok(self.emit_token(TokenKind::ListStart, 1)),
            ')' => Ok(self.emit_token(TokenKind::ListEnd, 1)),
            '\'' => Ok(self.emit_token(TokenKind::Quote, 1)),
            '$' => self.lex_binding(TokenKind::Bind, LexErrorKind::BindInvalid),
            '^' => self.lex_binding(TokenKind::Load, LexErrorKind::LoadInvalid),
            _ => self.lex_scalar().ok_or_else(|| {
                // Worst path possible; nothing from the above was able to bind
                // and we couldn't even get a symbol out of it.

                // Since we want to accumulate errors though, we should try and
                // skip just this character and see what else we could lex.
                let span = self.new_span(c.len_utf8());
                self.cursor += c.len_utf8();
                self.error(LexErrorKind::UnknownCharacter, span)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Class, Site};
    use TokenKind::{
        Bind, ListEnd, ListStart, Load, Number, Quote, Symbol, VecEnd, VecStart,
    };

    /// A collection of test cases: token, or a diagnostic, paired with the text
    /// its span should cover.
    type Spanned<T> = Vec<(T, String)>;

    /// Lex `text` as its own source, resolving every span back to the text it
    /// covers.
    ///
    /// A diagnostic sited anywhere but [`Site::Raw`] resolves to a marker
    /// rather than its text, so it fails the comparison in the caller.
    fn lex(text: &str) -> (Spanned<TokenKind>, Spanned<Class>) {
        let mut table = SourceTable::new();
        let id = table
            .add_source_raw("t", text.into())
            .expect("within bound");
        let (tokens, diags) = tokenise(id, &table);
        let source = table.get_source(id);
        let text_of = |span| source.span_text(span).to_string();

        let tokens = tokens
            .unwrap_or_default()
            .iter()
            .map(|t| (t.kind, text_of(t.span)))
            .collect();

        let diags = diags
            .items()
            .iter()
            .map(|diag| {
                let site = match diag.site {
                    Site::Raw(origin) => text_of(origin.span),
                    site => format!("<expected a Raw site, got {site:?}>"),
                };
                (diag.class, site)
            })
            .collect();
        (tokens, diags)
    }

    /// Assert `text` lexes to exactly `expected`, reporting nothing.
    fn assert_tokens(text: &str, expected: &[(TokenKind, &str)]) {
        let (tokens, diags) = lex(text);
        assert!(diags.is_empty(), "{text:?} unexpectedly raised {diags:?}");
        let got: Vec<_> =
            tokens.iter().map(|(k, s)| (*k, s.as_str())).collect();
        assert_eq!(got, expected, "tokenising {text:?}");
    }

    /// Assert lexing `text` reports exactly `expected`, as (class, span text).
    fn assert_errors(text: &str, expected: &[(Class, &str)]) {
        let (_, diags) = lex(text);
        let got: Vec<_> = diags.iter().map(|(c, s)| (*c, s.as_str())).collect();
        assert_eq!(got, expected, "tokenising {text:?}");
    }

    #[test]
    fn tokens_and_spans() {
        // Empty and whitespace begets empty
        assert_tokens("", &[]);
        assert_tokens("  \n\t ", &[]);

        // All the one-byte tokens should parse 1-1
        assert_tokens(
            "[](')",
            &[
                (VecStart, "["),
                (VecEnd, "]"),
                (ListStart, "("),
                (Quote, "'"),
                (ListEnd, ")"),
            ],
        );

        // Symbols are contiguous
        assert_tokens(
            "abc[def]",
            &[
                (Symbol, "abc"),
                (VecStart, "["),
                (Symbol, "def"),
                (VecEnd, "]"),
            ],
        );

        // Bind and Load eat up the next symbol.
        assert_tokens("$x ^y", &[(Bind, "$x"), (Load, "^y")]);

        // Complex symbol construction with whitespace
        assert_tokens(
            concat!("\t∀x:\n", "\tx≡0mod2\n", "⇔\n", "\t∃k:\n", "\tx=2k"),
            &[
                (Symbol, "∀x:"),
                (Symbol, "x≡0mod2"),
                (Symbol, "⇔"),
                (Symbol, "∃k:"),
                (Symbol, "x=2k"),
            ],
        );
    }

    #[test]
    fn integers_vs_symbols() {
        // A run is a Number only if it matches `-?[0-9]+` entirely; a digit
        // merely occurring in the run is not enough.
        for (text, kind) in [
            ("12", Number),
            ("0", Number),
            ("-12", Number),
            ("-", Symbol),
            ("1abc", Symbol),
            ("abc1", Symbol),
            ("12-", Symbol),
        ] {
            assert_tokens(text, &[(kind, text)]);
        }
    }

    #[test]
    fn trivia_skipped() {
        // Empty is empty
        assert_tokens(";", &[]);

        // Newlines separate the herd
        assert_tokens(
            concat!("a ; comment [ $ ) \n", "b"),
            &[(Symbol, "a"), (Symbol, "b")],
        );
        assert_tokens(
            concat!(";one\n", " ;two\n", "\n", ";three\n", "a"),
            &[(Symbol, "a")],
        );

        // End of file doesn't matter for comments.
        assert_tokens("a ;trailing", &[(Symbol, "a")]);

        // A CRLF source lexes exactly as its LF twin does.
        assert_tokens("a\r\nb", &[(Symbol, "a"), (Symbol, "b")]);

        // CR is trivia in its own right, not half of a terminator, so a lone
        // CR separates tokens too.  Being restricted, it ends a run rather
        // than joining one.
        assert_tokens("$x\r^y", &[(Bind, "$x"), (Load, "^y")]);

        // A comment runs to the LF, so a CR sitting before one is swallowed
        // by the comment rather than reported.
        assert_tokens("a ;comment\r\nb", &[(Symbol, "a"), (Symbol, "b")]);

        // NEL is Unicode `White_Space` but is not one of the four recognised
        // characters, so it is an error rather than trivia.  This is what the
        // carve-out being a fixed set - rather than `!is_whitespace` - buys.
        assert_errors("a\u{85}b", &[(Class::LexUnknownCharacter, "\u{85}")]);
    }

    #[test]
    fn control_characters_are_errors() {
        // A control character is not symbol material.  It is reported on its
        // own one-character span.
        // The exclusion is Unicode `Cc` rather than ASCII, so C1 (`\u{80}`
        // through `\u{9f}`) reports exactly as C0 and DEL do.
        for control in [
            '\u{0}', '\u{b}', '\u{c}', '\u{1b}', '\u{7f}', '\u{80}', '\u{9f}',
        ] {
            let text = format!("a{control}b");
            let control = control.to_string();
            assert_errors(&text, &[(Class::LexUnknownCharacter, &control)]);
        }

        // Each one is its own diagnostic, so they accumulate.
        assert_errors(
            "\u{0}\u{7f}",
            &[
                (Class::LexUnknownCharacter, "\u{0}"),
                (Class::LexUnknownCharacter, "\u{7f}"),
            ],
        );

        // The exclusion is control characters MINUS the recognised
        // whitespace; `\n` and `\t` are both, and stay trivia.
        assert_tokens(
            "a\tb\nc",
            &[(Symbol, "a"), (Symbol, "b"), (Symbol, "c")],
        );

        // Comments are not symbols, so nothing renders back to the user out of
        // one, and it stays liberal in what it swallows.
        assert_tokens(
            concat!("a ;com\u{0}ment\n", "b"),
            &[(Symbol, "a"), (Symbol, "b")],
        );

        // Format characters are excluded on the same grounds, so they report
        // identically.  A BOM is the mundane way in - an editor writes one and
        // the name it lands in renders as nothing - and the bidirectional
        // overrides are the adversarial way.
        for format in ['\u{feff}', '\u{200b}', '\u{200d}', '\u{202e}'] {
            let text = format!("a{format}b");
            let format = format.to_string();
            assert_errors(&text, &[(Class::LexUnknownCharacter, &format)]);
        }
    }

    #[test]
    fn leading_byte_order_mark() {
        // Phase 0 accounts for a leading mark, so lexing never sees one: a
        // marked source yields exactly what its unmarked twin does.  Spans
        // stay file-absolute and so differ by the mark's three bytes, which is
        // why this compares the text they cover rather than the offsets.
        let program = "[$x ^x] 'a 12";
        assert_eq!(lex(&format!("\u{feff}{program}")), lex(program));

        // Only the first is consumed, so a second is program text and is
        // rejected as the format character it is.
        assert_errors(
            "\u{feff}\u{feff}x",
            &[(Class::LexUnknownCharacter, "\u{feff}")],
        );

        // A source that is only a mark has no program text, which is an empty
        // token stream rather than an error.
        assert_tokens("\u{feff}", &[]);
    }

    #[test]
    fn binding_operators() {
        // Any non symbol scalar operand gets consumed in the diagnostic.
        assert_errors("$12", &[(Class::LexBindInvalid, "$12")]);
        assert_errors("^12", &[(Class::LexLoadInvalid, "^12")]);

        // If there's no scalar at all, then the `$` is the only thing that
        // matters.
        for text in ["$", "$ x", "$\n", "$;comment", "$["] {
            let (_, diags) = lex(text);
            assert_eq!(diags, vec![(Class::LexBindInvalid, "$".to_string())]);
        }

        // Invalid binds/loads accumulate.
        assert_errors(
            "$1 ^2 $3",
            &[
                (Class::LexBindInvalid, "$1"),
                (Class::LexLoadInvalid, "^2"),
                (Class::LexBindInvalid, "$3"),
            ],
        );
    }

    #[test]
    fn lex_recovery() {
        // Consider these two diagnostics:
        assert_errors("$12", &[(Class::LexBindInvalid, "$12")]);
        assert_errors("$[xyz]", &[(Class::LexBindInvalid, "$")]);

        // If both of these errors are present in a stream, we get two
        // diagnostics:
        assert_errors(
            "$12 $[xyz]",
            &[(Class::LexBindInvalid, "$12"), (Class::LexBindInvalid, "$")],
        );

        // This is the same for any other type of error we pick - we can mix and
        // match:
        assert_errors(
            "$a\0b ^[x y z]",
            &[
                (Class::LexUnknownCharacter, "\0"),
                (Class::LexLoadInvalid, "^"),
            ],
        );
    }

    #[test]
    fn sigil_tokens_always_carry_a_name() {
        // `Token::symbol_span` steps one byte past the sigil, and
        // `Span::from_u32` asserts an ordered range, so a `Bind`/`Load`
        // spanning its sigil alone would panic.  That is unreachable only
        // because a bare sigil is an error and never becomes a token; this
        // pins the invariant at the producer.
        for text in ["$x ^y", "^^x", "$a$b", "$ ^", "^^", "$", "'$x", "[$x]"] {
            let (tokens, _) = lex(text);
            for (kind, span) in
                tokens.iter().filter(|(k, _)| matches!(*k, Bind | Load))
            {
                assert!(
                    span.len() >= 2,
                    "{text:?} produced a {kind:?} spanning {span:?}, \
                     which has no name after its sigil"
                );
            }
        }
    }

    #[test]
    fn adjacent_sigils() {
        // Adjacent sigils will always report an error
        assert_errors("^^x", &[(Class::LexLoadInvalid, "^")]);

        // Generally binding will fail the moment the next character isn't part
        // of a symbol.
        assert_errors(
            "^^ $$",
            &[
                (Class::LexLoadInvalid, "^"),
                (Class::LexLoadInvalid, "^"),
                (Class::LexBindInvalid, "$"),
                (Class::LexBindInvalid, "$"),
            ],
        );
    }
}
