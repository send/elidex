//! WHATWG HTML §13.2.6 "Tree construction" — strict tree builder.
//!
//! Consumes the [`Tokenizer`] token stream and builds the document tree
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

mod foreign_adjust;
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
mod tests_foreign;
#[cfg(test)]
mod tests_fragment;
#[cfg(test)]
mod tests_html5lib_tree;

use elidex_ecs::{Attributes, EcsDom, Entity, Namespace};

use crate::result::{ParseFragmentOptions, ParseResult, ParseTier};
use crate::tokenizer::states::{State, Tokenizer};
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
    /// The token source. The tree builder drives raw-text transitions on
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
    /// step 9). `true` for document parsing (always allowed); for §13.4
    /// fragment parsing it carries
    /// [`crate::ParseFragmentOptions::allow_declarative_shadow`]
    /// (HTML §13.4 step 6).
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
    /// Declarative shadow roots are allowed (the document-parse default).
    /// Wired through [`crate::parse_strict`].
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

    /// Parse `html` as an HTML fragment in the given `context` element's
    /// context (WHATWG HTML §13.4 "Parsing HTML fragments"), returning the
    /// fragment's top-level nodes **detached** (parentless) in `dom` — the
    /// spec's "return root's children" (step 20). The caller places them.
    ///
    /// `context` is read-only (its tag, namespace, and ancestor chain select
    /// the tokenizer state, insertion mode, and form pointer); it is never
    /// mutated. On a parse error the partial subtree is torn down and `dom` is
    /// left pristine, so a strict-then-tolerant dispatcher can fall back over
    /// an uncontaminated dom. Wired through [`crate::parse_fragment_strict`].
    pub(crate) fn build_fragment(
        html: &str,
        dom: &mut EcsDom,
        context: Entity,
        opts: ParseFragmentOptions,
    ) -> Result<Vec<Entity>, StrictParseError> {
        // The tree builder owns its `EcsDom`, but fragment parsing must build
        // into the caller's live dom (where `context` and its ancestors live).
        // Move the caller's dom into the builder and back out afterwards — a
        // behaviour-preserving ownership shuffle that keeps construction
        // identical to document parsing (only the dom source differs) without
        // threading a lifetime through every mode handler. The placeholder
        // `EcsDom::new()` is dropped on the move-back.
        let owned = std::mem::replace(dom, EcsDom::new());
        let (result, returned) = Self::run_fragment(html, owned, context, opts);
        *dom = returned;
        result
    }

    /// Body of [`Self::build_fragment`]: set up the §13.4 fragment initial
    /// conditions on a builder that owns `dom`, drive tree construction, and
    /// return both the result and the (moved) dom.
    fn run_fragment(
        html: &str,
        mut dom: EcsDom,
        context: Entity,
        opts: ParseFragmentOptions,
    ) -> (Result<Vec<Entity>, StrictParseError>, EcsDom) {
        // The whole §13.4 build happens on a synthetic throwaway document, never
        // the caller's connected tree, so suppress mutation dispatch for its
        // duration: appending the synthetic root under a `Document` would
        // otherwise fire insert/remove events (`is_connected` treats any
        // `Document` root as connected), letting custom-element / observer /
        // Range consumers react to internal fragment nodes the caller has not
        // yet placed — and observe their teardown. The caller's own placement
        // of the returned detached nodes fires the real events; restored below.
        let saved_dispatcher = dom.take_mutation_dispatcher();
        // §13.4 steps 2 + 11-13: a throwaway Document holding a single
        // synthetic `<html>` root, which is the sole entry on the stack of
        // open elements. The fragment's nodes are the root's children and are
        // returned detached (step 20); the document + root are torn down. The
        // Document (not the bare element) is the root's owner so that foreign
        // (SVG / MathML) elements created during the parse get a valid owner
        // document (§13.2.6.1) — `create_element_ns` requires a Document owner.
        // A throwaway Document — created cache-free (`create_document_node`,
        // NOT `create_document_root`) so it never clobbers the caller's
        // persistent `document_root` cache, which it would then leave dangling
        // when despawned.
        let document = dom.create_document_node();
        let root = dom.create_element("html", Attributes::default());
        debug_assert!(
            dom.append_child(document, root),
            "appending a fresh root to a fresh document cannot fail"
        );
        let mut state = ParseState::new();
        state.open_elements.push(root);
        // §13.4 step 16 substitution source (consumed by
        // `reset_insertion_mode_appropriately`).
        state.fragment_context = Some(context);
        // §13.4 step 14: a `template` context seeds the template insertion
        // mode stack so a `</template>` / table reset resolves correctly. The
        // spec's `template` reference is HTML-namespace-only, so an SVG/MathML
        // element whose local name is `template` does not seed it.
        if dom.namespace_of(context) == Namespace::Html && dom.has_tag(context, "template") {
            state.template_modes.push(InsertionMode::InTemplate);
        }
        let mut tb = TreeBuilder {
            tokenizer: Tokenizer::new(html),
            dom,
            document,
            state,
            pending_text: String::new(),
            pending_table_text: String::new(),
            allow_declarative_shadow: opts.allow_declarative_shadow,
        };
        tb.set_fragment_tokenizer_state(context); // §13.4 step 10
        tb.reset_insertion_mode_appropriately(); // §13.4 step 16
        tb.set_fragment_form_pointer(context); // §13.4 step 17
        let result = match tb.drive() {
            Ok(()) => Ok(tb.take_fragment_children(root)),
            Err(err) => {
                // Rollback. Any consumed declarative-shadow template still on
                // the stack (its `</template>` never arrived) is stack-only —
                // not reachable from `document` — so despawn those first, then
                // tear the whole throwaway document subtree out. The caller's
                // dom is left with no orphaned live entities.
                let stack_only_templates: Vec<Entity> = tb
                    .state
                    .open_elements
                    .iter()
                    .copied()
                    .filter(|e| tb.state.template_content_targets.contains_key(e))
                    .collect();
                for template in stack_only_templates {
                    let _ = tb.dom.destroy_entity(template);
                }
                let _ = tb.dom.despawn_subtree(document);
                Err(err)
            }
        };
        // Restore the suppressed dispatcher before handing the dom back, so the
        // caller's placement of the returned detached nodes fires events.
        if let Some(dispatcher) = saved_dispatcher {
            tb.dom.set_mutation_dispatcher(dispatcher);
        }
        (result, tb.dom)
    }

    /// §13.4 step 20: return the synthetic root's children — detached
    /// (parentless) live nodes — in tree order, then tear down the throwaway
    /// document + root.
    ///
    /// `destroy_entity` orphans a node's children (clears their parent/sibling
    /// links, leaving them live) before despawning the node itself, so
    /// destroying the root *is* the detach: the children survive parentless and
    /// the root is gone. The now-childless document is despawned after.
    fn take_fragment_children(&mut self, root: Entity) -> Vec<Entity> {
        // DOM §4.5 "adopt": every returned node's node document is the context's
        // (not just foreign elements — HTML elements / text / comments resolve
        // `ownerDocument` via the tree root, which is the throwaway document
        // about to be despawned, so without this re-home they would dangle /
        // resolve to `None`). Re-home the whole subtree before tearing the
        // throwaway document down.
        if let Some(doc) = self.fragment_document() {
            self.dom.adopt_subtree(root, doc);
        }
        // Uncapped: `EcsDom::children` caps the sibling walk at
        // `MAX_ANCESTOR_DEPTH`, which would drop the tail of a fragment with
        // very many top-level nodes — and `destroy_entity(root)` then orphans
        // those dropped children as live, unreachable entities in the caller's
        // dom, violating both §13.4 step 20 ("return root's children") and the
        // no-leak isolation contract.
        let children = self.dom.child_list_uncapped(root);
        let _ = self.dom.destroy_entity(root);
        let _ = self.dom.destroy_entity(self.document);
        children
    }

    /// The context element's node document (WHATWG DOM `ownerDocument`) — the
    /// document the §13.4 fragment's returned nodes are adopted into. `None`
    /// only when the context is itself documentless (no live owner to re-home
    /// to). Document parsing has no fragment context, so this is `None` there.
    fn fragment_document(&self) -> Option<Entity> {
        self.state
            .fragment_context
            .and_then(|ctx| self.dom.owner_document(ctx))
    }

    /// §13.4 step 10: switch the tokenizer's initial state from the context
    /// element's tag — `title`/`textarea` → RCDATA, `style`/`xmp`/`iframe`/
    /// `noembed`/`noframes`/`noscript` → RAWTEXT (`noscript` because v1
    /// scripting is enabled, i.e. scriptingMode ≠ Disabled), `script` →
    /// script data, `plaintext` → PLAINTEXT, anything else → Data (left as-is).
    ///
    /// No "last start tag" is recorded, so per the §13.4 note there is no
    /// appropriate end tag in the fragment case: raw-text content runs to EOF
    /// (e.g. a `title`-context `</title>` is literal text, not a close).
    fn set_fragment_tokenizer_state(&mut self, context: Entity) {
        // §13.4 step 10's element-name cases are HTML elements — unqualified
        // element-type references in the spec are in the HTML namespace. A
        // foreign context whose local name happens to be `title` / `style` /
        // `script` / … (e.g. an SVG `<title>`) therefore does NOT switch to a
        // raw-text state; its children are parsed by the foreign-content rules,
        // so the tokenizer stays in the Data state.
        if self.dom.namespace_of(context) != Namespace::Html {
            return;
        }
        let state = self.dom.with_tag_name(context, |tag| match tag {
            Some("title" | "textarea") => State::Rcdata,
            Some("style" | "xmp" | "iframe" | "noembed" | "noframes" | "noscript") => {
                State::Rawtext
            }
            Some("script") => State::ScriptData,
            Some("plaintext") => State::Plaintext,
            _ => State::Data,
        });
        if !matches!(state, State::Data) {
            self.tokenizer.set_state(state);
        }
    }

    /// §13.4 step 17: set the form element pointer to the nearest `form`
    /// element on the context element's inclusive ancestor chain, if any.
    fn set_fragment_form_pointer(&mut self, context: Entity) {
        let mut node = Some(context);
        while let Some(entity) = node {
            // Form association is tree-scoped: do not cross a shadow boundary.
            // `get_parent` returns a `ShadowRoot`'s host (shadow-inclusive
            // ancestry, §13.4 step 17's walk is the DOM ancestor chain), so
            // stop at the shadow root — otherwise an outer light-DOM `<form>`
            // would seed the pointer for a shadow-tree context and make an
            // otherwise-valid `<form>` in the shadow fragment strict-reject.
            if self.dom.is_shadow_root(entity) {
                return;
            }
            // §13.4 step 17's `form` reference is HTML-namespace-only, so a
            // foreign element whose local name is `form` is not a form ancestor.
            if self.dom.namespace_of(entity) == Namespace::Html && self.dom.has_tag(entity, "form")
            {
                self.state.form_pointer = Some(entity);
                return;
            }
            node = self.dom.get_parent(entity);
        }
    }

    /// Whether this parse was created for the §13.4 HTML fragment parsing
    /// algorithm (the spec's "fragment case"). The canonical predicate for a
    /// mode handler whose fragment-case branch is not already distinguishable
    /// by stack shape — currently the "after body" `</html>` rule
    /// ([`modes::after_body`]), which rejects in the fragment case where
    /// document parsing transitions to "after after body". (Some fragment-case
    /// rules the spec phrases as stack-shape tests, e.g. "in frameset"'s
    /// "current node is the root html element", stay expressed that way.)
    pub(super) fn is_fragment(&self) -> bool {
        self.state.fragment_context.is_some()
    }

    /// Whether this is the §13.4 fragment case AND the context element is a
    /// `select` — the condition the "in body" insertion mode tests on
    /// `<input>` / `<select>` start tags (§13.2.6.4.7, post customizable-`select`:
    /// the old "in select" mode was folded into "in body" fragment-case
    /// branches). In a select-context fragment the select is the context, not on
    /// the stack, so the stack-shape `has_tag_in_scope("select")` test alone
    /// misses it.
    pub(super) fn fragment_context_is_select(&self) -> bool {
        self.state
            .fragment_context
            .is_some_and(|ctx| self.dom.has_tag(ctx, "select"))
    }

    /// Parse a full document: drive tree construction to "stop parsing", then
    /// move the finished tree into a [`ParseResult`].
    fn run(mut self) -> Result<ParseResult, StrictParseError> {
        self.drive()?;
        Ok(self.into_result())
    }

    /// The §13.2.6 tree-construction driver loop: pull tokens and dispatch
    /// each to the current insertion mode, reprocessing across mode switches,
    /// until a mode reaches "stop parsing" (returns `Ok(())`).
    ///
    /// Shared by document parsing ([`Self::run`]) and §13.4 fragment parsing
    /// ([`Self::build_fragment`]); only what each does with the finished tree
    /// differs (a [`ParseResult`] vs. the fragment root's detached children),
    /// not how tokens are driven.
    fn drive(&mut self) -> Result<(), StrictParseError> {
        loop {
            // §13.2.5.42 tokenizer feedback: mirror the adjusted current
            // node's namespace to the tokenizer so it recognizes `<![CDATA[`
            // only inside foreign content. Synced once per token here — the
            // single chokepoint before the next token is tokenized — rather
            // than at every stack mutation (D-fc-g).
            self.sync_foreign_content_flag();
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
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Dispatch `token` to the handler for the current insertion mode
    /// (§13.2.6.4 "The rules for parsing tokens in HTML content").
    ///
    /// The §13.2.6 tree-construction dispatcher first decides, per token,
    /// whether the adjusted current node's namespace + integration points
    /// route to the foreign-content rules (§13.2.6.5); that branch runs
    /// before the HTML-content insertion-mode match below.
    fn dispatch(&mut self, token: &Token) -> Result<Flow, StrictParseError> {
        if modes::foreign::in_foreign_content(self, token) {
            return modes::foreign::foreign_content(self, token);
        }
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

    /// Set the tokenizer's "foreign content" flag to whether the adjusted
    /// current node is in a non-HTML namespace (§13.2.5.42 + D-fc-g). When
    /// set, the markup-declaration-open state opens a CDATA section for
    /// `<![CDATA[`; otherwise that sequence is a parse error. The adjusted
    /// current node is the current node for document parsing.
    fn sync_foreign_content_flag(&mut self) {
        let foreign = self
            .state
            .adjusted_current_node()
            .is_some_and(|node| self.dom.namespace_of(node) != Namespace::Html);
        self.tokenizer.set_foreign_content_flag(foreign);
    }

    /// Move the finished DOM out into a [`ParseResult`]. Strict success always
    /// reports an empty error list, no detected encoding (`&str` input), and
    /// [`ParseTier::Clean`] — strict only reaches this point on conforming
    /// HTML5 (the `§11.3` Tier-1 happy path).
    fn into_result(self) -> ParseResult {
        ParseResult {
            dom: self.dom,
            document: self.document,
            errors: Vec::new(),
            encoding: None,
            tier: ParseTier::Clean,
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

/// Build a terminal [`StrictParseError`] for a §13.4 fragment construct strict
/// does not yet faithfully support — a non-HTML-namespace context element, or a
/// declarative shadow root that would attach to the external context. Like a
/// [`parse_error`] it aborts the parse, but it signals "strict declines, fall
/// back to the tolerant backend", not a WHATWG HTML §13.2.2 non-conformance;
/// `name` is diagnostic only. The full support for each lands behind its own
/// slot (foreign context / `setHTMLUnsafe` DSD-on-context in slice 2b).
pub(super) fn unsupported_fragment_construct(name: &str) -> StrictParseError {
    StrictParseError {
        errors: vec![name.to_string()],
    }
}
