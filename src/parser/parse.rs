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
#[expect(clippy::todo, unused_variables, reason = "phase 2 stub")]
pub fn parse(
    source_id: SourceId,
    tokens: &[Token],
    table: &mut SourceTable,
    interner: &mut Interner,
) -> (Option<Vec<HirForm>>, Diagnostics) {
    todo!("phase 2: the container stack")
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
        frame: &Frame,
        kind: HirKind,
        closing: Span,
    ) -> HirForm {
        let id = self
            .table
            .add_origin(self.source_id, frame.opening.join(closing));
        HirForm::new(id, kind)
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
                        &top,
                        HirKind::Quote(Box::new(form)),
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
            TokenKind::Quote => self.push_frame(FrameKind::Quote, token.span),
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
            TokenKind::VecEnd => {
                todo!()
            }
            TokenKind::ListEnd => {
                todo!()
            }
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
