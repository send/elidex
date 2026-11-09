//! Document accessor natives — `documentElement` / `head` / `body` /
//! `title` / `URL` / `readyState` / `compatMode` / `defaultView` /
//! `doctype` / `cookie` / `referrer` / `forms` / `images` /
//! `activeElement` / `hasFocus` / `hidden` / `visibilityState` /
//! `links` / `baseURI` / `documentURI`.  Pure-move split from
//! `document.rs` (C0 D-14 (b)); the accessor tables stay in
//! `document.rs`.
//!
//! Where each accessor's logic lives:
//! - `body` / `head` / `documentElement` — handlers walk the tree
//!   from the receiver document; return `null` when the structure is
//!   missing (no synthesised fallback).
//! - `title` (get/set) — handlers walk `<html>` → `<head>` → `<title>`;
//!   getter applies WHATWG whitespace normalisation; setter creates
//!   `<title>` if missing or no-ops if `<head>` is absent.
//! - `doctype` — handler returns the first `DocumentType` child via
//!   `EcsDom::node_kind_inferred` (legacy `DocTypeData` fallback
//!   preserved).
//! - `URL` / `documentURI` — `VmInner::navigation.current_url`
//!   (navigation state, not DOM).
//! - `readyState` — brand-checked, answers `"complete"` for every Document
//!   (slot proposal `#11-document-readiness-one-source`: the bound
//!   document's readiness has a second, unused source).
//! - Browsing-context facts (`defaultView`, `cookie`, `referrer`, `hidden`,
//!   `visibilityState`, `hasFocus`, `getSelection`) consult
//!   [`document_has_browsing_context`].
//! - `cookie` / `referrer` — `elidex-net::CookieJar` /
//!   `NavigationState`, kept VM-side because they read browsing-context
//!   state.
//! - `forms` / `images` / `links` — VM-side natives construct
//!   [`elidex_dom_api::LiveCollection`] directly; the walker /
//!   cache / case-insensitive matching live engine-independent in
//!   that crate.
//! - `activeElement` / `hasFocus` — focus state from `HostData`,
//!   carved out in `#11-focus-management-hoist`.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, VmError};
use super::document::document_receiver;
use super::dom_bridge::{invoke_dom_api, wrap_entity_or_null};
use elidex_ecs::Entity;

/// Whether `doc` has a browsing context (HTML §7.3.2): in this VM a Document's
/// node navigable is non-null exactly when it is the bound document
/// (`HostData::document`).  The one place the VM's single browsing-context
/// ↔ Document association is read; its storage stays per-VM until B1
/// (`#11-browsing-context-state-ecs-components`).
pub(super) fn document_has_browsing_context(ctx: &NativeContext<'_>, doc: Entity) -> bool {
    ctx.vm
        .host_data
        .as_deref()
        .and_then(super::super::host_data::HostData::document_entity_opt)
        == Some(doc)
}

/// Whether `doc`'s visibility state is "hidden" (WHATWG HTML §6.2): a Document
/// without a browsing context keeps its initial "hidden" (*set the initial
/// visibility state* runs only for a navigable's document); the bound one
/// follows the tab (`HostData::is_tab_hidden`).
fn document_is_hidden(ctx: &NativeContext<'_>, doc: Entity) -> bool {
    if !document_has_browsing_context(ctx, doc) {
        return true;
    }
    ctx.vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_tab_hidden)
}

/// Locate the `<html>` root child of `doc_entity`.  Returns `None`
/// if the document has no `<html>` child (e.g. empty tree).
fn find_html_root_of(ctx: &mut NativeContext<'_>, doc_entity: Entity) -> Option<Entity> {
    let dom = ctx.host().dom();
    dom.first_child_with_tag(doc_entity, "html")
}

/// Brand-check `this` as a Document and dispatch to a no-arg
/// `DomApiHandler` by name.
///
/// Three outcomes (matching `document_receiver` / `require_receiver`):
/// - `this` is the bound Document (or any other HostObject whose
///   `NodeKind` is `Document`, e.g. a cloned doc) → dispatch to
///   `handler_name`.
/// - `this` is a HostObject while the VM is unbound → return `fallback`
///   (the existing default, PM 556 D2).  A non-HostObject `this` throws
///   `TypeError` (PM 556 D1), bound or not.
/// - `this` is a HostObject of a different `NodeKind` (e.g.
///   `Document.documentElement.get.call(element)`) → propagates
///   the WebIDL "Illegal invocation" `TypeError` from
///   `document_receiver`.  This is **not** swallowed into `fallback`;
///   re-using this helper without that branding contract produces
///   a different brand-check shape.
fn invoke_document_accessor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    handler_name: &'static str,
    fallback: JsValue,
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, method)? else {
        return Ok(fallback);
    };
    invoke_dom_api(ctx, handler_name, doc, &[])
}

// ---------------------------------------------------------------------------
// Getters: documentElement / head / body / title / URL / readyState
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_document_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    invoke_document_accessor(
        ctx,
        this,
        "documentElement",
        "document.documentElement.get",
        JsValue::Null,
    )
}

pub(super) fn native_document_get_head(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    invoke_document_accessor(ctx, this, "head", "document.head.get", JsValue::Null)
}

pub(super) fn native_document_get_body(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    invoke_document_accessor(ctx, this, "body", "document.body.get", JsValue::Null)
}

pub(super) fn native_document_get_title(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = JsValue::String(ctx.vm.well_known.empty);
    invoke_document_accessor(ctx, this, "title", "document.title.get", empty)
}

/// `URL` / `documentURI` share one value (`VmInner::navigation.current_url`,
/// R-10 in the C1-pre memo); the brand check is independent of that store, so
/// each attribute getter runs `document_receiver` under its own name.
fn document_url_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    name: &str,
) -> Result<JsValue, VmError> {
    document_receiver(ctx, this, name)?;
    // `current_url` is a `Url`; `as_str()` produces the canonical WHATWG
    // serialisation (what `document.URL` / `documentURI` returns per WHATWG
    // DOM §4.5).
    let url = ctx.vm.navigation.current_url.as_str().to_string();
    let sid = ctx.vm.strings.intern(&url);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    document_url_value(ctx, this, "URL")
}

pub(super) fn native_document_get_document_uri(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    document_url_value(ctx, this, "documentURI")
}

/// `document.baseURI` getter (D-31; WHATWG DOM §4.4 Interface Node
/// `baseURI` getter, anchor `#dom-node-baseuri`).  Routes through the
/// dom-api `document.baseURI.get` handler which reads the cached
/// `DocumentBaseUrl` component maintained by `BaseUrlMaintainer`.
pub(super) fn native_document_get_base_uri(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = JsValue::String(ctx.vm.well_known.empty);
    invoke_document_accessor(ctx, this, "baseURI", "document.baseURI.get", empty)
}

pub(super) fn native_document_get_ready_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Web IDL *attribute getter* brand check (no `[LegacyLenientThis]` on
    // `readyState`): `document_receiver` throws `TypeError` for a receiver
    // that is not a Document platform object.  A Document answers its current
    // document readiness (HTML §3.1.5): HEAD's constant for the bound one (its
    // readiness has a second source, slot `#11-document-readiness-one-source`),
    // the initial value `"complete"` for any other (no per-Document store; a
    // DOMParser document's spec value is "interactive", the same slot).  The
    // native never returns `""`.
    document_receiver(ctx, this, "readyState")?;
    Ok(JsValue::String(ctx.vm.well_known.complete))
}

// ---------------------------------------------------------------------------
// title setter / compatMode / defaultView / doctype
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
    invoke_dom_api(
        ctx,
        "document.title.set",
        doc,
        &[JsValue::String(value_sid)],
    )
}

/// `document.compatMode` — WHATWG DOM §4.5 accessor, routed to the engine-independent
/// `GetCompatMode` handler (the mode is not modelled, so every Document is
/// `"CSS1Compat"`; slot proposal `#11-document-mode`).  An unbound
/// `HostObject` receiver answers the empty string rather than leaking a
/// plausible "CSS1Compat" from a detached call site; a non-`HostObject` or
/// wrong-kind receiver throws `TypeError`.
pub(super) fn native_document_get_compat_mode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = JsValue::String(ctx.vm.well_known.empty);
    invoke_document_accessor(ctx, this, "compatMode", "document.compatMode.get", empty)
}

/// `document.defaultView` — WHATWG HTML §7.2.2 The Window object.  Returns the Window
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
    if !document_has_browsing_context(ctx, doc) {
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
    invoke_document_accessor(ctx, this, "doctype", "doctype.get", JsValue::Null)
}

// ---------------------------------------------------------------------------
// cookie / referrer + forms / images / links (snapshot arrays)
// ---------------------------------------------------------------------------

/// `document.cookie` getter (WHATWG HTML §3.1.4).  Delegates to
/// [`elidex_net::CookieJar::cookies_for_script`], which is the
/// canonical script-visible filter (`HttpOnly` and Secure-on-non-HTTPS
/// suppression both live there).  When no jar is installed (test
/// harness, standalone VM) we fall back to the cookie-averse path
/// and return `""`.
// `compat-webapi`-gated (A3): `document.cookie` is `Legacy`, so the accessor glue
// is compiled out of `App` builds (the `CookieJar` itself stays — HTTP cookies +
// `navigator.cookieEnabled` need it in every mode).
#[cfg(feature = "compat-webapi")]
pub(super) fn native_document_get_cookie(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `document_receiver` returns `Ok(None)` for an unbound VM's HostObject
    // receiver (a non-HostObject such as `getter.call({})` throws); that case
    // must observe the cookie-averse fallback.
    // `Ok(Some(doc))` for a non-bound Document (e.g. a clone made
    // by `document.cloneNode(true)`) must also fall back, because
    // browsing-context cookie state belongs to the active Document
    // alone — see the `defaultView` accessor for the same guard.
    let Some(doc) = document_receiver(ctx, this, "cookie")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if !document_has_browsing_context(ctx, doc) {
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

/// `document.cookie = value` (WHATWG HTML §3.1.4).  Forwards a single
/// `Set-Cookie`-syntax string to
/// [`elidex_net::CookieJar::set_cookie_from_script`], which is the
/// canonical attribute parser (rejecting `HttpOnly` and
/// Secure-over-HTTP per RFC 6265 §5.3).  When no jar is installed
/// the assignment silently no-ops, matching the cookie-averse
/// Document path the spec permits.
// `compat-webapi`-gated (A3): see [`native_document_get_cookie`].
#[cfg(feature = "compat-webapi")]
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
    if !document_has_browsing_context(ctx, doc) {
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

/// `document.referrer` (WHATWG HTML §3.1.4).  Returns the URL of
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
    // `defaultView`: an unbound VM's HostObject `this` and
    // detached Document clones must both fall back to the empty
    // string.  `NavigationState::referrer` is browsing-context
    // state, owned by the bound Document alone.
    let Some(doc) = document_receiver(ctx, this, "referrer")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if !document_has_browsing_context(ctx, doc) {
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
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::Forms,
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
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
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::Images,
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}

/// `document.activeElement` (WHATWG HTML §6.6.6 Focus management APIs).
///
/// Returns the currently focused Element, or — when no element is
/// focused (or the focused entity has since been detached) — the
/// document's `<body>` per the spec fallback.  If neither is available,
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
    // Resolve the focus target: the focused element (read from the
    // canonical `ElementState::FOCUS` component via the engine-indep
    // `current_focus`, which applies the connectedness filter — a
    // detached element's stale bit does not leak), else <body>, else
    // <html> root (spec fallback for documents without a body).  Mirror
    // `body` / `documentElement` accessors so the fallback chain stays
    // consistent.
    let target = {
        let dom = ctx.host().dom();
        let focused_connected = elidex_dom_api::focus::current_focus(dom, doc);
        focused_connected.or_else(|| {
            let html = find_html_root_of(ctx, doc)?;
            // Spec fallback: first body-or-frameset child of `<html>`,
            // else `<html>`.  Reuses the engine-independent
            // `first_body_or_frameset_child` helper that `GetBody` calls
            // so the two accessors stay locked together (PR #156 R6
            // drifted them once when the body|frameset find was
            // inlined here as a two-pass that lost document order).
            elidex_dom_api::char_data::first_body_or_frameset_child(ctx.host().dom(), html)
                .or(Some(html))
        })
    };
    Ok(wrap_entity_or_null(ctx.vm, target))
}

/// `document.hasFocus()` (WHATWG HTML §6.6.6 Focus management APIs; the
/// has-focus steps are §6.6.4 Processing model).
///
/// The has-focus steps are based on the top-level browsing context having
/// **system focus**, not merely an element-level focused area. So:
/// - A **hidden** top-level browsing context (background / minimized tab, driven
///   by `HostData::set_visibility`) has no system focus → `hasFocus()` is
///   `false` even while an element retains the focused area. `activeElement`
///   still reports that retained element (focus is preserved across tab
///   switches), which is why ONLY this read is visibility-gated, not
///   `activeElement` (Codex S2 final pass).
/// - Otherwise read the canonical `ElementState::FOCUS` bit via `current_focus`
///   (its connectedness filter keeps `hasFocus() === true` ⇒ `activeElement
///   !== body`).
///
/// The remaining gap — a *visible* context that lacks system focus because
/// another window is focused — needs a window-level focus signal the VM does
/// not model (single browsing context); deferred to slot
/// `#11-system-window-focus-events` (the OS window focus/blur signal).
pub(super) fn native_document_has_focus(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "hasFocus")? else {
        return Ok(JsValue::Boolean(false));
    };
    // HTML §6.6.4 has-focus steps 1-2: no node navigable -> false; then a
    // hidden top-level browsing context has no system focus.
    if document_is_hidden(ctx, doc) {
        return Ok(JsValue::Boolean(false));
    }
    let has = elidex_dom_api::focus::current_focus(ctx.host().dom(), doc).is_some();
    Ok(JsValue::Boolean(has))
}

/// `document.hidden` (WHATWG HTML §6.2 `#dom-document-hidden`).
///
/// For the bound document: `true` when its top-level browsing context is
/// hidden (background tab / minimized / occluded), driven by the embedding
/// shell via `HostData::set_visibility`.  A Document without a browsing
/// context (a DOMParser / `cloneNode` document) answers its initial state,
/// `true`.  A retained wrapper called while the VM is unbound keeps the
/// default `false` (visible).  A receiver that is not a Document throws
/// `TypeError`.
pub(super) fn native_document_get_hidden(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Two-stage guard (matching `cookie` / `referrer` / `defaultView`): an
    // unbound VM's HostObject receiver falls back to the default (visible); a
    // Document without a browsing context answers its initial visibility state,
    // "hidden" (HTML §6.2 *set the initial visibility state* runs only for a
    // navigable's document), never the bound tab's state.
    let Some(doc) = document_receiver(ctx, this, "hidden")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(document_is_hidden(ctx, doc)))
}

/// `document.visibilityState` (WHATWG HTML §6.2 `#dom-document-visibilitystate`).
///
/// `"hidden"` when the page is hidden, else `"visible"`.  The spec
/// `VisibilityState` enum is binary (the `prerender` state was removed),
/// so this mirrors `document.hidden`.
pub(super) fn native_document_get_visibility_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Two-stage guard (see `native_document_get_hidden`): a Document without a
    // browsing context answers the initial "hidden"; an unbound VM's HostObject
    // receiver defaults to "visible".
    let Some(doc) = document_receiver(ctx, this, "visibilityState")? else {
        return Ok(JsValue::String(ctx.vm.well_known.visible));
    };
    let sid = if document_is_hidden(ctx, doc) {
        ctx.vm.well_known.hidden
    } else {
        ctx.vm.well_known.visible
    };
    Ok(JsValue::String(sid))
}

/// `document.links` — live `HTMLCollection` of every `<a>` /
/// `<area>` descendant carrying an `href` attribute (§4.5: anchors
/// without `href` are **excluded** from the collection; the filter
/// runs on read inside [`elidex_dom_api::LiveCollection`]'s
/// populate path).
pub(super) fn native_document_get_links(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "links")? else {
        return Ok(super::dom_bridge::wrap_entities_as_array(ctx.vm, &[]));
    };
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::Links,
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}

/// `document.getSelection()` (Selection API §4.1 `Selection? getSelection()`):
/// brand-check `this`; a Document without a browsing context has no selection
/// (`null`); otherwise the Window's selection.
pub(super) fn native_document_get_selection(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "getSelection")? else {
        return Ok(JsValue::Null);
    };
    if !document_has_browsing_context(ctx, doc) {
        return Ok(JsValue::Null);
    }
    Ok(JsValue::Object(ctx.vm.alloc_or_cached_selection()))
}
