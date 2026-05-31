//! WHATWG HTML §13.2.6 "Tree construction" — strict tree builder.
//!
//! Consumes the [`Tokenizer`] (A2) token stream and builds the document tree
//! directly in [`EcsDom`], one ECS entity per node, with no intermediate
//! representation (contrast: the tolerant compat path goes html5ever →
//! `RcDom` → `convert_document` walk → `EcsDom`). The current node's identity
//! *is* its ECS [`Entity`]; the stack of open elements is a `Vec<Entity>`.
//!
//! # Strict semantics (no error recovery)
//!
//! Every WHATWG HTML §13.2.2 parse error in the tree-construction stage
//! aborts with [`StrictParseError`] — there is no foster parenting
//! (§13.2.6.1), no adoption agency (§13.2.6.4.7), and no implicit
//! misnested-tag recovery. For fully-conforming HTML5 those branches are
//! unreachable, so the strict builder produces the spec tree for valid input
//! and rejects everything else at the first error.
//!
//! # Layering
//!
//! This module depends only on [`elidex_ecs`]. `EcsDom::*` calls are confined
//! to node marshalling (create / append / inspect); the tree-construction
//! algorithm itself lives entirely here, in the engine-independent parser
//! crate (CLAUDE.md Layering mandate).

mod implied_end;
mod insert;
mod modes;
mod parse_state;
mod reset_insertion_mode;
mod shadow;
mod text_only;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_html5lib_tree;

use elidex_ecs::{EcsDom, Entity};

use crate::result::ParseResult;
use crate::tokenizer::states::Tokenizer;
use crate::tokenizer::token::Token;
use crate::StrictParseError;

use parse_state::{InsertionMode, ParseState};

/// What the build loop should do after a mode handler processes a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    /// Advance to the next token.
    Next,
    /// Reprocess the same token (a mode handler switched the insertion mode
    /// and asked for the token to be re-dispatched — the spec's "reprocess
    /// the token" / "reprocess the current token").
    Reprocess,
    /// Stop parsing (the spec's "stop parsing", §13.2.7).
    Stop,
}

/// The strict tree builder: owns the tokenizer, the DOM under construction,
/// and the parse state, and drives WHATWG HTML §13.2.6 tree construction.
pub(crate) struct TreeBuilder {
    /// The token source (A2). The tree builder drives raw-text transitions on
    /// it via [`Tokenizer::set_state`].
    tokenizer: Tokenizer,
    /// The DOM under construction. Moved out into [`ParseResult`] on success.
    dom: EcsDom,
    /// The document root (parent of the `html` element).
    document: Entity,
    /// The mutable §13.2.4 parse state.
    state: ParseState,
    /// Buffer of consecutive character tokens awaiting coalescing into a
    /// single text node (§13.2.6.1 "insert a character"). Flushed before any
    /// non-character token is processed and on "stop parsing". See
    /// [`TreeBuilder::flush_text`].
    pending_text: String,
    /// The §13.2.6.4.10 "pending table character tokens" list, kept separate
    /// from [`Self::pending_text`] because the "in table text" mode must
    /// inspect the whole run for non-whitespace before deciding to insert it
    /// or (strict) reject.
    pending_table_text: String,
    /// The Document's "allow declarative shadow roots" flag (§13.2.6.4.4
    /// step 9). `true` for document parsing; A4 threads
    /// [`crate::ParseFragmentOptions::allow_declarative_shadow`] through here
    /// for fragment parsing.
    allow_declarative_shadow: bool,
}

impl TreeBuilder {
    /// Create a tree builder over `html` for document parsing.
    ///
    /// `allow_declarative_shadow` initializes the Document's "allow
    /// declarative shadow roots" flag (§13.2.6.4.4 step 9).
    fn new(html: &str, allow_declarative_shadow: bool) -> Self {
        let mut dom = EcsDom::new();
        let document = dom.create_document_root();
        TreeBuilder {
            tokenizer: Tokenizer::new(html),
            dom,
            document,
            state: ParseState::new(),
            pending_text: String::new(),
            pending_table_text: String::new(),
            allow_declarative_shadow,
        }
    }

    /// Parse `html` as a full document in strict mode (WHATWG HTML §13.2.6).
    ///
    /// Declarative shadow roots are allowed (the document-parse default). A4
    /// wires this through [`crate::parse_strict`]; A3 drives it from
    /// crate-internal tests only.
    pub(crate) fn build(html: &str) -> Result<ParseResult, StrictParseError> {
        TreeBuilder::new(html, true).run()
    }

    /// Parse `html` as a full document with an explicit "allow declarative
    /// shadow roots" flag. Test seam for declarative-shadow coverage.
    #[cfg(test)]
    pub(crate) fn build_with_declarative_shadow(
        html: &str,
        allow: bool,
    ) -> Result<ParseResult, StrictParseError> {
        TreeBuilder::new(html, allow).run()
    }

    /// The §13.2.6 tree-construction driver loop: pull tokens and dispatch
    /// each to the current insertion mode, reprocessing across mode switches,
    /// until a mode reaches "stop parsing".
    fn run(mut self) -> Result<ParseResult, StrictParseError> {
        loop {
            let token = self.tokenizer.next_token()?;
            // §13.2.6.4.7: a U+000A LINE FEED immediately following a
            // `<pre>` / `<listing>` / `<textarea>` start tag is dropped as an
            // authoring convenience. The flag is armed when those elements are
            // inserted and disarmed by the very next token, LF or not.
            if self.state.skip_next_lf {
                self.state.skip_next_lf = false;
                if matches!(token, Token::Character('\n')) {
                    continue;
                }
            }
            // §13.2.6.1 inserts characters as they are seen; the strict
            // builder buffers consecutive character tokens for text-node
            // coalescing (D-f) and flushes the buffer before any other token
            // is processed, so the text lands while the tree state still
            // reflects where those characters belong.
            if !matches!(token, Token::Character(_)) {
                self.flush_text();
            }
            loop {
                match self.dispatch(&token)? {
                    Flow::Next => break,
                    // Reprocess the same token: fall through to re-iterate the
                    // loop (a mode handler switched the insertion mode).
                    Flow::Reprocess => {}
                    Flow::Stop => {
                        // Any trailing character run was flushed above (EOF is
                        // not a character token); nothing left to coalesce.
                        return Ok(self.into_result());
                    }
                }
            }
        }
    }

    /// Dispatch `token` to the handler for the current insertion mode
    /// (§13.2.6.4 "The rules for parsing tokens in HTML content").
    fn dispatch(&mut self, token: &Token) -> Result<Flow, StrictParseError> {
        match self.state.mode {
            InsertionMode::Initial => modes::initial::initial(self, token),
            InsertionMode::BeforeHtml => modes::before_html::before_html(self, token),
            InsertionMode::BeforeHead => modes::before_head::before_head(self, token),
            InsertionMode::InHead => modes::in_head::in_head(self, token),
            InsertionMode::InHeadNoscript => modes::in_head_noscript::in_head_noscript(self, token),
            InsertionMode::AfterHead => modes::after_head::after_head(self, token),
            InsertionMode::InBody => modes::in_body::in_body(self, token),
            InsertionMode::Text => modes::text::text(self, token),
            InsertionMode::InTable => modes::in_table::in_table(self, token),
            InsertionMode::InTableText => modes::in_table_text::in_table_text(self, token),
            InsertionMode::InCaption => modes::in_caption::in_caption(self, token),
            InsertionMode::InColumnGroup => modes::in_column_group::in_column_group(self, token),
            InsertionMode::InTableBody => modes::in_table_body::in_table_body(self, token),
            InsertionMode::InRow => modes::in_row::in_row(self, token),
            InsertionMode::InCell => modes::in_cell::in_cell(self, token),
            InsertionMode::InTemplate => modes::in_template::in_template(self, token),
            InsertionMode::AfterBody => modes::after_body::after_body(self, token),
            InsertionMode::InFrameset => modes::in_frameset::in_frameset(self, token),
            InsertionMode::AfterFrameset => modes::after_frameset::after_frameset(self, token),
            InsertionMode::AfterAfterBody => modes::after_after_body::after_after_body(self, token),
            InsertionMode::AfterAfterFrameset => {
                modes::after_after_frameset::after_after_frameset(self, token)
            }
        }
    }

    /// Move the finished DOM out into a [`ParseResult`]. Strict success always
    /// reports an empty error list and no detected encoding (`&str` input).
    fn into_result(self) -> ParseResult {
        ParseResult {
            dom: self.dom,
            document: self.document,
            errors: Vec::new(),
            encoding: None,
        }
    }
}

/// Build a terminal [`StrictParseError`] naming the WHATWG HTML §13.2.2
/// parse-error condition `name`. Returned (not stored) — the first error
/// returned from a handler aborts the whole parse.
pub(super) fn parse_error(name: &str) -> StrictParseError {
    StrictParseError {
        errors: vec![name.to_string()],
    }
}
