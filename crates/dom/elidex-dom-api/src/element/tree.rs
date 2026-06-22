//! Tree mutation handlers: appendChild, insertBefore, removeChild, insertAdjacent*.

use elidex_ecs::{EcsDom, Entity, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_append_child, apply_insert_before, apply_remove_child, apply_replace_child, DomApiError,
    DomApiErrorKind, DomApiHandler, JsObjectRef, Mutation, MutationRecord, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg, require_string_arg};

// The `apply_*` childList builders expand a `DocumentFragment` into its children
// (WHATWG DOM §4.2.3 "insert" step 1) and return an **empty** record list for an
// *empty* fragment — a valid no-op (§4.2.3 step 3), NOT a failure. They also
// return an empty list for a genuine failure (cycle / self-ancestor / bad
// reference child). The handlers below disambiguate via [`is_empty_fragment_noop`].

/// Whether an empty `apply_*` record list for `child` inserted under `parent` is a
/// valid **empty-`DocumentFragment` no-op** (§4.2.3 insert step 3) rather than a
/// hierarchy failure.
///
/// A genuinely empty fragment is a no-op ONLY when it would pass §4.2.3 "ensure
/// pre-insertion validity" step 2: it must not be `parent` itself nor a
/// host-including inclusive ancestor of `parent` — so `frag.appendChild(frag)`
/// still throws. A non-empty fragment never reaches the empty-list branch unless
/// it was rejected (cycle ⇒ ancestor-of-`parent`), which the same predicate
/// catches; the explicit emptiness check keeps the intent legible.
fn is_empty_fragment_noop(dom: &EcsDom, parent: Entity, child: Entity) -> bool {
    dom.is_document_fragment(child)
        && dom.children_iter(child).next().is_none()
        && !dom.is_ancestor_or_self(child, parent)
}

// ---------------------------------------------------------------------------
// appendChild
// ---------------------------------------------------------------------------

/// `element.appendChild(child)` — appends a child node.
pub struct AppendChild;

impl DomApiHandler for AppendChild {
    fn method_name(&self) -> &str {
        "appendChild"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let child_ref = require_object_ref_arg(args, 0)?;
        let (child_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(child_ref))
            .ok_or_else(|| not_found_error("child not found"))?;
        // Apply through the EcsDom chokepoint AND build the §4.3.2 childList record
        // list in one step. A move yields two records (source-parent removal +
        // destination); a DocumentFragment yields two (the §4.2.3 step-4.2 fragment
        // record + the destination record carrying the expanded children). Records
        // are staged for §4.3 microtask delivery.
        let records = apply_append_child(dom, this, child_entity);
        if records.is_empty() {
            // Empty list = failure (cycle / invalid parent) EXCEPT a valid empty
            // DocumentFragment no-op (§4.2.3 insert step 3).
            if is_empty_fragment_noop(dom, this, child_entity) {
                return Ok(JsValue::ObjectRef(child_ref));
            }
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "appendChild: hierarchy request error (cycle or invalid parent)".into(),
            });
        }
        for record in records {
            session.push_notify_record(record);
        }
        Ok(JsValue::ObjectRef(child_ref))
    }
}

// ---------------------------------------------------------------------------
// insertBefore
// ---------------------------------------------------------------------------

/// `element.insertBefore(newChild, refChild)` — inserts a child before a reference child.
pub struct InsertBefore;

impl DomApiHandler for InsertBefore {
    fn method_name(&self) -> &str {
        "insertBefore"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new_ref = require_object_ref_arg(args, 0)?;
        let (new_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(new_ref))
            .ok_or_else(|| not_found_error("newChild not found"))?;

        // WebIDL `Node?` — both `null` and `undefined` mean "no
        // reference child"; missing arg is the same.
        let ref_child_is_null =
            matches!(args.get(1), None | Some(JsValue::Null | JsValue::Undefined));
        if ref_child_is_null {
            // null reference child = append (WHATWG DOM §4.2.3 pre-insert).
            let records = apply_append_child(dom, this, new_entity);
            if records.is_empty() {
                // Empty = failure EXCEPT a valid empty DocumentFragment no-op
                // (§4.2.3 step 3); a null ref needs no reference-child check.
                if is_empty_fragment_noop(dom, this, new_entity) {
                    return Ok(JsValue::ObjectRef(new_ref));
                }
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertBefore: hierarchy request error (cycle or invalid parent)"
                        .into(),
                });
            }
            for record in records {
                session.push_notify_record(record);
            }
            return Ok(JsValue::ObjectRef(new_ref));
        }

        let ref_ref = require_object_ref_arg(args, 1)?;
        let (ref_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_ref))
            .ok_or_else(|| not_found_error("refChild not found"))?;
        let records = apply_insert_before(dom, this, new_entity, ref_entity);
        if records.is_empty() {
            // Empty = failure (invalid reference child or cycle) EXCEPT a valid
            // empty DocumentFragment inserted before a *valid* reference child
            // (§4.2.3 step 3). A bad reference child is always an error, even for an
            // empty fragment — so the no-op additionally requires ref ∈ parent.
            if is_empty_fragment_noop(dom, this, new_entity)
                && dom.get_parent(ref_entity) == Some(this)
            {
                return Ok(JsValue::ObjectRef(new_ref));
            }
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "insertBefore: hierarchy request error (invalid reference child or cycle)"
                    .into(),
            });
        }
        for record in records {
            session.push_notify_record(record);
        }
        Ok(JsValue::ObjectRef(new_ref))
    }
}

// ---------------------------------------------------------------------------
// removeChild
// ---------------------------------------------------------------------------

/// `element.removeChild(child)` — removes a child node.
pub struct RemoveChild;

impl DomApiHandler for RemoveChild {
    fn method_name(&self) -> &str {
        "removeChild"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let child_ref = require_object_ref_arg(args, 0)?;
        let (child_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(child_ref))
            .ok_or_else(|| not_found_error("child not found"))?;
        match apply_remove_child(dom, this, child_entity) {
            Some(record) => session.push_notify_record(record),
            None => return Err(not_found_error("child is not a child of this element")),
        }
        Ok(JsValue::ObjectRef(child_ref))
    }
}

// ---------------------------------------------------------------------------
// replaceChild
// ---------------------------------------------------------------------------

/// `parent.replaceChild(newChild, oldChild)` — replaces `oldChild` with
/// `newChild` and returns the replaced node (the §4.4 `replaceChild` method runs
/// the WHATWG DOM §4.2.3 "replace" algorithm, `#concept-node-replace`).
///
/// Error mapping (steps of the §4.2.3 "replace" algorithm):
/// - `oldChild` is not a child of `parent` → `NotFoundError`
///   (§4.2.3 step 3; matches Chrome/Firefox/WebKit).
/// - `newChild` is `parent` itself, an ancestor of `parent`, or any other
///   pre-insertion validity violation → `HierarchyRequestError`
///   (§4.2.3 steps 1-2, 4). Cross-document violations also map here per
///   §4.2.3 — `WrongDocumentError` is not a separate `DomApiErrorKind`.
///
/// The replace is delegated to a single `EcsDom::replace_child` op so the
/// `MutationObserver` integration coalesces the old-child removal + new-child
/// insertion into **one** record per spec (`#concept-node-replace` step 14),
/// not the two a naive remove-then-insert composition would produce. A
/// **move** into the replace slot (an already-parented `newChild`) additionally
/// emits the source-parent removal record from `newChild`'s adopt (B1.2a, §4.5
/// step 2, NOT suppressed) — that is a distinct record on a different parent,
/// not a split of the coalesced one.
pub struct ReplaceChild;

impl DomApiHandler for ReplaceChild {
    fn method_name(&self) -> &str {
        "replaceChild"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new_ref = require_object_ref_arg(args, 0)?;
        let old_ref = require_object_ref_arg(args, 1)?;
        let (new_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(new_ref))
            .ok_or_else(|| not_found_error("newChild not found"))?;
        let (old_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(old_ref))
            .ok_or_else(|| not_found_error("oldChild not found"))?;

        // §4.2.3 "replace" step 3: if oldChild's parent is not parent, NotFoundError.
        // EcsDom::replace_child collapses every failure to a single bool;
        // splitting the parent check out lets us distinguish NotFoundError
        // from HierarchyRequestError without re-implementing the rest of
        // the algorithm at the API layer.
        if dom.get_parent(old_entity) != Some(this) {
            return Err(not_found_error(
                "the node to be replaced is not a child of this node",
            ));
        }

        // Self-replace (`parent.replaceChild(x, x)`) is a no-op per
        // browser parity (Chrome / Firefox / WebKit) — the spec
        // §4.2.3 "replace" step 8 reference-child adjustment makes the
        // insert+remove sequence collapse to nothing.  EcsDom::replace_child rejects
        // `new == old` early (would surface as HierarchyRequestError),
        // so handle it here before dispatching.
        if new_entity == old_entity {
            return Ok(JsValue::ObjectRef(old_ref));
        }

        // §4.2.3 "replace": for a fresh newChild, one coalesced childList record
        // (added=[new], removed=[old]); for an already-parented newChild (a move
        // into the replace slot), `apply_replace_child` adds the source-parent
        // removal record from newChild's adopt (NOT suppressed) → two records. A
        // **DocumentFragment** newChild expands its children into the coalesced
        // record's addedNodes and prepends the §4.2.3 step-4.2 fragment record;
        // an *empty* fragment still removes oldChild and yields one coalesced
        // record (added=«», removed=[old]) — so the empty list below is always a
        // genuine failure here (no fragment no-op short-circuit, unlike append).
        let records = apply_replace_child(dom, this, new_entity, old_entity);
        if records.is_empty() {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "replaceChild: hierarchy request error (cycle, invalid kind, \
                          or self/ancestor receiver)"
                    .into(),
            });
        }
        for record in records {
            session.push_notify_record(record);
        }
        Ok(JsValue::ObjectRef(old_ref))
    }
}

// ---------------------------------------------------------------------------
// textContent helpers
// ---------------------------------------------------------------------------

/// Collect all text content from an entity and its descendants.
pub fn collect_text_content(entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    collect_text_recursive(entity, dom, &mut result);
    result
}

fn collect_text_recursive(entity: Entity, dom: &EcsDom, result: &mut String) {
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        result.push_str(&tc.0);
        return;
    }
    for child in dom.children_iter(entity) {
        collect_text_recursive(child, dom, result);
    }
}

// ---------------------------------------------------------------------------
// insertAdjacentElement / insertAdjacentText (WHATWG DOM §4.9 + §4.2.3)
// ---------------------------------------------------------------------------
//
// # Convergence (B1.2b-2)
//
// These handlers are the single algorithm home for ALL runtimes (boa/wasm and,
// post-S5, the VM): they compute the WHATWG "insert adjacent" site
// (`#insert-adjacent`) read-only, then mutate through the record-producing
// `apply_insert_before` / `apply_append_child` primitives so a move yields the
// §4.5-adopt source-removal record alongside the destination record
// (MutationObserver parity with `appendChild`/`insertBefore`). The VM is now a
// thin dispatcher (`vm/host/element_insert_adjacent.rs`) that brand-checks the
// receiver + the `Element` arg (engine-bound marshalling — it distinguishes a
// detached wrapper from a wrong-type one) and routes here; the prior VM
// re-implementation of the position+insert algorithm is gone (One-issue-one-way).
// The Element-kind guard in `InsertAdjacentElement::invoke` is the sole
// protection on the boa/wasm path (where a non-Element arg resolved through the
// identity map would otherwise be relinked) and defense-in-depth on the VM path.

/// `element.insertAdjacentElement(position, element)` — inserts an element
/// at the specified position relative to `this` (WHATWG DOM §4.9
/// `#dom-element-insertadjacentelement`). Returns the inserted element, or null
/// on the parent-null `beforebegin`/`afterend` no-op.
pub struct InsertAdjacentElement;

impl DomApiHandler for InsertAdjacentElement {
    fn method_name(&self) -> &str {
        "insertAdjacentElement"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = require_string_arg(args, 0)?;
        let elem_ref = require_object_ref_arg(args, 1)?;
        let (elem_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(elem_ref))
            .ok_or_else(|| not_found_error("element not found"))?;

        // WebIDL `Element element`: a non-Element arg (Text / Comment /
        // ShadowRoot / DocumentFragment) must be rejected, not inserted. This
        // engine-independent guard closes the boa-reachable gap where such a
        // node, resolved through the identity map, would otherwise be relinked
        // by the insert below. (The VM additionally brand-checks this arg as
        // marshalling, distinguishing detached wrappers — defense-in-depth.)
        if !dom.is_element(elem_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::TypeError,
                message: "Failed to execute 'insertAdjacentElement' on 'Element': \
                          parameter 2 is not of type 'Element'."
                    .into(),
            });
        }

        match resolve_adjacent_site(this, &position, dom)? {
            // "If element's parent is null, return null" — a silent no-op.
            AdjacentSite::NoOp => Ok(JsValue::Null),
            AdjacentSite::Into { parent, before } => {
                let records = apply_adjacent_insert(parent, before, elem_entity, dom);
                if records.is_empty() {
                    return Err(hierarchy_error("insertAdjacentElement"));
                }
                for record in records {
                    session.push_notify_record(record);
                }
                Ok(JsValue::ObjectRef(elem_ref))
            }
        }
    }
}

/// `element.insertAdjacentText(position, data)` — creates a text node and
/// inserts it at the specified position relative to `this` (WHATWG DOM §4.9
/// `#dom-element-insertadjacenttext`). Returns undefined (void).
pub struct InsertAdjacentText;

impl DomApiHandler for InsertAdjacentText {
    fn method_name(&self) -> &str {
        "insertAdjacentText"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = require_string_arg(args, 0)?;
        let data = require_string_arg(args, 1)?;

        // Resolve the insertion site BEFORE allocating the Text node: a bad
        // position (SyntaxError) or the parent-null `beforebegin`/`afterend`
        // no-op returns without ever creating a Text, so no unreferenced Text
        // entity leaks into the ECS (no JS handle is ever returned to anchor it
        // for GC). WHATWG §4.9 step 2: the Text's node document is this's.
        match resolve_adjacent_site(this, &position, dom)? {
            AdjacentSite::NoOp => Ok(JsValue::Undefined),
            AdjacentSite::Into { parent, before } => {
                let owner = dom.owner_document(this);
                let text_node = dom.create_text_with_owner(data, owner);
                let records = apply_adjacent_insert(parent, before, text_node, dom);
                if records.is_empty() {
                    // Insertion failed (a fresh Text cannot cycle, but defend
                    // anyway) — destroy the unreferenced Text so the error path
                    // leaks nothing.
                    let _ = dom.destroy_entity(text_node);
                    return Err(hierarchy_error("insertAdjacentText"));
                }
                for record in records {
                    session.push_notify_record(record);
                }
                Ok(JsValue::Undefined)
            }
        }
    }
}

/// The resolved insertion site for the WHATWG DOM "insert adjacent" algorithm
/// (`#insert-adjacent`), computed read-only (no tree mutation).
enum AdjacentSite {
    /// `beforebegin`/`afterend` on a parent-less element: the spec returns null
    /// (a silent no-op), NOT an error.
    NoOp,
    /// Insert the node into `parent`, before `before` (`None` = append at end).
    Into {
        parent: Entity,
        before: Option<Entity>,
    },
}

/// Resolve `position` against `this` into an [`AdjacentSite`] per the four
/// WHATWG "insert adjacent" cases (`#insert-adjacent`), read-only and
/// ASCII-case-insensitive. The "Otherwise" branch is a SyntaxError. `this` must
/// be an Element (an InvalidStateError otherwise — the VM/boa receiver
/// brand-checks uphold this; the guard is defense-in-depth).
fn resolve_adjacent_site(
    this: Entity,
    position: &str,
    dom: &EcsDom,
) -> Result<AdjacentSite, DomApiError> {
    if !dom.is_element(this) {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidStateError,
            message: "insertAdjacent: context node must be an Element".into(),
        });
    }
    // `afterbegin`/`afterend` resolve their reference child against the **exposed**
    // (light-tree) chain — `children_iter().next()` / `next_exposed_sibling` skip an
    // internal `ShadowRoot` (DOM §4.8: a shadow root is not a light-tree child of its
    // host, so it is never the "first child" insertion reference). Raw
    // `get_first_child`/`get_next_sibling` would point at the ShadowRoot for a host
    // whose shadow root was attached before any light children, inserting the node
    // before it and leaking the encapsulated entity into the childList record's
    // `nextSibling`. (The deleted VM `perform_adjacent_insert` used the shadow-skipping
    // walk; the prior dom-api helper used the raw accessors — this converges both onto
    // the correct exposed reference.)
    Ok(match position.to_ascii_lowercase().as_str() {
        "beforebegin" => match dom.get_parent(this) {
            Some(parent) => AdjacentSite::Into {
                parent,
                before: Some(this),
            },
            None => AdjacentSite::NoOp,
        },
        "afterbegin" => AdjacentSite::Into {
            parent: this,
            before: dom.children_iter(this).next(),
        },
        "beforeend" => AdjacentSite::Into {
            parent: this,
            before: None,
        },
        "afterend" => match dom.get_parent(this) {
            Some(parent) => AdjacentSite::Into {
                parent,
                before: dom.next_exposed_sibling(this),
            },
            None => AdjacentSite::NoOp,
        },
        _ => {
            return Err(DomApiError {
                kind: DomApiErrorKind::SyntaxError,
                message: format!(
                    "insertAdjacent: invalid position '{position}' (expected beforebegin, \
                     afterbegin, beforeend, afterend)"
                ),
            });
        }
    })
}

/// Apply a non-no-op [`AdjacentSite`] through the record-producing `apply_*`
/// primitives. A fresh node yields one childList record; an already-parented
/// node (a move) yields two (§4.5-adopt source-removal + destination, NOT
/// suppressed). An empty list is a genuine failure (cycle / shadow-root reject /
/// invalid reference child) the caller maps to HierarchyRequestError.
fn apply_adjacent_insert(
    parent: Entity,
    before: Option<Entity>,
    node: Entity,
    dom: &mut EcsDom,
) -> Vec<MutationRecord> {
    match before {
        Some(ref_child) => apply_insert_before(dom, parent, node, ref_child),
        None => apply_append_child(dom, parent, node),
    }
}

/// `HierarchyRequestError` for a failed insert-adjacent (cycle / invalid
/// insertion point).
fn hierarchy_error(method: &str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::HierarchyRequestError,
        message: format!("{method}: hierarchy request error (cycle or invalid insertion point)"),
    }
}

// ---------------------------------------------------------------------------
// innerHTML getter
// ---------------------------------------------------------------------------

use crate::util::{escape_attr, escape_html};
use elidex_ecs::{Attributes, TagType};

/// HTML raw text elements whose text children must NOT be escaped during
/// serialization (the content is literal, not entity-decoded by parsers).
const RAW_TEXT_ELEMENTS: &[&str] = &[
    "script", "style", "xmp", "iframe", "noembed", "noframes", "noscript",
];

/// `element.innerHTML` setter — replaces children with parsed HTML.
///
/// Records a `Mutation::SetInnerHtml` which is applied during
/// `session.flush()` so that any caller flushing through the session
/// (notably the boa runtime) keeps surfacing innerHTML changes as
/// `MutationRecord`s through the flush return value — required for
/// MutationObserver delivery and custom-element-reaction enqueueing
/// on that path. The VM-direct path in `vm/host/dom_inner_html.rs`
/// bypasses this handler and applies the algorithm synchronously
/// (calling [`elidex_script_session::apply_set_inner_html`]) before invoking
/// `Vm::deliver_mutation_records` itself, so the two paths share one
/// engine-indep algorithm without colliding on observer delivery.
pub struct SetInnerHtml;

impl DomApiHandler for SetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = match args.first() {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };
        session.record_mutation(Mutation::SetInnerHtml { entity: this, html });
        Ok(JsValue::Undefined)
    }
}

/// `element.insertAdjacentHTML(position, text)` — parses HTML and inserts at position.
///
/// Position values: "beforebegin", "afterbegin", "beforeend", "afterend".
/// Uses the same fragment parser as innerHTML setter. Parsed nodes are inserted
/// directly via DOM operations (not via mutation recording, since the parser
/// needs mutable DOM access).
pub struct InsertAdjacentHtml;

impl DomApiHandler for InsertAdjacentHtml {
    fn method_name(&self) -> &str {
        "insertAdjacentHTML"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = match args.first() {
            Some(JsValue::String(s)) => s.to_ascii_lowercase(),
            _ => {
                return Err(DomApiError::syntax_error(
                    "insertAdjacentHTML requires a position string",
                ));
            }
        };
        let html = match args.get(1) {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };

        // Validate position and parent requirement before recording.
        match position.as_str() {
            "beforebegin" | "afterend" => {
                if dom.get_parent(this).is_none() {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: format!(
                            "insertAdjacentHTML: element has no parent for {position}"
                        ),
                    });
                }
            }
            "afterbegin" | "beforeend" => {}
            _ => {
                return Err(DomApiError::syntax_error(
                    "Invalid position for insertAdjacentHTML",
                ));
            }
        }

        // Record mutation — applied during session.flush() with proper
        // MutationRecord generation for MutationObserver support.
        session.record_mutation(Mutation::InsertAdjacentHtml {
            entity: this,
            position,
            html,
        });

        Ok(JsValue::Undefined)
    }
}

/// `element.innerHTML` getter — serializes children to HTML.
pub struct GetInnerHtml;

impl DomApiHandler for GetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = serialize_inner_html(this, dom);
        Ok(JsValue::String(html))
    }
}

/// Serialize children of an entity to HTML using the default options
/// (matches plain `innerHTML` / `outerHTML` getter behaviour — shadow
/// roots are skipped to preserve encapsulation per WHATWG DOM §4.8).
pub fn serialize_inner_html(entity: Entity, dom: &EcsDom) -> String {
    serialize_inner_html_with_options(entity, dom, &SerializeOptions::default())
}

/// Serialize `entity` itself plus its descendants — opening tag with
/// attributes, recursive child serialization, closing tag. Shadow
/// roots remain skipped per default `SerializeOptions`; for `getHTML`
/// callers needing shadow visibility, the entity tag is serialized
/// here and the inner content is routed through
/// [`serialize_inner_html_with_options`] separately.
pub fn serialize_outer_html(entity: Entity, dom: &EcsDom) -> String {
    let mut html = String::new();
    serialize_node(entity, dom, &mut html, false, &SerializeOptions::default());
    html
}

/// Options controlling shadow root visibility for [`serialize_inner_html_with_options`].
///
/// Per HTML §4.4.6 `Element.getHTML(options)` / `ShadowRoot.getHTML(options)`:
/// - `serializable_shadow_roots: true` emits a `<template shadowrootmode>`
///   element for any shadow root whose host is currently being serialized
///   AND whose `serializable` flag is set.
/// - `explicit_shadow_roots` is a set of shadow root entities that must
///   ALWAYS be emitted regardless of their `serializable` flag or mode
///   ("closed" included) — matches the spec's "regardless of whether or
///   not they are marked as serializable" clause.
#[derive(Default, Clone, Debug)]
pub struct SerializeOptions {
    /// When true, serialize each visited shadow root whose `serializable`
    /// flag is set as a declarative `<template shadowrootmode>` child of
    /// its host (HTML §13.5 fragment-serialization with shadow roots).
    pub serializable_shadow_roots: bool,
    /// Force-emit declarative templates for these shadow root entities
    /// regardless of their `serializable` flag. (HTML §4.4.6 places no
    /// `mode` restriction on either path — both `Open` and `Closed`
    /// shadows may be serialized when their host opted in.)
    pub explicit_shadow_roots: std::collections::HashSet<Entity>,
}

/// Serialize children of `entity` to HTML, honouring `opts` for shadow
/// root visibility. Equivalent to [`serialize_inner_html`] when `opts`
/// is the default.
///
/// Per HTML §13.5 fragment serialization with shadow roots, when
/// `entity` is itself a shadow host whose shadow root should be
/// emitted (see [`SerializeOptions`]), the declarative
/// `<template shadowrootmode>` is written *first*, ahead of the
/// light-tree children. This ordering matches the spec's "shadow
/// roots are serialized as the first child of their host" rule and
/// makes `setHTMLUnsafe(host.getHTML(opts))` round-trip correctly.
pub fn serialize_inner_html_with_options(
    entity: Entity,
    dom: &EcsDom,
    opts: &SerializeOptions,
) -> String {
    let mut html = String::new();
    emit_own_shadow_root_if_needed(entity, dom, &mut html, opts);
    let raw_text = dom
        .world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|tag| RAW_TEXT_ELEMENTS.contains(&tag.0.as_str()));
    // HTML §13.3: a `<template>`'s serialized children are its *template
    // contents* (the detached fragment), not its (empty) light children.
    for child in dom.children_iter(serialization_children_root(entity, dom)) {
        serialize_node(child, dom, &mut html, raw_text, opts);
    }
    html
}

/// The node whose children the HTML fragment serialization algorithm
/// (HTML §13.3) walks for `entity`: a `<template>`'s content fragment ("let
/// the node also be current node's template contents"), else `entity` itself.
fn serialization_children_root(entity: Entity, dom: &EcsDom) -> Entity {
    dom.template_contents_fragment(entity).unwrap_or(entity)
}

/// HTML void elements that must not have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Returns `true` if the attribute name contains characters that would break
/// HTML serialization.
fn is_safe_attr_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b != b'"' && b != b'>' && b != b'<' && b != b'=' && !b.is_ascii_whitespace())
}

/// Decide whether to emit a given shadow root as a declarative template.
/// Per HTML §4.4.6 there is no mode-based filter (both `Open` and
/// `Closed` may be serialized whenever the host opted in): when the
/// caller-supplied `explicit_shadow_roots` set contains `sr` it is
/// always emitted; otherwise it is emitted iff the caller passed
/// `serializable_shadow_roots = true` AND the root carries the
/// `serializable` init flag.
fn shadow_root_should_emit(
    sr: Entity,
    sr_component: &elidex_ecs::ShadowRoot,
    opts: &SerializeOptions,
) -> bool {
    if opts.explicit_shadow_roots.contains(&sr) {
        return true;
    }
    opts.serializable_shadow_roots && sr_component.serializable
}

/// Shared helper: if `entity` is a shadow host and the host's shadow
/// root meets `opts`'s emission criteria, append a declarative
/// `<template shadowrootmode>` block describing it. Called both at
/// the [`serialize_inner_html_with_options`] entry (for the host the
/// caller asked about) and inside [`serialize_node`] (for descendant
/// shadow hosts encountered while walking the light tree).
fn emit_own_shadow_root_if_needed(
    entity: Entity,
    dom: &EcsDom,
    html: &mut String,
    opts: &SerializeOptions,
) {
    let Some(sr) = dom.get_shadow_root(entity) else {
        return;
    };
    let Ok(sr_component) = dom.world().get::<&elidex_ecs::ShadowRoot>(sr) else {
        return;
    };
    if !shadow_root_should_emit(sr, &sr_component, opts) {
        return;
    }
    let sr_copy: elidex_ecs::ShadowRoot = *sr_component;
    drop(sr_component);
    emit_shadow_root_template(sr, &sr_copy, dom, html, opts);
}

fn emit_shadow_root_template(
    sr: Entity,
    sr_component: &elidex_ecs::ShadowRoot,
    dom: &EcsDom,
    html: &mut String,
    opts: &SerializeOptions,
) {
    html.push_str("<template shadowrootmode=\"");
    html.push_str(match sr_component.mode {
        elidex_ecs::ShadowRootMode::Open => "open",
        elidex_ecs::ShadowRootMode::Closed => "closed",
    });
    html.push('"');
    if sr_component.delegates_focus {
        html.push_str(" shadowrootdelegatesfocus=\"\"");
    }
    if sr_component.clonable {
        html.push_str(" shadowrootclonable=\"\"");
    }
    if sr_component.serializable {
        html.push_str(" shadowrootserializable=\"\"");
    }
    // `shadowrootslotassignment` is an enumerated declarative attribute
    // (manual / named) that the parser hook honours; omit for the
    // default `Named` so round-tripping unchanged hosts stays terse.
    if matches!(
        sr_component.slot_assignment,
        elidex_ecs::SlotAssignmentMode::Manual
    ) {
        html.push_str(" shadowrootslotassignment=\"manual\"");
    }
    html.push('>');
    for child in dom.children_iter(sr) {
        serialize_node(child, dom, html, false, opts);
    }
    html.push_str("</template>");
}

fn serialize_node(
    entity: Entity,
    dom: &EcsDom,
    html: &mut String,
    in_raw_text: bool,
    opts: &SerializeOptions,
) {
    // `EcsDom::children_iter` already skips ShadowRoot entities, so
    // they never reach this function — shadow visibility is gated by
    // [`emit_own_shadow_root_if_needed`] on the host instead, which
    // handles both the outer entity (called from
    // [`serialize_inner_html_with_options`]) and descendant hosts
    // (called for every Element below).
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        if in_raw_text {
            html.push_str(&tc.0);
        } else {
            html.push_str(&escape_html(&tc.0));
        }
        return;
    }
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        html.push('<');
        html.push_str(&tag.0);
        // HTML §13.3 "Serializing HTML fragments": if the node's *is
        // value* is non-null and the element has no `is` attribute in
        // its attribute list, append ` is="..."` (the spec places this
        // BEFORE the attribute loop) — this is what lets a customized
        // built-in created via `createElement(tag, {is})` (which sets
        // NO `is` attribute per DOM §4.5) survive a serialize→parse
        // round-trip.  The sparse `CustomElementState` probe runs
        // first so the overwhelmingly common no-CE element pays one
        // failed lookup; the membership check is the spec condition
        // (`has_attribute`), independent of emission filtering.
        // `escape_attr` is load-bearing: the is value is an arbitrary
        // author string (DOM §4.9 step 6.3 imposes no validity), so
        // raw emission would inject markup.
        if let Ok(ce) = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
        {
            if let Some(is_value) = ce.is_value() {
                if !dom.has_attribute(entity, "is") {
                    html.push_str(" is=\"");
                    html.push_str(&escape_attr(is_value));
                    html.push('"');
                }
            }
        }
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            let mut sorted: Vec<(&str, &str)> = attrs.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, value) in sorted {
                if !is_safe_attr_name(name) {
                    continue;
                }
                html.push(' ');
                html.push_str(name);
                html.push_str("=\"");
                html.push_str(&escape_attr(value));
                html.push('"');
            }
        }
        html.push('>');
        if VOID_ELEMENTS.contains(&tag.0.as_str()) {
            return;
        }
        emit_own_shadow_root_if_needed(entity, dom, html, opts);
        let child_raw_text = RAW_TEXT_ELEMENTS.contains(&tag.0.as_str());
        // HTML §13.3: a `<template>` serializes its template contents (the
        // detached fragment), not its (empty) light children.
        for child in dom.children_iter(serialization_children_root(entity, dom)) {
            serialize_node(child, dom, html, child_raw_text, opts);
        }
        html.push_str("</");
        html.push_str(&tag.0);
        html.push('>');
        return;
    }
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, html, false, opts);
    }
}

// ---------------------------------------------------------------------------
// Attribute name validation (WHATWG DOM §5.1)
// ---------------------------------------------------------------------------

/// Validate an attribute name per the WHATWG DOM spec.
pub fn validate_attribute_name(name: &str) -> Result<(), DomApiError> {
    if name.is_empty() {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidCharacterError,
            message: "attribute name must not be empty".into(),
        });
    }
    for ch in name.chars() {
        if ch.is_whitespace() || ch == '\0' || ch == '/' || ch == '=' || ch == '>' {
            return Err(DomApiError {
                kind: DomApiErrorKind::InvalidCharacterError,
                message: format!("attribute name contains invalid character: {ch:?}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_empty_fragment_noop;
    use elidex_ecs::{Attributes, EcsDom};

    // Codex PR387 R1 F1/F3: the empty-fragment-vs-failure disambiguation must
    // enforce §4.2.3 step-2 pre-insert validity — an empty fragment that is
    // `parent` itself or an ancestor of `parent` is a hierarchy error, NOT a no-op.
    #[test]
    fn empty_fragment_valid_placement_is_noop() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let frag = dom.create_document_fragment();
        assert!(is_empty_fragment_noop(&dom, parent, frag));
    }

    #[test]
    fn empty_self_fragment_is_not_noop() {
        // `frag.appendChild(frag)` — inclusive-ancestor of itself → hierarchy error.
        let mut dom = EcsDom::new();
        let frag = dom.create_document_fragment();
        assert!(!is_empty_fragment_noop(&dom, frag, frag));
    }

    #[test]
    fn empty_fragment_ancestor_of_parent_is_not_noop() {
        // frag > parent: the (empty after a move, say) fragment is an ancestor of
        // `parent` → hierarchy error, not a no-op.
        let mut dom = EcsDom::new();
        let frag = dom.create_document_fragment();
        let parent = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(frag, parent);
        assert!(!is_empty_fragment_noop(&dom, parent, frag));
    }

    #[test]
    fn empty_non_fragment_is_not_noop() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let elem = dom.create_element("span", Attributes::default());
        assert!(!is_empty_fragment_noop(&dom, parent, elem));
    }
}
