//! Parser from token streams.
//!
//! This is the core routine which compiles a stream of [`Token`]s to
//! [`HirForm`]s.

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    interner::{Interner, SymId},
    lexer::{Token, TokenKind},
    parser::{HirForm, HirKind, ParseError, ParseErrorKind},
    source::{SourceId, SourceTable, Span, SyntaxId, SyntaxOrigin},
};

/// Parse the token stream of the source given by [`SourceId`] in a
/// [`SourceTable`], returning the body of [`HirForm`]s if no errors arise,
/// otherwise [`None`].
///
/// This constructs its own [`Diagnostics`] which is always passed back to the
/// caller.
///
/// `table` is taken mutably because every form registers its origin through
/// [`SourceTable::add_origin`].
#[must_use]
pub fn parse(
    source_id: SourceId,
    tokens: &[Token],
    table: &mut SourceTable,
    interner: &mut Interner,
) -> (Option<Vec<HirForm>>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let mut parser = Parser::new(source_id, table, interner, &mut diagnostics);
    for &token in tokens {
        parser.parse_singular(token);
    }
    parser.report_unclosed();
    let forms = parser.forms;
    ((!diagnostics.has_errors()).then_some(forms), diagnostics)
}

/// Parser state machine structure.
struct Parser<'a> {
    /// [`SourceId`] of the Source being parsed.
    source_id: SourceId,
    /// Source Table.
    table: &'a mut SourceTable,
    /// Symbol Interner.
    interner: &'a mut Interner,
    /// [`Diagnostics`] to accumulate in.
    diagnostics: &'a mut Diagnostics,
    /// Stack of [`Frame`]s used during parsing.
    stack: Vec<Frame>,
    /// Accumulation of [`HirForm`]s.
    forms: Vec<HirForm>,
}

impl<'a> Parser<'a> {
    /// Construct new parser state.
    const fn new(
        source_id: SourceId,
        table: &'a mut SourceTable,
        interner: &'a mut Interner,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            source_id,
            table,
            interner,
            diagnostics,
            stack: Vec::new(),
            forms: Vec::new(),
        }
    }

    /// Get the text of a given [`Span`].
    fn text_of(&self, span: Span) -> &str {
        self.table.text_of(&SyntaxOrigin {
            source: self.source_id,
            span,
        })
    }

    /// Mint a [`SyntaxOrigin`] for the given [`Span`].
    fn add_syntax(&mut self, span: Span) -> SyntaxId {
        self.table.add_origin(self.source_id, span)
    }

    /// Intern the contents of [`Span`].
    fn intern_span(&mut self, span: Span) -> SymId {
        let text = self.table.text_of(&SyntaxOrigin {
            source: self.source_id,
            span,
        });
        self.interner.intern(text)
    }

    /// Check if the top of the [`Frame`] stack is a datum.
    fn top_is_datum(&self) -> bool {
        self.stack.last().is_some_and(|f| f.is_datum)
    }

    /// Construct a new [`ParseError`] of kind [`ParseErrorKind`] at the given
    /// [`Span`].
    const fn error(&self, span: Span, kind: ParseErrorKind) -> ParseError {
        ParseError {
            origin: SyntaxOrigin {
                source: self.source_id,
                span,
            },
            kind,
        }
    }

    /// Report a [`ParseError`] into [`Self::diagnostics`].
    fn report(&mut self, error: ParseError) {
        self.diagnostics.push(Diagnostic::from(error));
    }

    /// Push a new [`Frame`] onto the [`Frame`] stack of [`FrameKind`] opening
    /// at a given [`Span`].
    fn push_frame(&mut self, kind: FrameKind, opening: Span) {
        self.stack
            .push(Frame::new(kind, opening, self.top_is_datum()));
    }

    /// Register a [`SyntaxOrigin`] for the given [`Frame`].
    ///
    /// `closing` is joined with the opening of the given [`Frame`], so the
    /// resultant form's origin spans the whole construct rather than just its
    /// opening token.
    fn close(
        &mut self,
        kind: HirKind,
        opening: Span,
        closing: Span,
    ) -> HirForm {
        let id = self.table.add_origin(self.source_id, opening.join(closing));
        HirForm::new(id, kind)
    }

    /// Close a frame using the given [`Token`] which should be a general
    /// container closer.
    ///
    /// If the [`TokenKind`] is not appropriate given the [`Frame`] stack, this
    /// will report a diagnostic regarding the specifics.
    fn close_frame(&mut self, token: Token) {
        debug_assert!(
            matches!(token.kind, TokenKind::VecEnd | TokenKind::ListEnd),
            "close_frame called with non closer token {token:?}"
        );

        let form = loop {
            // Try to get some frame from the frame stack so we can close it.
            let Some(Frame { kind, opening, .. }) = self.stack.pop() else {
                // If there's nothing on the stack, then this closer is
                // unexpected.  Report.
                self.report(
                    self.error(token.span, ParseErrorKind::UnexpectedCloser),
                );
                return;
            };

            let Some((expected, hirkind)) = kind.into_container() else {
                // This FrameKind is not a container, which means it's a quote.
                // Since the quote is still pending a child, this is an error.
                // Report, then loop again to catch a parent container.
                self.report(
                    self.error(opening, ParseErrorKind::QuoteWithoutForm),
                );
                continue;
            };

            // If we get the wrong closer for this container, report it, then
            // close the container up anyway so the rest of the parse can keep
            // catching errors.
            if expected != token.kind {
                let span = opening.join(token.span);
                self.report(self.error(span, ParseErrorKind::MismatchedCloser));
            }

            break self.close(hirkind, opening, token.span);
        };
        self.yield_form(form);
    }

    /// Yields `form` into the current parsing context.
    ///
    /// A container frame consumes the form and remains open. A quote frame
    /// consumes exactly one form, closes, and yields the resulting quote into
    /// its parent. Consequently, a single call may close several nested quote
    /// frames.
    ///
    /// If the receiving frame is in datum position, `Load` and `Bind` forms
    /// produce a diagnostic but are still deposited.
    fn yield_form(&mut self, mut form: HirForm) {
        loop {
            let Some(mut top) = self.stack.pop() else {
                self.forms.push(form);
                return;
            };

            // It's an error to have a Load or Bind HirForm in a datum frame.
            if top.is_datum
                && matches!(form.kind, HirKind::Load(_) | HirKind::Bind(_))
            {
                let span = self.table.get_origin(form.id).span;
                self.report(self.error(span, ParseErrorKind::BindingInDatum));
                // We still deposit this `form`.
            }

            match top.kind {
                FrameKind::Vector(ref mut forms)
                | FrameKind::List(ref mut forms) => {
                    forms.push(form);
                    self.stack.push(top);
                    return;
                }
                FrameKind::Quote => {
                    // NOTE: A quote is closed the moment a form yields into it.
                    // We need to yield this quote _again_ back into whatever
                    // parent frame it's a part of, so this forces a loop.
                    let origin = self.table.get_origin(form.id);

                    // We set `form` here so it can be yielded back into the
                    // parent.  NOTE: if an error is reported due to a Bind or
                    // Load, it only happens once as form becomes a
                    // HirKind::Quote.
                    form = self.close(
                        HirKind::Quote(Box::new(form)),
                        top.opening,
                        origin.span,
                    );
                }
            }
        }
    }

    /// Parse an integer.
    ///
    /// A literal that does not fit an `i64` reports a [`Diagnostic`], but is
    /// left in as a poison [`HirKind::Int`] of zero.
    ///
    /// Since upstream `parse` drops the parsed forms if there are any
    /// Diagnostics, there's no chance of this poison value being used in later
    /// compiler phases.
    fn parse_int(&mut self, token: Token) -> HirForm {
        debug_assert_eq!(
            token.kind,
            TokenKind::Number,
            "parse_int called with non number token {token:?}"
        );

        // FIXME(oreo)[2026-08-16 02:33]:: we're relying on the fact that the
        // tokeniser classifies TokenKind::Number as any sequence of -?[0-9]+.
        // This will break if and when that is no longer true.
        let parsed = str::parse::<i64>(self.text_of(token.span));
        let value = parsed.unwrap_or_else(|_| {
            self.report(self.error(token.span, ParseErrorKind::IntOverflow));
            0
        });

        HirForm::new(self.add_syntax(token.span), HirKind::Int(value))
    }

    /// Parse a symbolic-like [`Token`].
    ///
    /// `make` is used to construct the correct [`HirKind`] given the [`SymId`],
    /// and naturally induces strong assertions on the kind of [`HirForm`]s this
    /// function could produce.
    fn parse_sym_like(
        &mut self,
        token: Token,
        make: fn(SymId) -> HirKind,
    ) -> HirForm {
        debug_assert!(
            matches!(
                token.kind,
                TokenKind::Symbol | TokenKind::Bind | TokenKind::Load
            ),
            "parse_sym_like called with non symbolic token {token:?}"
        );

        let syntax_id = self.add_syntax(token.span);
        let sym_id = self.intern_span(token.symbol_span());
        HirForm::new(syntax_id, make(sym_id))
    }

    /// Parse a singular [`Token`].
    ///
    /// This essentially acts as the transition function for the [`Parser`]
    /// state machine, using the singular [`Token`] as input.
    fn parse_singular(&mut self, token: Token) {
        match token.kind {
            TokenKind::VecStart => {
                self.push_frame(FrameKind::Vector(Vec::new()), token.span);
            }
            TokenKind::ListStart => {
                self.push_frame(FrameKind::List(Vec::new()), token.span);
            }
            TokenKind::Quote => {
                // We've got a new quote despite there already being a quote on
                // the frame stack => error.
                if let Some(Frame {
                    kind: FrameKind::Quote,
                    opening,
                    ..
                }) = self.stack.last()
                {
                    let span = opening.join(token.span);
                    self.report(self.error(span, ParseErrorKind::NestedQuote));
                }
                self.push_frame(FrameKind::Quote, token.span);
            }
            TokenKind::Number => {
                let form = self.parse_int(token);
                self.yield_form(form);
            }
            TokenKind::Symbol => {
                let form = self.parse_sym_like(token, HirKind::Call);
                self.yield_form(form);
            }
            TokenKind::Bind => {
                let form = self.parse_sym_like(token, HirKind::Bind);
                self.yield_form(form);
            }
            TokenKind::Load => {
                let form = self.parse_sym_like(token, HirKind::Load);
                self.yield_form(form);
            }
            TokenKind::VecEnd | TokenKind::ListEnd => {
                self.close_frame(token);
            }
        }
    }

    /// Report errors from unclosed [`Frame`]s in the [`Frame`] stack.
    ///
    /// This should be called after the token stream has been completely parsed
    /// by the [`Parser`] state machine.  It catches stray frames and reports
    /// errors for them.
    ///
    /// This does clear the [`Frame`] stack afterwards.
    fn report_unclosed(&mut self) {
        // We need to completely take the stack away so we don't get a borrow
        // error when trying to report the errors later.
        let stack = std::mem::take(&mut self.stack);
        for frame in stack {
            let error_kind = match frame.kind {
                FrameKind::Vector(_) => ParseErrorKind::UnterminatedVector,
                FrameKind::List(_) => ParseErrorKind::UnterminatedList,
                FrameKind::Quote => ParseErrorKind::QuoteWithoutForm,
            };
            self.report(self.error(frame.opening, error_kind));
        }
    }
}

/// Types of [`Frame`]s.
#[derive(Debug)]
enum FrameKind {
    /// Vector.
    Vector(Vec<HirForm>),
    /// List.
    List(Vec<HirForm>),
    /// Quote.
    Quote,
}

impl FrameKind {
    /// Consume this frame kind as a container: the [`TokenKind`] that closes
    /// it, paired with the [`HirKind`] it becomes once closed.
    ///
    /// Returns None if [`FrameKind::Quote`] since that's obviously not a
    /// container.
    fn into_container(self) -> Option<(TokenKind, HirKind)> {
        match self {
            Self::Vector(cs) => Some((TokenKind::VecEnd, HirKind::Vector(cs))),
            Self::List(cs) => Some((TokenKind::ListEnd, HirKind::List(cs))),
            Self::Quote => None,
        }
    }
}

/// A frame which accumulates [`HirForm`]s.
#[derive(Debug)]
struct Frame {
    /// Type of Frame.
    kind: FrameKind,
    /// The Span this frame started on.
    opening: Span,
    /// Whether the frame is in datum position.  This propagates from the frame
    /// down to its children.
    is_datum: bool,
}

impl Frame {
    /// Construct a new [`Frame`].
    const fn new(
        kind: FrameKind,
        opening: Span,
        parent_is_datum: bool,
    ) -> Self {
        // '<x> and (<xs>) are both in datum position.
        let is_datum = parent_is_datum
            || matches!(kind, FrameKind::Quote | FrameKind::List(_));
        Self {
            kind,
            opening,
            is_datum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::{Class, Site},
        lexer::tokenise,
        parser::{dfs, print_forms},
    };

    /// A collection of test cases: a rendered form, or a diagnostic, paired
    /// with the text its span covers.
    type Spanned<T> = Vec<(T, String)>;

    /// One form's shape and payload, with its [`SyntaxId`] dropped.
    ///
    /// A reprint cannot reproduce the original's spans, so the round trip
    /// compares [`HirKind`] structure and interned [`SymId`] alone.
    #[derive(Debug, PartialEq, Eq)]
    enum Skeleton {
        /// [`HirKind::Int`]'s converted value.
        Int(i64),
        /// [`HirKind::Bind`]'s symbol.
        Bind(SymId),
        /// [`HirKind::Load`]'s symbol.
        Load(SymId),
        /// [`HirKind::Call`]'s symbol.
        Call(SymId),
        /// [`HirKind::Quote`], whose child follows it.
        Quote,
        /// [`HirKind::List`], whose children follow it.
        List,
        /// [`HirKind::Vector`], whose children follow it.
        Vector,
    }

    /// Every form in `forms` as a [`Skeleton`], pre-order, paired with depth.
    ///
    /// Pre-order plus depth determines a tree, so comparing two of these is
    /// structural equality.  Built on [`dfs`] to stay iterative.
    fn skeletons(forms: &[HirForm]) -> Vec<(Skeleton, usize)> {
        let mut out = Vec::new();
        dfs(forms, |form, depth| {
            let skeleton = match &form.kind {
                HirKind::Int(n) => Skeleton::Int(*n),
                HirKind::Bind(s) => Skeleton::Bind(*s),
                HirKind::Load(s) => Skeleton::Load(*s),
                HirKind::Call(s) => Skeleton::Call(*s),
                HirKind::Quote(_) => Skeleton::Quote,
                HirKind::List(_) => Skeleton::List,
                HirKind::Vector(_) => Skeleton::Vector,
            };
            out.push((skeleton, depth));
        });
        out
    }

    /// Render a form back to source-like text, ignoring [`SyntaxId`]s.
    ///
    /// [`HirForm`]'s [`PartialEq`] compares [`SyntaxId`]s, so a hand-written
    /// expected tree would compare identities rather than shape.  Rendering
    /// sidesteps that and reads as the program it came from.
    fn shape(form: &HirForm, interner: &Interner) -> String {
        match &form.kind {
            HirKind::Int(n) => n.to_string(),
            HirKind::Call(s) => interner.resolve(*s).to_string(),
            HirKind::Bind(s) => format!("${}", interner.resolve(*s)),
            HirKind::Load(s) => format!("^{}", interner.resolve(*s)),
            HirKind::Quote(f) => format!("'{}", shape(f, interner)),
            HirKind::List(fs) => format!("({})", shapes(fs, interner)),
            HirKind::Vector(fs) => format!("[{}]", shapes(fs, interner)),
        }
    }

    /// Render a body of forms, space separated.
    fn shapes(forms: &[HirForm], interner: &Interner) -> String {
        forms
            .iter()
            .map(|f| shape(f, interner))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Lex and parse `text` as its own source, resolving every span back to
    /// the text it covers.
    ///
    /// A diagnostic sited anywhere but [`Site::Raw`] resolves to a marker
    /// rather than its text, so it fails the comparison in the caller.
    fn parse_text(text: &str) -> (Option<Spanned<String>>, Spanned<Class>) {
        let mut table = SourceTable::new();
        let mut interner = Interner::new();
        let id = table
            .add_source_raw("t", text.into())
            .expect("within bound");
        let tokens = tokenise(id, &table).0.expect("lexes cleanly");
        let (forms, diags) = parse(id, &tokens, &mut table, &mut interner);

        // Test the compiler invariant of bijection between the count of forms
        // and tokens.
        if let Some(forms) = &forms {
            let expected = tokens
                .iter()
                .filter(|t| {
                    !matches!(t.kind, TokenKind::VecEnd | TokenKind::ListEnd)
                })
                .count();
            let mut got = 0usize;
            dfs(forms, |_, _| got += 1);
            assert_eq!(
                got, expected,
                "{text:?} yielded {got} forms from {expected} non-closing tokens"
            );
        }

        // Test the round trip forms -> [`parser::print`] -> reparse.
        if let Some(forms) = &forms {
            let mut printed = String::new();
            print_forms(forms, &interner, &mut printed)
                .expect("writing to a String cannot fail");

            let reprint = table
                .add_source_raw("t-reprint", printed.clone())
                .expect("within bound");
            let tokens = tokenise(reprint, &table)
                .0
                .expect("a printed body lexes cleanly");
            let (reparsed, _) =
                parse(reprint, &tokens, &mut table, &mut interner);
            let reparsed = reparsed.expect("a printed body parses cleanly");

            // Origins are excluded by construction: the reprint is a
            // different source, so every `SyntaxId` on the right differs even
            // when the trees agree.
            assert_eq!(
                skeletons(forms),
                skeletons(&reparsed),
                "{text:?} printed as {printed:?}, which parsed to a different tree"
            );
        }

        let source = table.get_source(id);
        let forms = forms.map(|forms| {
            forms
                .iter()
                .map(|form| {
                    let span = table.get_origin(form.id).span;
                    (shape(form, &interner), source.span_text(span).to_string())
                })
                .collect()
        });

        let diags = diags
            .items()
            .iter()
            .map(|diag| {
                let site = match diag.site {
                    Site::Raw(origin) => {
                        source.span_text(origin.span).to_string()
                    }
                    site => format!("<expected a Raw site, got {site:?}>"),
                };
                (diag.class, site)
            })
            .collect();
        (forms, diags)
    }

    /// Assert `text` parses to exactly `expected`, reporting nothing.
    fn assert_forms(text: &str, expected: &[&str]) {
        let (forms, diags) = parse_text(text);
        assert!(diags.is_empty(), "{text:?} unexpectedly raised {diags:?}");
        let forms = forms.expect("a clean parse yields a body");
        let got: Vec<_> = forms.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(got, expected, "parsing {text:?}");
    }

    /// Assert `text` parses cleanly to forms whose origins cover exactly the
    /// given text, as (rendered form, span text).
    fn assert_spans(text: &str, expected: &[(&str, &str)]) {
        let (forms, diags) = parse_text(text);
        assert!(diags.is_empty(), "{text:?} unexpectedly raised {diags:?}");
        let forms = forms.expect("a clean parse yields a body");
        let got: Vec<_> = forms
            .iter()
            .map(|(f, span)| (f.as_str(), span.as_str()))
            .collect();
        assert_eq!(got, expected, "parsing {text:?}");
    }

    /// Assert parsing `text` reports exactly `expected`, as (class, span
    /// text).
    fn assert_errors(text: &str, expected: &[(Class, &str)]) {
        let (forms, diags) = parse_text(text);
        let got: Vec<_> = diags.iter().map(|(c, s)| (*c, s.as_str())).collect();
        assert_eq!(got, expected, "parsing {text:?}");
        assert!(
            forms.is_none(),
            "{text:?} reported and still handed back a body"
        );
    }

    #[test]
    fn forms_and_shapes() {
        // Nothing begets nothing.
        assert_forms("", &[]);
        assert_forms("  \n; comment\n", &[]);

        // A body is a sequence, not a tree: several top-level forms stand
        // beside each other.
        assert_forms("12 -3 x", &["12", "-3", "x"]);
        assert_forms("$x ^y z", &["$x", "^y", "z"]);

        // Containers nest, and hold their children in order.
        assert_forms("[$x ^x]", &["[$x ^x]"]);
        assert_forms("(1 2)", &["(1 2)"]);
        assert_forms("[[1] (2 3)]", &["[[1] (2 3)]"]);
        assert_forms("[] ()", &["[]", "()"]);

        // A quote wraps exactly the next form, whatever its size.
        assert_forms("'x", &["'x"]);
        assert_forms("'[a b]", &["'[a b]"]);
        assert_forms("'(a b)", &["'(a b)"]);

        // A quote discharging inside a container must not take the container
        // with it: one pop too many during discharge drops the vector, and
        // this is where that shows.
        assert_forms("['x]", &["['x]"]);
        assert_forms("[a 'b c]", &["[a 'b c]"]);
        assert_forms("[['x]]", &["[['x]]"]);
    }

    #[test]
    fn quotes_separated_by_brackets() {
        // Nesting is direct adjacency only.  A quote separated from another
        // by a bracket is ordinary recursive denotation and stays legal.
        assert_forms("'['x]", &["'['x]"]);
        assert_forms("'('x)", &["'('x)"]);
        assert_forms("'[a 'b]", &["'[a 'b]"]);
    }

    #[test]
    fn spans_cover_whole_forms() {
        // A leaf covers its token, sigil included.
        assert_spans(" 12 ", &[("12", "12")]);
        assert_spans("$x", &[("$x", "$x")]);

        // A container spans from its opener to its closer, so the whitespace
        // and the brackets are inside the form's origin rather than beside
        // it.  This is what `opening.join(closing)` buys.
        assert_spans("[ a b ]", &[("[a b]", "[ a b ]")]);
        assert_spans("(  )", &[("()", "(  )")]);

        // A quote has no closer, so its origin runs from the apostrophe to
        // the end of the form that satisfied it.
        assert_spans("'x", &[("'x", "'x")]);
        assert_spans("'[a]", &[("'[a]", "'[a]")]);
    }

    #[test]
    fn binding_in_datum() {
        // A form beneath a Quote or a List is in datum position, and the
        // property passes inward and is never cleared.  Adjacency to `'` has
        // nothing to do with it: the last three carry no quote at all.
        for (text, sigil) in [
            ("'^x", "^x"),
            ("'[^x]", "^x"),
            ("'(a $x)", "$x"),
            ("(^x)", "^x"),
            ("([$y])", "$y"),
            ("[(^x)]", "^x"),
        ] {
            assert_errors(text, &[(Class::ParseBindingInDatum, sigil)]);
        }

        // A Vector is not datum position in its own right, so a binding in
        // one is ordinary code.
        assert_forms("[$x ^x]", &["[$x ^x]"]);
        assert_forms("[[$x]]", &["[[$x]]"]);
    }

    #[test]
    fn quote_errors() {
        // Direct adjacency, spanning both apostrophes.
        assert_errors("''x", &[(Class::ParseNestedQuote, "''")]);

        // One diagnostic per adjacent pair: three quotes are two nestings,
        // not one.
        assert_errors(
            "'''x",
            &[
                (Class::ParseNestedQuote, "''"),
                (Class::ParseNestedQuote, "''"),
            ],
        );

        // A quote with nothing to wrap, found by the drain.
        assert_errors("'", &[(Class::ParseQuoteWithoutForm, "'")]);
        assert_errors("x '", &[(Class::ParseQuoteWithoutForm, "'")]);

        // A closer arriving on a pending quote reports the quote and is then
        // retried against the parent, so the vector still closes cleanly.
        assert_errors("[']", &[(Class::ParseQuoteWithoutForm, "'")]);

        // With no parent to retry against, the closer is orphaned in its own
        // right.  Both diagnostics are true.
        assert_errors(
            "']",
            &[
                (Class::ParseQuoteWithoutForm, "'"),
                (Class::ParseUnexpectedCloser, "]"),
            ],
        );
    }

    #[test]
    fn bracket_errors() {
        // A closer with no open container at all.
        assert_errors("]", &[(Class::ParseUnexpectedCloser, "]")]);
        assert_errors("a )", &[(Class::ParseUnexpectedCloser, ")")]);

        // A wrong-kinded closer spans the whole construct, so the render
        // highlights both ends rather than naming them in prose.
        assert_errors("[a b)", &[(Class::ParseMismatchedCloser, "[a b)")]);
        assert_errors("(a]", &[(Class::ParseMismatchedCloser, "(a]")]);

        // Unclosed containers are found by the drain, one error each, every
        // one pointing at its own opener.
        assert_errors("[", &[(Class::ParseUnterminatedVector, "[")]);
        assert_errors("(", &[(Class::ParseUnterminatedList, "(")]);

        // The drain reports bottom-to-top, so openers arrive in source order
        // like every other diagnostic in the compiler.
        assert_errors(
            "([[",
            &[
                (Class::ParseUnterminatedList, "("),
                (Class::ParseUnterminatedVector, "["),
                (Class::ParseUnterminatedVector, "["),
            ],
        );

        // Recovery changes what the drain sees: the vector closes under the
        // mismatch and yields into the list, which is then left open.
        assert_errors(
            "([a b)",
            &[
                (Class::ParseMismatchedCloser, "[a b)"),
                (Class::ParseUnterminatedList, "("),
            ],
        );
    }

    #[test]
    fn integer_conversion() {
        // The bounds themselves fit.
        assert_forms("9223372036854775807", &["9223372036854775807"]);
        assert_forms("-9223372036854775808", &["-9223372036854775808"]);

        // One past either end does not.  Phase 1 classified these as Numbers
        // by shape alone, so the parser is the first phase that can tell.
        assert_errors(
            "9223372036854775808",
            &[(Class::ParseIntOverflow, "9223372036854775808")],
        );
        assert_errors(
            "-9223372036854775809",
            &[(Class::ParseIntOverflow, "-9223372036854775809")],
        );

        // A failed conversion yields no form, and the surrounding parse
        // carries on rather than unwinding.
        assert_errors(
            "[1 9223372036854775808 2]",
            &[(Class::ParseIntOverflow, "9223372036854775808")],
        );
    }
}
