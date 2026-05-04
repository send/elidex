//! Document host globals — `document.getElementById`,
//! `document.createElement`, `document.createTextNode`, and the
//! getters for `body` / `head` / `documentElement` / `title` / `URL` /
//! `readyState` (WHATWG DOM §4.5 + HTML §3.2.9, §7.1, §12.2.8).
//!
//! # Scope
//!
//! - `getElementById(id)` — pre-order DFS from the document root
//!   (WHATWG DOM §4.2.4 "document descendants").  Uses
//!   `EcsDom::find_by_id`.
//! - `createElement(tag)` / `createTextNode(data)` /
//!   `createComment(data)` / `createDocumentFragment()` — allocate
//!   ECS entities and return their wrappers.  Text wrappers chain
//!   through `Text.prototype → CharacterData.prototype →
//!   Node.prototype → EventTarget.prototype` so `data`, `length`,
//!   `splitText`, `appendData` etc. resolve on the returned handle;
//!   Comment wrappers chain through `CharacterData.prototype`
//!   directly; Fragment wrappers chain through `Node.prototype`.
//! - `body` / `head` / `documentElement` — tree walk from the
//!   document root looking for the first `<html>` child, then within
//!   that for `<head>` / `<body>`.  Phase 2 returns `null` when the
//!   structure is missing rather than synthesising fallback nodes.
//! - `title` (get) — concatenates text children of the first
//!   `<title>` element descending from `<head>`; setter lands with
//!   the rest of the HTMLDocument polish in PR4f.
//! - `URL` / `documentURI` — `VmInner::navigation.current_url`.
//! - `readyState` — stub returning `"complete"` (the VM has no notion
//!   of document loading state yet; the shell owns that in PR6).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::Vm;
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api, parse_dom_selector,
    query_selector_in_subtree_all, query_selector_in_subtree_first, wrap_entities_as_array,
    wrap_entity_or_null,
};

use elidex_ecs::{Entity, NodeKind};

// ---------------------------------------------------------------------------
// Tree walk from the receiver document.
// ---------------------------------------------------------------------------

/// Resolve the target Document entity for a document-method call,
/// with WebIDL branding.
///
/// - `Ok(Some(entity))`: bound HostObject receiver whose kind is
///   `Document` — returned for the bound global and for cloned
///   Document wrappers (so `cloned.querySelector(...)` searches
///   the clone's subtree).
/// - `Ok(None)`: unbound VM or non-HostObject `this` — silent
///   no-op, matching the rest of elidex's unbound-receiver
///   policy (post-unbind retained references must not panic).
/// - `Err(TypeError)`: HostObject `this` whose kind is NOT
///   `Document` (e.g. `docMethod.call(element)`).  WebIDL
///   "Illegal invocation" brand check.
fn document_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    super::event_target::require_receiver(ctx, this, "Document", method, |k| {
        k == NodeKind::Document
    })
}

/// Locate the `<html>` root child of `doc_entity`.  Returns `None`
/// if the document has no `<html>` child (e.g. empty tree).
fn find_html_root_of(ctx: &mut NativeContext<'_>, doc_entity: Entity) -> Option<Entity> {
    let dom = ctx.host().dom();
    dom.first_child_with_tag(doc_entity, "html")
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_element_by_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    // WebIDL brand-check runs BEFORE argument ToString so an invalid
    // receiver does not trigger user-supplied toString.
    let Some(doc) = document_receiver(ctx, this, "getElementById")? else {
        return Ok(JsValue::Null);
    };
    // Spec-precise ToString coercion (Object via `[Symbol.toPrimitive]`
    // / `toString`) runs at the call site — handler's
    // `require_string_arg` would reject `ObjectRef` and cannot
    // reproduce the WebIDL stringifier path.
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "getElementById", doc, &[JsValue::String(target_sid)])
}

// ---------------------------------------------------------------------------
// querySelector / querySelectorAll
// ---------------------------------------------------------------------------

pub(super) fn native_document_query_selector(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    // Brand-check before argument ToString (WebIDL precedence).
    let Some(doc) = document_receiver(ctx, this, "querySelector")? else {
        return Ok(JsValue::Null);
    };
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let syntax_err = ctx.vm.well_known.dom_exc_syntax_error;
    let selectors = parse_dom_selector(&selector_str, "querySelector", syntax_err)?;
    let matched = query_selector_in_subtree_first(ctx.host().dom(), doc, &selectors);
    Ok(wrap_entity_or_null(ctx.vm, matched))
}

pub(super) fn native_document_query_selector_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    // Brand-check before argument ToString (WebIDL precedence).
    let Some(doc) = document_receiver(ctx, this, "querySelectorAll")? else {
        return Ok(JsValue::Null);
    };
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let syntax_err = ctx.vm.well_known.dom_exc_syntax_error;
    let selectors = parse_dom_selector(&selector_str, "querySelectorAll", syntax_err)?;
    let entities = query_selector_in_subtree_all(ctx.host().dom(), doc, &selectors);
    // WHATWG §4.2.6: `querySelectorAll` returns a **static** NodeList.
    // Store the snapshot entity vec; every read re-serves from the
    // cached list (no re-traversal, no filter re-evaluation).
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Snapshot { entities });
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// getElementsByTagName / getElementsByClassName
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_elements_by_tag_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc) = document_receiver(ctx, this, "getElementsByTagName")? else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let tag = coerce_first_arg_to_string(ctx, args)?;
    let tag_sid = ctx.vm.strings.intern(&tag);
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::ByTag {
            root: doc,
            tag: tag_sid,
            all: tag == "*",
        });
    Ok(JsValue::Object(id))
}

pub(super) fn native_document_get_elements_by_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc) = document_receiver(ctx, this, "getElementsByClassName")? else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let class_str = coerce_first_arg_to_string(ctx, args)?;
    let class_names: Vec<_> = class_str
        .split_whitespace()
        .map(|c| ctx.vm.strings.intern(c))
        .collect();
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::ByClass {
            root: doc,
            class_names,
        });
    Ok(JsValue::Object(id))
}

/// `document.getElementsByName(name)` — WHATWG HTML §3.1.5.
/// Returns a **live NodeList** matching every descendant whose
/// `name` content attribute equals the argument (case-sensitive).
pub(super) fn native_document_get_elements_by_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc) = document_receiver(ctx, this, "getElementsByName")? else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    let name_sid = ctx.vm.strings.intern(&name);
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::ByName {
            doc,
            name: name_sid,
        });
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// createElement / createTextNode
// ---------------------------------------------------------------------------
//
// Unbound callers get `null` (not a throw) to match the no-op
// behaviour of `getElementById` / `body` / `head` etc.  In practice
// the only way to reach these natives while unbound is to retain a
// `document` reference across an `unbind()` boundary; throwing
// TypeError would crash the listener instead of producing the same
// "document is detached" semantics that other methods already surface.

pub(super) fn native_document_create_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createElement")? else {
        return Ok(JsValue::Null);
    };
    // ToString at call site; handler does the lowercase normalisation
    // and the "node document" anchoring (WHATWG DOM §4.5 / §4.4).
    let tag_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(
        ctx,
        "createElement",
        doc_entity,
        &[JsValue::String(tag_sid)],
    )
}

pub(super) fn native_document_create_text_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createTextNode")? else {
        return Ok(JsValue::Null);
    };
    let data = coerce_first_arg_to_string(ctx, args)?;
    let new_entity = ctx
        .host()
        .dom()
        .create_text_with_owner(data, Some(doc_entity));
    // Text wrappers chain through `Text.prototype →
    // CharacterData.prototype → Node.prototype → …` so `data` /
    // `length` / `splitText` resolve on the returned handle.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

/// `document.createComment(data)` — WHATWG DOM §4.5.  Allocates a
/// Comment entity with the coerced `data` string and returns its
/// wrapper.  Unbound receiver → `null` (matches the rest of the
/// document method family).
pub(super) fn native_document_create_comment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createComment")? else {
        return Ok(JsValue::Null);
    };
    let data = coerce_first_arg_to_string(ctx, args)?;
    let new_entity = ctx
        .host()
        .dom()
        .create_comment_with_owner(data, Some(doc_entity));
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

/// `document.createDocumentFragment()` — WHATWG DOM §4.5.  Allocates
/// a `DocumentFragment` entity that is **not** linked into any tree
/// and returns its wrapper.  Unbound / non-Document receiver → `null`.
pub(super) fn native_document_create_document_fragment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createDocumentFragment")? else {
        return Ok(JsValue::Null);
    };
    let new_entity = ctx
        .host()
        .dom()
        .create_document_fragment_with_owner(Some(doc_entity));
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

// ---------------------------------------------------------------------------
// Getters: documentElement / head / body / title / URL / readyState
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_document_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let html = document_receiver(ctx, this, "documentElement")?
        .and_then(|doc| find_html_root_of(ctx, doc));
    Ok(wrap_entity_or_null(ctx.vm, html))
}

pub(super) fn native_document_get_head(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let head = document_receiver(ctx, this, "head")?
        .and_then(|doc| find_html_root_of(ctx, doc))
        .and_then(|html| ctx.host().dom().first_child_with_tag(html, "head"));
    Ok(wrap_entity_or_null(ctx.vm, head))
}

pub(super) fn native_document_get_body(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let body = document_receiver(ctx, this, "body")?
        .and_then(|doc| find_html_root_of(ctx, doc))
        .and_then(|html| ctx.host().dom().first_child_with_tag(html, "body"));
    Ok(wrap_entity_or_null(ctx.vm, body))
}

pub(super) fn native_document_get_title(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let title_text: String = {
        let Some(doc) = document_receiver(ctx, this, "title")? else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let Some(html) = find_html_root_of(ctx, doc) else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let dom = ctx.host().dom();
        let Some(head) = dom.first_child_with_tag(html, "head") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let Some(title) = dom.first_child_with_tag(head, "title") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        // Concat all text-node children (matches WHATWG §3.2.9
        // "descendant text content", but for title we only walk
        // immediate children — title cannot legally contain nested
        // elements).
        let mut out = String::new();
        for child in dom.children_iter(title) {
            if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(child) {
                out.push_str(&text.0);
            }
        }
        out
    };
    let sid = ctx.vm.strings.intern(&title_text);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_url(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `current_url` is now a `Url`; `as_str()` produces the
    // canonical WHATWG serialisation (what `document.URL` /
    // `documentURI` returns per WHATWG DOM §4.5.1).
    let url = ctx.vm.navigation.current_url.as_str().to_string();
    let sid = ctx.vm.strings.intern(&url);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_ready_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: scripts run after parse completes (eval is synchronous
    // from the shell's perspective), so `complete` is honest.  The
    // shell wires real lifecycle transitions in PR6.
    Ok(JsValue::String(ctx.vm.well_known.complete))
}

// ---------------------------------------------------------------------------
// title setter / compatMode / defaultView / doctype (PR4f C6)
// ---------------------------------------------------------------------------

/// `document.title = x` — WHATWG §4.5.  For HTML documents:
/// 1. Find the `<title>` inside `<head>` if any.
/// 2. If none exists but `<head>` does, create a new `<title>` and
///    append it to `<head>`.
/// 3. If no `<head>` exists, the setter is a **no-op** per spec.
/// 4. Replace the title element's children with a single Text node
///    containing the coerced string.
pub(super) fn native_document_set_title(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "title")? else {
        return Ok(JsValue::Undefined);
    };
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, value)?;
    let new_text = ctx.vm.strings.get_utf8(value_sid);

    // Locate <head> under <html> for the receiver document.  Spec
    // uses "html root" → "first head child"; legacy html5ever-shaped
    // trees hand us exactly that shape already.
    let Some(html) = find_html_root_of(ctx, doc) else {
        return Ok(JsValue::Undefined);
    };
    let head_opt = ctx.host().dom().first_child_with_tag(html, "head");
    let Some(head) = head_opt else {
        // No <head> — spec is explicit: return without mutating.
        return Ok(JsValue::Undefined);
    };

    // find_or_create_title.  We want a single <title> in <head>; if
    // absent, allocate one and append (with the correct owner
    // document).
    let title = match ctx.host().dom().first_child_with_tag(head, "title") {
        Some(t) => t,
        None => {
            let new_title = ctx.host().dom().create_element_with_owner(
                "title",
                elidex_ecs::Attributes::default(),
                Some(doc),
            );
            let _ = ctx.host().dom().append_child(head, new_title);
            new_title
        }
    };

    // Clear existing text-node children; legal <title> content per
    // WHATWG is text-only but we defensively include Element children
    // too in case a bad parse left some in there.
    let existing: Vec<elidex_ecs::Entity> = ctx.host().dom().children_iter(title).collect();
    for child in existing {
        let _ = ctx.host().dom().remove_child(title, child);
    }
    let text_entity = ctx.host().dom().create_text_with_owner(new_text, Some(doc));
    let _ = ctx.host().dom().append_child(title, text_entity);
    Ok(JsValue::Undefined)
}

/// `document.compatMode` — WHATWG §4.5 accessor.
/// Fixed `"CSS1Compat"` (standards mode); `BackCompat` (quirks mode)
/// requires a quirks-aware parser pass that elidex does not yet have.
pub(super) fn native_document_get_compat_mode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `document_receiver` returns `Ok(None)` for unbound VMs and
    // non-HostObject receivers (silent no-op policy, matches every
    // other document accessor — `body` returns null, `title` returns
    // "", etc.).  Fall through to the empty string in that case so
    // `Object.getOwnPropertyDescriptor(...).get.call({})` does not
    // leak a plausible "CSS1Compat" string from a detached call
    // site.  Wrong-NodeKind receivers still throw TypeError via the
    // `?` — unchanged brand-check contract.
    if document_receiver(ctx, this, "compatMode")?.is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    Ok(JsValue::String(ctx.vm.well_known.css1_compat))
}

/// `document.defaultView` — WHATWG §4.5.  Returns the Window
/// (`globalThis`) wrapper for the bound VM; a Document that is not
/// currently the bound document (e.g. a detached clone) has no
/// associated `Window` and therefore returns `null` per spec.
///
/// The bound `globalThis` is an independently-allocated `HostObject`
/// (`VmInner::global_object`), not an entry in the element
/// `wrapper_cache`.  Returning `global_object` directly preserves
/// WHATWG's `document.defaultView === globalThis` invariant; calling
/// `create_element_wrapper(window_entity)` here would allocate a
/// parallel wrapper that does not compare equal.
pub(super) fn native_document_get_default_view(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "defaultView")? else {
        return Ok(JsValue::Null);
    };
    // Only the bound document has a Window.  Non-bound (cloned)
    // documents are detached from any browsing context.
    let bound_doc = ctx.vm.host_data.as_deref().map(|hd| hd.document());
    if bound_doc != Some(doc) {
        return Ok(JsValue::Null);
    }
    Ok(JsValue::Object(ctx.vm.global_object))
}

/// `document.doctype` — WHATWG §4.5.  The first child whose
/// `NodeKind` is `DocumentType`, or `null`.
pub(super) fn native_document_get_doctype(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "doctype")? else {
        return Ok(JsValue::Null);
    };
    let doctype = {
        let dom = ctx.host().dom();
        let mut found = None;
        for child in dom.children_iter(doc) {
            // `node_kind_inferred` matches the legacy-fallback used
            // by `HostData::prototype_kind_for` and
            // `require_node_arg`: entities that carry `DocTypeData`
            // payload without an explicit `NodeKind` component still
            // surface as doctype.  Keeps html5ever-produced fixtures
            // (which predate the `NodeKind` component) discoverable.
            if matches!(dom.node_kind_inferred(child), Some(NodeKind::DocumentType)) {
                found = Some(child);
                break;
            }
        }
        found
    };
    Ok(wrap_entity_or_null(ctx.vm, doctype))
}

// ---------------------------------------------------------------------------
// cookie / referrer (stubs) + forms / images / links (snapshot arrays)
// ---------------------------------------------------------------------------

/// `document.cookie` getter (WHATWG §6.5.2).  Delegates to
/// [`elidex_net::CookieJar::cookies_for_script`], which is the
/// canonical script-visible filter (`HttpOnly` and Secure-on-non-HTTPS
/// suppression both live there).  When no jar is installed (test
/// harness, standalone VM) we fall back to the cookie-averse path
/// and return `""`.
pub(super) fn native_document_get_cookie(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `document_receiver` returns `Ok(None)` for unbound VMs and
    // non-HostObject receivers (e.g. `getter.call({})`); the
    // brand-bypass cases must observe the cookie-averse fallback.
    // `Ok(Some(doc))` for a non-bound Document (e.g. a clone made
    // by `document.cloneNode(true)`) must also fall back, because
    // browsing-context cookie state belongs to the active Document
    // alone — see the `defaultView` accessor for the same guard.
    let Some(doc) = document_receiver(ctx, this, "cookie")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let bound_doc = ctx.vm.host_data.as_deref().map(|hd| hd.document());
    if bound_doc != Some(doc) {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    // `host_if_bound` borrows `ctx` mutably, but
    // `cookies_for_script(&current_url)` needs `&ctx.vm` at the
    // same time — we release the host borrow by `Arc::clone`'ing
    // the jar (single atomic refcount bump, no heap copy) before
    // reaching back into `ctx.vm`.
    let jar = ctx.host_if_bound().and_then(|hd| hd.cookie_jar()).cloned();
    let value = jar
        .map(|jar| jar.cookies_for_script(&ctx.vm.navigation.current_url))
        .unwrap_or_default();
    if value.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&value);
    Ok(JsValue::String(sid))
}

/// `document.cookie = value` (WHATWG §6.5.2).  Forwards a single
/// `Set-Cookie`-syntax string to
/// [`elidex_net::CookieJar::set_cookie_from_script`], which is the
/// canonical attribute parser (rejecting `HttpOnly` and
/// Secure-over-HTTP per RFC 6265 §5.3).  When no jar is installed
/// the assignment silently no-ops, matching the cookie-averse
/// Document path the spec permits.
pub(super) fn native_document_set_cookie(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Mirror the getter's two-stage guard — non-Document receivers
    // *and* non-bound Document receivers (clones made by
    // `document.cloneNode(true)`) must be unable to mutate the
    // bound browsing context's cookie jar.
    let Some(doc) = document_receiver(ctx, this, "cookie")? else {
        return Ok(JsValue::Undefined);
    };
    let bound_doc = ctx.vm.host_data.as_deref().map(|hd| hd.document());
    if bound_doc != Some(doc) {
        return Ok(JsValue::Undefined);
    }
    // Resolve the jar BEFORE coercing the assigned value: the
    // cookie-averse contract is "silent no-op on assignment", so
    // `document.cookie = Symbol()` must not throw on a VM with no
    // jar installed.  Coercing first would surface `to_string`'s
    // `Symbol → USVString` TypeError where the previous stub was
    // intentionally non-throwing.
    //
    // Cloning the `Arc` (cheap atomic bump) lets the jar outlive
    // the `host_if_bound` mutable borrow so we can read
    // `ctx.vm.navigation` afterwards.
    let Some(jar) = ctx.host_if_bound().and_then(|hd| hd.cookie_jar()).cloned() else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let value = ctx.vm.strings.get_utf8(sid);
    jar.set_cookie_from_script(&ctx.vm.navigation.current_url, &value);
    Ok(JsValue::Undefined)
}

/// `document.referrer` (WHATWG HTML §3.1.5).  Returns the URL of
/// the previous Document if the embedding shell has populated
/// `NavigationState::referrer` via
/// [`super::super::Vm::set_navigation_referrer`]; otherwise the empty
/// string.  The VM does **not** derive a referrer automatically —
/// the shell is the only writer of this slot today.
pub(super) fn native_document_get_referrer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Two-stage guard, matching the cookie accessors and
    // `defaultView`: brand-bypass (non-HostObject `this`) and
    // detached Document clones must both fall back to the empty
    // string.  `NavigationState::referrer` is browsing-context
    // state, owned by the bound Document alone.
    let Some(doc) = document_receiver(ctx, this, "referrer")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let bound_doc = ctx.vm.host_data.as_deref().map(|hd| hd.document());
    if bound_doc != Some(doc) {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let Some(url) = ctx.vm.navigation.referrer.as_ref() else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let sid = ctx.vm.strings.intern(url.as_str());
    Ok(JsValue::String(sid))
}

/// `document.forms` — live `HTMLCollection` of every `<form>`
/// descendant (WHATWG §3.1.5).
pub(super) fn native_document_get_forms(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "forms")? else {
        return Ok(super::dom_bridge::wrap_entities_as_array(ctx.vm, &[]));
    };
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Forms { doc });
    Ok(JsValue::Object(id))
}

/// `document.images` — live `HTMLCollection` of every `<img>`
/// descendant.
pub(super) fn native_document_get_images(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "images")? else {
        return Ok(super::dom_bridge::wrap_entities_as_array(ctx.vm, &[]));
    };
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Images { doc });
    Ok(JsValue::Object(id))
}

/// `document.activeElement` (WHATWG HTML §6.6.3).
///
/// Returns the currently focused Element, or — when no element is
/// focused (or the focused entity has since been detached) — the
/// document's `<body>` per spec step 2.  If neither is available,
/// returns `documentElement` (spec fallback for documents without a
/// body, e.g. during parser construction of the HTML skeleton).
pub(super) fn native_document_get_active_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "activeElement")? else {
        return Ok(JsValue::Null);
    };
    // Resolve the focus target: focused_entity if set and still
    // attached, else <body>, else <html> root (spec fallback for
    // documents without a body).  Mirror `body` / `documentElement`
    // accessors so the fallback chain stays consistent.
    let focused = super::html_element_proto::focused_entity(ctx);
    let target = {
        let dom = ctx.host().dom();
        // A focused element counts only when it remains connected
        // to the document — walking up via `get_parent` must reach
        // `doc`.  Detached (removed) subtrees fall through to the
        // body / root fallback so stale `focused_entity` from a
        // prior `focus()` + `remove()` does not leak.
        let focused_connected = focused.and_then(|e| {
            if !dom.contains(e) {
                return None;
            }
            let mut cur = Some(e);
            while let Some(c) = cur {
                if c == doc {
                    return Some(e);
                }
                cur = dom.get_parent(c);
            }
            None
        });
        focused_connected.or_else(|| {
            let html = find_html_root_of(ctx, doc)?;
            ctx.host()
                .dom()
                .first_child_with_tag(html, "body")
                .or(Some(html))
        })
    };
    Ok(wrap_entity_or_null(ctx.vm, target))
}

/// `document.hasFocus()` (WHATWG HTML §6.7).
///
/// Phase 2 approximation: returns whether an element is currently
/// focused **and** still connected to this document.  Detached
/// focused entities do not count — this mirrors `activeElement`'s
/// connectedness filter so the two APIs agree (`hasFocus() === true`
/// ⇒ `activeElement !== body`).  A full spec implementation tracks
/// system focus at the window level; same-origin Document vs
/// top-level Window focus arbitration is deferred to the PR5d
/// cross-window tranche.
pub(super) fn native_document_has_focus(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "hasFocus")? else {
        return Ok(JsValue::Boolean(false));
    };
    let Some(focused) = super::html_element_proto::focused_entity(ctx) else {
        return Ok(JsValue::Boolean(false));
    };
    let dom = ctx.host().dom();
    if !dom.contains(focused) {
        return Ok(JsValue::Boolean(false));
    }
    let mut cur = Some(focused);
    while let Some(c) = cur {
        if c == doc {
            return Ok(JsValue::Boolean(true));
        }
        cur = dom.get_parent(c);
    }
    Ok(JsValue::Boolean(false))
}

/// `document.links` — live `HTMLCollection` of every `<a>` /
/// `<area>` descendant carrying an `href` attribute (§4.5: anchors
/// without `href` are **excluded** from the collection; the filter
/// runs on read inside `resolve_entities_with_needles`).
pub(super) fn native_document_get_links(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "links")? else {
        return Ok(super::dom_bridge::wrap_entities_as_array(ctx.vm, &[]));
    };
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Links { doc });
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Installation — add the own-properties to the document wrapper at bind time.
// ---------------------------------------------------------------------------

impl Vm {
    /// Re-install the document-only own-properties each bind cycle.
    /// Called by `install_document_global` after the HostObject
    /// wrapper has been refreshed.
    ///
    /// We re-install rather than once-at-VM-init because the
    /// document wrapper is cached per-Entity in
    /// `HostData::wrapper_cache` — successive binds with the same
    /// document entity return the same wrapper ObjectId, so
    /// properties survive without any work, but the first bind
    /// *must* populate it.  The "already installed" check below
    /// keeps rebind cycles O(1).
    pub(in crate::vm) fn install_document_methods_if_needed(&mut self, doc_wrapper: ObjectId) {
        // The bound document is the default target; cloned Document
        // entities are installed separately via
        // `VmInner::install_document_methods_for_entity` (invoked from
        // `native_node_clone_node` after allocating the clone's
        // wrapper).
        let doc_entity = self
            .host_data()
            .expect("install_document_methods_if_needed requires HostData")
            .document();
        self.inner
            .install_document_methods_for_entity(doc_entity, doc_wrapper);
    }
}

impl super::super::VmInner {
    /// Populate the document-specific own-property suite onto
    /// `doc_wrapper`, keyed by `doc_entity` so repeated bind / clone
    /// cycles skip the install.  Used by both
    /// [`Vm::install_document_methods_if_needed`](super::super::Vm::install_document_methods_if_needed)
    /// (bound document, each bind) and
    /// `native_node_clone_node` (cloned Document entities).
    ///
    /// The per-entity idempotency set is load-bearing: a single VM
    /// can host multiple Document entities over its lifetime (bound
    /// document + clones), each with a distinct wrapper ObjectId, and
    /// every one of them needs the install exactly once.
    pub(in crate::vm) fn install_document_methods_for_entity(
        &mut self,
        doc_entity: elidex_ecs::Entity,
        doc_wrapper: ObjectId,
    ) {
        let already_installed = self
            .host_data
            .as_deref()
            .is_some_and(|hd| hd.document_methods_installed.contains(&doc_entity));
        if already_installed {
            return;
        }
        self.install_methods(doc_wrapper, DOCUMENT_METHODS);
        self.install_ro_accessors(doc_wrapper, DOCUMENT_RO_ACCESSORS);
        self.install_rw_accessors(doc_wrapper, DOCUMENT_RW_ACCESSORS);
        // ParentNode mixin (WHATWG §5.2.4) shared with
        // `Element.prototype`.
        self.install_parent_node_mixin(doc_wrapper);
        if let Some(hd) = self.host_data.as_deref_mut() {
            hd.document_methods_installed.insert(doc_entity);
        }
    }
}

// Method + accessor tables are file-scope constants so they are not
// rebuilt on every bind and so the `install_document_methods_if_needed`
// body reads top-down.
const DOCUMENT_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("getElementById", native_document_get_element_by_id),
    ("querySelector", native_document_query_selector),
    ("querySelectorAll", native_document_query_selector_all),
    (
        "getElementsByTagName",
        native_document_get_elements_by_tag_name,
    ),
    (
        "getElementsByClassName",
        native_document_get_elements_by_class_name,
    ),
    ("getElementsByName", native_document_get_elements_by_name),
    ("createElement", native_document_create_element),
    ("createTextNode", native_document_create_text_node),
    ("createComment", native_document_create_comment),
    (
        "createDocumentFragment",
        native_document_create_document_fragment,
    ),
    // PR5b §C1: focus-management readers.  `hasFocus()` returns
    // whether any element is currently focused (Phase 2: whether
    // `HostData::focused_entity` is `Some`).  Spec §6.7 defines
    // hasFocus in terms of the system focus — we approximate as
    // "some element inside this Document has focus".
    ("hasFocus", native_document_has_focus),
];

const DOCUMENT_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("documentElement", native_document_get_document_element),
    ("head", native_document_get_head),
    ("body", native_document_get_body),
    ("URL", native_document_get_url),
    ("documentURI", native_document_get_url),
    ("readyState", native_document_get_ready_state),
    ("compatMode", native_document_get_compat_mode),
    ("defaultView", native_document_get_default_view),
    ("doctype", native_document_get_doctype),
    // PR4f C7 stubs / snapshots — see native-fn docstrings for defer
    // targets (PR6 / PR5b).
    ("referrer", native_document_get_referrer),
    ("forms", native_document_get_forms),
    ("images", native_document_get_images),
    ("links", native_document_get_links),
    // PR5b §C1 — `activeElement` returns the focused Element (or
    // `body` when no element is focused, per WHATWG §6.6.3 step 2).
    ("activeElement", native_document_get_active_element),
];

/// Read/write Document accessors.  `title` is WHATWG-backed; `cookie`
/// is currently a stub whose setter silently drops writes (see the
/// setter docstring for the PR6 integration path).
const DOCUMENT_RW_ACCESSORS: &[(&str, super::super::NativeFn, super::super::NativeFn)] = &[
    (
        "title",
        native_document_get_title,
        native_document_set_title,
    ),
    (
        "cookie",
        native_document_get_cookie,
        native_document_set_cookie,
    ),
];
