//! Parser from token streams.
//!
//! This is the core routine which compiles a stream of [`Token`]s to
//! [`HirForm`]s.

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    interner::Interner,
    lexer::Token,
    parser::{HirForm, HirKind, ParseError, ParseErrorKind},
    source::{SourceId, SourceTable, Span, SyntaxOrigin},
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
    /// Remaining tokens to parse.
    remtokens: &'a [Token],
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
        tokens: &'a [Token],
    ) -> Self {
        Self {
            source_id,
            table,
            interner,
            diagnostics,
            remtokens: tokens,
            stack: Vec::new(),
            forms: Vec::new(),
        }
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

    /// Report a [`ParseError`] at the given [`SyntaxOrigin`] of
    /// [`ParseErrorKind`].
    fn report_error(&mut self, origin: SyntaxOrigin, kind: ParseErrorKind) {
        self.diagnostics
            .push(Diagnostic::from(ParseError { origin, kind }));
    }

    /// Yield the given `form` with respect to the current [`Frame`] stack.
    ///
    /// If the [`Frame`] stack is empty, `form` is simply pushed into the
    /// accumulated `self.forms`.  Otherwise, let `F` be the top of the
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
    /// still desposited.
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
                let origin = *self.table.get_origin(form.id);
                self.report_error(origin, ParseErrorKind::BindingInDatum);
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
