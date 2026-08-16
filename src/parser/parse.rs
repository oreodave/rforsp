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

/// Parser state structure.
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

            match kind.close_with(token.kind) {
                // Happy path - this is the correct closer.
                Ok(hirkind) => break self.close(hirkind, opening, token.span),

                // If the top frame is a container and we get the wrong closer,
                // report then close it up anyway so the rest of the parse path
                // can keep catching errors.
                Err(FrameKind::Vector(cs)) => {
                    let span = opening.join(token.span);
                    self.report(
                        self.error(span, ParseErrorKind::MismatchedCloser),
                    );
                    break self.close(HirKind::Vector(cs), opening, token.span);
                }
                Err(FrameKind::List(cs)) => {
                    let span = opening.join(token.span);
                    self.report(
                        self.error(span, ParseErrorKind::MismatchedCloser),
                    );
                    break self.close(HirKind::List(cs), opening, token.span);
                }

                // We're trying to close on a quote which is still pending a
                // child - that's a separate error.  Report it, then loop again
                // to see if we can catch a parent container.
                Err(FrameKind::Quote) => {
                    self.report(
                        self.error(opening, ParseErrorKind::QuoteWithoutForm),
                    );
                }
            }
        };
        self.yield_form(form);
    }

    /// Yield the given `form` with respect to the current [`Frame`] stack.
    ///
    /// If the [`Frame`] stack is empty, `form` is simply pushed into the
    /// accumulated [`Self::forms`].  Otherwise, let `F` be the top of the
    /// [`Frame`] stack:
    /// - If `F` is a "container" ([`FrameKind::Vector`] or [`FrameKind::List`])
    ///   then the given `form` is simply added to the container's collection of
    ///   [`HirForm`]s.
    /// - If `F` is a quote then it is closed via [`Self::close`] (making a
    ///   [`HirKind::Quote`] form), and is then iteratively yielded into the
    ///   parent frame.
    ///
    /// Reports a [`ParseErrorKind::BindingInDatum`] if `F` is a datum frame and
    /// `form` is a [`HirKind::Load`] or [`HirKind::Bind`].  NOTE: The `form` is
    /// still deposited.
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

    /// Attempt to parse an integer.
    ///
    /// # Errors
    /// - If any error arises from attempting to parse the integer.
    fn parse_int(&mut self, token: Token) -> Result<HirForm, ParseError> {
        debug_assert_eq!(
            token.kind,
            TokenKind::Number,
            "parse_int called with non number token {token:?}"
        );

        let text = self.text_of(token.span);
        // FIXME(oreo)[2026-08-16 02:33]:: we're relying on the fact that the
        // tokeniser classifies TokenKind::Number as any sequence of -?[0-9]+.
        // This will break if and when that is no longer true.
        str::parse::<i64>(text)
            .map(|n| HirForm::new(self.add_syntax(token.span), HirKind::Int(n)))
            .map_err(|_| self.error(token.span, ParseErrorKind::IntOverflow))
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
            TokenKind::Number => match self.parse_int(token) {
                Ok(form) => self.yield_form(form),
                Err(e) => self.report(e),
            },
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
#[derive(Debug, PartialEq, Eq)]
enum FrameKind {
    /// Vector.
    Vector(Vec<HirForm>),
    /// List.
    List(Vec<HirForm>),
    /// Quote.
    Quote,
}

impl FrameKind {
    /// Close this frame kind with `closer`, yielding a [`HirKind`] if
    /// successful.
    ///
    /// # Errors
    /// - If the given `closer` token isn't appropriate for this [`FrameKind`].
    fn close_with(self, closer: TokenKind) -> Result<HirKind, Self> {
        match (self, closer) {
            (Self::Vector(cs), TokenKind::VecEnd) => Ok(HirKind::Vector(cs)),
            (Self::List(cs), TokenKind::ListEnd) => Ok(HirKind::List(cs)),
            (other, _) => Err(other),
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
    /// Is this frame supposed to accumulate datums?
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
