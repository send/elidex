//! WHATWG HTML §13.2.4 "Parse state" — the tree-construction parse state.
//!
//! Holds the mutable bookkeeping the tree builder threads through every
//! token: the [insertion mode](https://html.spec.whatwg.org/multipage/parsing.html#the-insertion-mode)
//! (§13.2.4.1), the [stack of open elements](https://html.spec.whatwg.org/multipage/parsing.html#the-stack-of-open-elements)
//! (§13.2.4.2), the [element pointers](https://html.spec.whatwg.org/multipage/parsing.html#the-element-pointers)
//! (§13.2.4.4), and the [other parsing state flags](https://html.spec.whatwg.org/multipage/parsing.html#other-parsing-state-flags)
//! (§13.2.4.5).
//!
//! # Scope-out: the list of active formatting elements (§13.2.4.3)
//!
//! Strict mode does **not** maintain the list of active formatting elements.
//! That structure exists only to drive the adoption agency algorithm
//! (§13.2.6.4.7) and "reconstruct the active formatting elements", both of
//! which are error-recovery machinery. For fully-conforming nesting the
//! adoption agency degenerates to a plain pop and reconstruction is a no-op,
//! so the output tree is identical whether or not the list is tracked
//! (output-equivalence). Any input that would exercise non-trivial adoption
//! agency is non-conforming and aborts with [`crate::StrictParseError`].

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity, Namespace};

/// The tree builder's insertion mode (WHATWG HTML §13.2.4.1).
///
/// One variant per insertion mode named in §13.2.6.4.1–.21. The historical
/// "in select" / "in select in table" modes were removed from the spec with
/// the customizable-`<select>` change and are intentionally absent: `<select>`
/// content is now parsed through the "in body" rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsertionMode {
    /// §13.2.6.4.1 "initial".
    Initial,
    /// §13.2.6.4.2 "before html".
    BeforeHtml,
    /// §13.2.6.4.3 "before head".
    BeforeHead,
    /// §13.2.6.4.4 "in head".
    InHead,
    /// §13.2.6.4.5 "in head noscript".
    InHeadNoscript,
    /// §13.2.6.4.6 "after head".
    AfterHead,
    /// §13.2.6.4.7 "in body".
    InBody,
    /// §13.2.6.4.8 "text".
    Text,
    /// §13.2.6.4.9 "in table".
    InTable,
    /// §13.2.6.4.10 "in table text".
    InTableText,
    /// §13.2.6.4.11 "in caption".
    InCaption,
    /// §13.2.6.4.12 "in column group".
    InColumnGroup,
    /// §13.2.6.4.13 "in table body".
    InTableBody,
    /// §13.2.6.4.14 "in row".
    InRow,
    /// §13.2.6.4.15 "in cell".
    InCell,
    /// §13.2.6.4.16 "in template".
    InTemplate,
    /// §13.2.6.4.17 "after body".
    AfterBody,
    /// §13.2.6.4.18 "in frameset".
    InFrameset,
    /// §13.2.6.4.19 "after frameset".
    AfterFrameset,
    /// §13.2.6.4.20 "after after body".
    AfterAfterBody,
    /// §13.2.6.4.21 "after after frameset".
    AfterAfterFrameset,
}

/// A "specific scope" element-type list (WHATWG HTML §13.2.4.2).
///
/// Each variant selects the set of element types that terminate the
/// "have an element in _scope_" stack walk in a failure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// "have an element in scope".
    Default,
    /// "have an element in list item scope" (default + `ol`/`ul`).
    ListItem,
    /// "have an element in button scope" (default + `button`).
    Button,
    /// "have an element in table scope" (`html`/`table`/`template` only).
    Table,
}

/// Whether the element `(namespace, tag)` is a scope-boundary element type
/// for `scope` (WHATWG HTML §13.2.4.2).
///
/// The "has an element in scope" element types are namespace-qualified: the
/// base list (`applet`/`caption`/…/`template`) is HTML-namespace, and the
/// foreign integration-point boundaries — MathML
/// `mi`/`mo`/`mn`/`ms`/`mtext`/`annotation-xml` and SVG
/// `foreignObject`/`desc`/`title` — terminate the walk only in their own
/// namespace. Tag-name alone is insufficient: an SVG `title` is a boundary
/// while an HTML `title` is not, so foreign content (PR #256 namespaces)
/// makes the namespace discriminator load-bearing.
fn is_scope_boundary(namespace: Namespace, tag: &str, scope: Scope) -> bool {
    let base = match namespace {
        Namespace::Html => matches!(
            tag,
            "applet"
                | "caption"
                | "html"
                | "table"
                | "td"
                | "th"
                | "marquee"
                | "object"
                | "select"
                | "template"
        ),
        Namespace::MathMl => {
            matches!(tag, "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml")
        }
        Namespace::Svg => matches!(tag, "foreignObject" | "desc" | "title"),
    };
    match scope {
        Scope::Default => base,
        Scope::ListItem => base || (namespace == Namespace::Html && matches!(tag, "ol" | "ul")),
        Scope::Button => base || (namespace == Namespace::Html && tag == "button"),
        // §13.2.4.2 "in table scope" uses a distinct, smaller list — HTML
        // `html`/`table`/`template` only, *not* the default list plus extras,
        // and notably not the foreign boundaries.
        Scope::Table => {
            namespace == Namespace::Html && matches!(tag, "html" | "table" | "template")
        }
    }
}

/// Run the §13.2.4.2 "have an element in _scope_" walk, matching any element
/// whose `(namespace, tag)` satisfies `is_target`.
///
/// Walks the stack from the current node (bottom) upward, terminating in a
/// match state when an element satisfies `is_target` and in a failure state
/// when a scope-boundary element type for `scope` is reached first. Both the
/// target predicate and the boundary check are namespace-qualified (an HTML
/// `p` is not matched by a foreign `p`, and an SVG `title` boundary is not an
/// HTML `title`).
pub(crate) fn has_in_scope(
    dom: &EcsDom,
    stack: &[Entity],
    scope: Scope,
    is_target: impl Fn(Namespace, &str) -> bool,
) -> bool {
    for &entity in stack.iter().rev() {
        let namespace = dom.namespace_of(entity);
        let verdict = dom.with_tag_name(entity, |tag| match tag {
            Some(name) if is_target(namespace, name) => Some(true),
            Some(name) if is_scope_boundary(namespace, name, scope) => Some(false),
            _ => None,
        });
        if let Some(matched) = verdict {
            return matched;
        }
    }
    false
}

/// Run the §13.2.4.2 "have an element in _scope_" walk for a specific element
/// (matched by entity identity rather than tag name).
///
/// Used for the `</form>` handling, where the target is the element pointed to
/// by the form element pointer.
pub(crate) fn has_entity_in_scope(
    dom: &EcsDom,
    stack: &[Entity],
    target: Entity,
    scope: Scope,
) -> bool {
    for &entity in stack.iter().rev() {
        if entity == target {
            return true;
        }
        let namespace = dom.namespace_of(entity);
        let boundary = dom.with_tag_name(entity, |tag| match tag {
            Some(name) => is_scope_boundary(namespace, name, scope),
            None => false,
        });
        if boundary {
            return false;
        }
    }
    false
}

/// The mutable tree-construction state (WHATWG HTML §13.2.4).
pub(crate) struct ParseState {
    /// §13.2.4.1 The insertion mode.
    pub(crate) mode: InsertionMode,
    /// §13.2.4.1 The original insertion mode (saved when switching to "text"
    /// or "in table text").
    pub(crate) original_mode: Option<InsertionMode>,
    /// §13.2.4.2 The stack of open elements (grows downward; current node is
    /// the last entry).
    pub(crate) open_elements: Vec<Entity>,
    /// §13.2.4.1 The stack of template insertion modes.
    pub(crate) template_modes: Vec<InsertionMode>,
    /// §13.2.4.4 The head element pointer.
    pub(crate) head_pointer: Option<Entity>,
    /// §13.2.4.4 The form element pointer.
    pub(crate) form_pointer: Option<Entity>,
    /// §13.2.4.5 The frameset-ok flag (`true` == "ok").
    ///
    /// Maintained per spec (the "set the frameset-ok flag to not ok" steps) for
    /// fidelity, but never read in strict mode: its only consumer is the
    /// `<frameset>` start tag in "in body", which strict rejects
    /// unconditionally. Kept as faithful §13.2.4.5 parse state.
    pub(crate) frameset_ok: bool,
    /// §13.2.4.5 The scripting flag. Fixed to `true` for the v1 baseline: the
    /// strict parser models a scripting-enabled document, so `<noscript>` in
    /// "in head" follows the generic raw text algorithm and the "in head
    /// noscript" mode is only reachable with scripting disabled.
    pub(crate) scripting: bool,
    /// Per-`<template>` content insertion target, populated only for
    /// declarative shadow templates (§13.2.6.4.4 step 10): the template's
    /// "template contents" is the attached shadow root, so children inserted
    /// while that template is the current node are routed to the shadow root
    /// instead of the template element. Normal templates are absent from the
    /// map and default to holding their content as direct children.
    pub(crate) template_content_targets: HashMap<Entity, Entity>,
    /// Whether the next character token, if it is a U+000A LINE FEED, should
    /// be ignored. Set after inserting `<pre>` / `<listing>` / `<textarea>`,
    /// whose leading newline is dropped as an authoring convenience
    /// (§13.2.6.4.7).
    pub(crate) skip_next_lf: bool,
}

impl ParseState {
    /// Create the initial parse state (§13.2.4 initial values): insertion mode
    /// "initial", empty stacks, null element pointers, frameset-ok "ok",
    /// scripting enabled.
    pub(crate) fn new() -> Self {
        ParseState {
            mode: InsertionMode::Initial,
            original_mode: None,
            open_elements: Vec::new(),
            template_modes: Vec::new(),
            head_pointer: None,
            form_pointer: None,
            frameset_ok: true,
            scripting: true,
            template_content_targets: HashMap::new(),
            skip_next_lf: false,
        }
    }

    /// §13.2.4.2 The current node — the bottommost (most recently pushed)
    /// element on the stack of open elements, or `None` in the fragment-free
    /// document case before the `html` element is pushed.
    pub(crate) fn current_node(&self) -> Option<Entity> {
        self.open_elements.last().copied()
    }
}

/// Whether `ch` is one of the five WHATWG HTML "ASCII whitespace" characters
/// the tree-construction stage treats specially (U+0009, U+000A, U+000C,
/// U+000D, U+0020).
pub(crate) fn is_html_whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\u{000C}' | '\r' | ' ')
}

/// Whether `tag` is in the "special" category of WHATWG HTML §13.2.4.2.
///
/// Deliberately HTML-namespace only: the MathML
/// (`mi`/`mo`/`mn`/`ms`/`mtext`/`annotation-xml`) and SVG
/// (`foreignObject`/`desc`/`title`/…) special types are omitted even though
/// foreign elements now reach the stack (§13.2.6.5). The "special" category
/// drives only the adoption agency algorithm and the "any other end tag"
/// HTML-content walk — both error-recovery machinery — and foreign content
/// runs its own end-tag walk (§13.2.6.5), never the HTML special check. A
/// misnested foreign end tag is rejected by the current-node identity check,
/// not by reaching a foreign "special" element, so the discriminator stays
/// tag-only without affecting strict accept/reject behaviour.
pub(crate) fn is_special_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "applet"
            | "area"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "bgsound"
            | "blockquote"
            | "body"
            | "br"
            | "button"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "embed"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "iframe"
            | "img"
            | "input"
            | "keygen"
            | "li"
            | "link"
            | "listing"
            | "main"
            | "marquee"
            | "menu"
            | "meta"
            | "nav"
            | "noembed"
            | "noframes"
            | "noscript"
            | "object"
            | "ol"
            | "p"
            | "param"
            | "plaintext"
            | "pre"
            | "script"
            | "search"
            | "section"
            | "select"
            | "source"
            | "style"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "template"
            | "textarea"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
            | "wbr"
            | "xmp"
    )
}
