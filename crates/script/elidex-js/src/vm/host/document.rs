//! Document host globals — `document.getElementById`,
//! `document.createElement`, `document.createTextNode`, and the
//! getters for `body` / `head` / `documentElement` / `title` / `URL` /
//! `readyState` (`body`/`head`/`title` = WHATWG HTML §3.1.7 DOM tree
//! accessors; `documentElement`/`URL` = WHATWG DOM §4.5 Interface Document;
//! `readyState` = HTML §3.1.5 Reporting document loading status).
//!
//! # Module layout
//!
//! The node factories (`createElement` / `createTextNode` /
//! `createComment` / `createDocumentFragment`) live in
//! [`super::document_factories`]; the property accessors
//! (`documentElement` / `head` / `body` / `title` / `URL` / `readyState` /
//! `compatMode` / `defaultView` / `doctype` / `cookie` / `referrer` /
//! `forms` / `images` / `activeElement` / `hasFocus` / `hidden` /
//! `visibilityState` / `links`) in [`super::document_accessors`].  This
//! file keeps the shared helpers, the id / selector / `getElementsBy*`
//! lookups, and the install code + method/accessor tables.
//!
//! # Scope
//!
//! Most natives in this module are thin marshalling shims over the
//! engine-independent `DomApiHandler`s in
//! `crates/dom/elidex-dom-api/src/{document,char_data/document_props}.rs`
//! (see [`super::dom_bridge::invoke_dom_api`]).  The shape of every
//! migrated native is identical: brand-check the receiver via
//! [`document_receiver`], short-circuit on unbound / non-Document,
//! coerce the first arg if any, and dispatch by handler name.
//!
//! - `getElementById(id)` — handler `"getElementById"`; pre-order DFS
//!   from the document root (WHATWG DOM §4.2.4 "document descendants").
//! - `querySelector` / `querySelectorAll` — handler `"querySelector"`
//!   plus the engine-independent free function
//!   `elidex_dom_api::query_selector_all` (the latter because handlers
//!   cannot return `Vec<Entity>`).  Both reject `:host` /
//!   `::slotted()` (WHATWG Selectors §1.2).
//! - `createElement` / `createTextNode` / `createComment` /
//!   `createDocumentFragment` — handlers anchor the new node's
//!   "node document" (WHATWG DOM §4.4) to the receiver Document so
//!   `clonedDoc.create*().ownerDocument === clonedDoc`.  Text
//!   wrappers chain through `Text.prototype → CharacterData.prototype
//!   → Node.prototype → EventTarget.prototype`; Comment wrappers
//!   through `CharacterData.prototype`; Fragment wrappers through
//!   `Node.prototype`.
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
//! - `readyState` — stub returning `"complete"` (the VM has no notion
//!   of document loading state yet; the shell owns that in PR6).
//! - `cookie` / `referrer` — `elidex-net::CookieJar` /
//!   `NavigationState`, kept VM-side because they read browsing-context
//!   state.
//! - `forms` / `images` / `links` / `getElementsByTagName` /
//!   `ClassName` / `Name` — VM-side natives construct
//!   [`elidex_dom_api::LiveCollection`] directly; the walker /
//!   cache / case-insensitive matching live engine-independent in
//!   that crate.
//! - `activeElement` / `hasFocus` — focus state from `HostData`,
//!   carved out in `#11-focus-management-hoist`.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::Vm;
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
    query_selector_all_snapshot, wrap_entities_as_array,
};

use super::document_accessors::{
    native_document_get_active_element, native_document_get_base_uri, native_document_get_body,
    native_document_get_compat_mode, native_document_get_cookie, native_document_get_default_view,
    native_document_get_doctype, native_document_get_document_element, native_document_get_forms,
    native_document_get_head, native_document_get_hidden, native_document_get_images,
    native_document_get_links, native_document_get_ready_state, native_document_get_referrer,
    native_document_get_title, native_document_get_url, native_document_get_visibility_state,
    native_document_has_focus, native_document_set_cookie, native_document_set_title,
};
use super::document_factories::{
    native_document_create_comment, native_document_create_document_fragment,
    native_document_create_element, native_document_create_text_node,
};
use elidex_ecs::{Entity, NodeKind};
use elidex_script_session::live_collection_spec_level;
// Read only by the (compat-webapi-gated) `document.cookie` accessor install (A3).
#[cfg(feature = "compat-webapi")]
use elidex_script_session::document_cookie_spec_level;

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
pub(super) fn document_receiver(
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
pub(super) fn find_html_root_of(ctx: &mut NativeContext<'_>, doc_entity: Entity) -> Option<Entity> {
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
/// - `this` is unbound / non-HostObject → return `fallback` (silent
///   no-op, matches the rest of the document accessor family).
/// - `this` is a HostObject of a different `NodeKind` (e.g.
///   `Document.documentElement.get.call(element)`) → propagates
///   the WebIDL "Illegal invocation" `TypeError` from
///   `document_receiver`.  This is **not** swallowed into `fallback`;
///   re-using this helper without that branding contract produces
///   a different brand-check shape.
pub(super) fn invoke_document_accessor(
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
    // ToString at call site to honour the WebIDL stringifier path
    // (handler's `require_string_arg` rejects `ObjectRef`).
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "querySelector", doc, &[JsValue::String(target_sid)])
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
    // WHATWG §4.2.6: `querySelectorAll` returns a **static** NodeList.
    // The handler protocol cannot return `Vec<Entity>`, so this opts
    // out of `invoke_dom_api` and uses the engine-independent free
    // function instead.
    query_selector_all_snapshot(ctx, doc, &selector_str)
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
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::ByTagName(tag),
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
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
    // ASCII whitespace per WHATWG DOM §4.2.6.2 + HTML §2.4.5.3
    // ("ordered set parser"). Matches the element-side splitter in
    // `LiveCollection`'s `ByClassNames` matcher; `split_whitespace`
    // would split on Unicode whitespace (e.g. NBSP) the spec
    // doesn't recognise as a token boundary.
    let class_names: Vec<String> = class_str
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect();
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::ByClassNames(class_names),
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}

/// `document.getElementsByName(name)` — WHATWG HTML §3.1.7 (DOM tree accessors).
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
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        doc,
        elidex_dom_api::CollectionFilter::ByName(name),
        // WHATWG HTML §3.1.7: getElementsByName returns NodeList.
        elidex_dom_api::CollectionKind::NodeList,
    ));
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
        // The document own-methods install in three ordered slices so the gated
        // live-collection getters land at their ORIGINAL ordinal position (between
        // `querySelectorAll` and `createElement`). elidex installs document methods
        // as the wrapper's OWN properties (no shared `Document.prototype`) and
        // `install_methods` appends shape entries in call order, so install order IS
        // the `Object.getOwnPropertyNames(document)` order — a single *trailing*
        // gated install would reorder the live-collection names after `getSelection`,
        // an observable enumeration-order change A1's no-behavior-change contract
        // must not make (Codex R9).
        self.install_methods(doc_wrapper, DOCUMENT_METHODS_PRE_LIVE_COLLECTION);
        // Seam-1c (A1 core/compat gate): the live-collection getters route through
        // the general `installs_dom(level)` predicate reading the family's SINGLE
        // source `live_collection_spec_level()` (Codex R7). A1's source is `Living`
        // (no API moves). B0/B1 demote the family by flipping that one source AND
        // route the rest of the family — `forms`/`images`/`links`,
        // ParentNode `children`, `Element.prototype` getters, `table.rows`,
        // `select.options` — through the **same** source (the full surface sweep is
        // B0's, A0 §5 B0 row; this gate is the representative `Document` seam, not
        // the whole family).
        if self.installs_dom(live_collection_spec_level()) {
            self.install_methods(doc_wrapper, DOCUMENT_LIVE_COLLECTION_METHODS);
        }
        self.install_methods(doc_wrapper, DOCUMENT_METHODS_POST_LIVE_COLLECTION);
        // WHATWG DOM §4.4 / §6.1 / §6.4 traversal factories.  Slot
        // `#11-traversal-and-range-pr-a2-bindings`.  Installed
        // separately to keep the document method tables stable.
        self.install_methods(doc_wrapper, super::document_traversal::FACTORIES);
        self.install_ro_accessors(doc_wrapper, DOCUMENT_RO_ACCESSORS);
        self.install_rw_accessors(doc_wrapper, DOCUMENT_RW_ACCESSORS);
        // Seam-1b (A1 core/compat gate): `document.cookie` routes through the
        // general `installs(level)` predicate reading its single source
        // `document_cookie_spec_level()`; A3 demoted that one source to `Legacy`
        // (HTML §3.1.4), so the accessor + its natives are now `compat-webapi`-gated
        // — present under `BrowserCompat` (byte-identical), dropped from `App`
        // builds. The `CookieJar` itself stays always-compiled (HTTP cookies +
        // `navigator.cookieEnabled` read it in every mode).
        #[cfg(feature = "compat-webapi")]
        if self.installs(document_cookie_spec_level()) {
            self.install_rw_accessors(doc_wrapper, DOCUMENT_COOKIE_RW_ACCESSOR);
        }
        // ParentNode mixin (WHATWG §5.2.4) shared with
        // `Element.prototype`.
        self.install_parent_node_mixin(doc_wrapper);
        // Event-handler IDL attributes (WHATWG HTML §8.1.8.2.1):
        // Document mixes in GlobalEventHandlers +
        // DocumentAndElementEventHandlers + the Document-specific
        // partial (`onreadystatechange` / `onvisibilitychange`).
        // Installed per-entity on the wrapper (no shared
        // `Document.prototype` in elidex), gated by the same
        // idempotency set.
        self.install_event_handler_attrs(
            doc_wrapper,
            &[
                elidex_script_session::HandlerScope::Global,
                elidex_script_session::HandlerScope::DocumentElement,
                elidex_script_session::HandlerScope::DocumentOnly,
            ],
        );
        if let Some(hd) = self.host_data.as_deref_mut() {
            hd.document_methods_installed.insert(doc_entity);
        }
    }
}

// Method + accessor tables are file-scope constants so they are not
// rebuilt on every bind and so the `install_document_methods_if_needed`
// body reads top-down.
/// Document own-methods installed BEFORE the gated live-collection getters, kept
/// in their original `DOCUMENT_METHODS` order (id + selector lookups). Split from
/// [`DOCUMENT_METHODS_POST_LIVE_COLLECTION`] so the gated
/// [`DOCUMENT_LIVE_COLLECTION_METHODS`] install lands at its original ordinal
/// position — own-property enumeration order = install order (see the install
/// comment in `install_document_methods_for_entity`; Codex R9).
const DOCUMENT_METHODS_PRE_LIVE_COLLECTION: &[(&str, super::super::NativeFn)] = &[
    ("getElementById", native_document_get_element_by_id),
    ("querySelector", native_document_query_selector),
    ("querySelectorAll", native_document_query_selector_all),
];

/// Document own-methods installed AFTER the gated live-collection getters (node
/// factories + focus / selection readers), kept in their original
/// `DOCUMENT_METHODS` order — see [`DOCUMENT_METHODS_PRE_LIVE_COLLECTION`].
const DOCUMENT_METHODS_POST_LIVE_COLLECTION: &[(&str, super::super::NativeFn)] = &[
    ("createElement", native_document_create_element),
    ("createTextNode", native_document_create_text_node),
    ("createComment", native_document_create_comment),
    (
        "createDocumentFragment",
        native_document_create_document_fragment,
    ),
    // Focus-management readers (WHATWG HTML §6.6.6).  `hasFocus()`
    // returns whether some connected element in this Document carries
    // the canonical `ElementState::FOCUS` bit (read via
    // `elidex_dom_api::focus::current_focus`).  The spec defines
    // hasFocus in terms of system focus — we approximate as "some
    // element inside this Document has focus".
    ("hasFocus", native_document_has_focus),
    // Selection API §2: `getSelection()` on Document mirrors the
    // Window-side binding; both resolve to the same singleton
    // wrapper held in `HostData::selection_instance`.
    ("getSelection", super::window::native_window_get_selection),
];

/// Seam-1c of the A1 Web-API core/compat gate: the `Document` live-collection
/// getters, extracted from [`DOCUMENT_METHODS`] so their JS-property **install**
/// can be gated by one [`installs_dom`](super::super::VmInner::installs_dom)
/// guard (the install seam is the *property-absence* lever — these getters
/// allocate a live `HTMLCollection`/`NodeList` directly and have no
/// `DomApiHandler`, so the registry seam does not reach them).
///
/// A1 routes them at the live-collection family's single source
/// `live_collection_spec_level()` ([`Living`](elidex_plugin::DomSpecLevel::Living)
/// in A1 — no API moves, installed in every mode). **B0/B1 own the `Legacy`
/// decision** (flip that one source) and the full-family sweep
/// (`forms`/`images`/`links`/`children` + `Element.prototype` getters /
/// `table.rows` / `select.options` / … — sites outside this `Document` set,
/// routed through the **same** source). Spec homes differ:
/// `getElementsByTagName`/`getElementsByClassName` = DOM §4.5;
/// `getElementsByName` = **HTML §3.1.7** (DOM tree accessors) — B0 cites each home.
const DOCUMENT_LIVE_COLLECTION_METHODS: &[(&str, super::super::NativeFn)] = &[
    (
        "getElementsByTagName",
        native_document_get_elements_by_tag_name,
    ),
    (
        "getElementsByClassName",
        native_document_get_elements_by_class_name,
    ),
    ("getElementsByName", native_document_get_elements_by_name),
];

const DOCUMENT_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("documentElement", native_document_get_document_element),
    ("head", native_document_get_head),
    ("body", native_document_get_body),
    // ParentNode mixin (WHATWG §4.2.6) RO accessors — Document has
    // no shared prototype; selector pair stays in `DOCUMENT_METHODS`.
    (
        "firstElementChild",
        super::parentnode::native_pn_first_element_child,
    ),
    (
        "lastElementChild",
        super::parentnode::native_pn_last_element_child,
    ),
    ("children", super::parentnode::native_pn_children),
    (
        "childElementCount",
        super::parentnode::native_pn_child_element_count,
    ),
    ("URL", native_document_get_url),
    ("documentURI", native_document_get_url),
    ("baseURI", native_document_get_base_uri),
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
    // `activeElement` returns the focused Element (or `body` when no
    // element is focused, per WHATWG HTML §6.6.6 Focus management APIs).
    ("activeElement", native_document_get_active_element),
    // WHATWG HTML §6.2 Page visibility — `document.hidden` /
    // `document.visibilityState`, backed by `HostData::tab_hidden`
    // (shell-driven via `HostDriver::set_visibility`).
    ("hidden", native_document_get_hidden),
    ("visibilityState", native_document_get_visibility_state),
    // CSSOM §6.8 — `document.styleSheets`.  Returns a fresh
    // `StyleSheetList` wrapper; not `[SameObject]` (matches Chrome).
    (
        "styleSheets",
        super::cssom_sheet::native_document_get_style_sheets,
    ),
];

/// Read/write Document accessors.  `title` is WHATWG-backed.  (`document.cookie`
/// was extracted into [`DOCUMENT_COOKIE_RW_ACCESSOR`] for the A1 core/compat gate
/// — seam-1b.)
const DOCUMENT_RW_ACCESSORS: &[(&str, super::super::NativeFn, super::super::NativeFn)] = &[(
    "title",
    native_document_get_title,
    native_document_set_title,
)];

/// Seam-1b of the A1 Web-API core/compat gate: `document.cookie`, extracted from
/// [`DOCUMENT_RW_ACCESSORS`] so its JS-property **install** can be gated by one
/// [`installs`](super::super::VmInner::installs) guard (the install seam is the
/// absence lever). A1 routes it at its single source `document_cookie_spec_level()`
/// ([`Modern`](elidex_plugin::WebApiSpecLevel::Modern) in A1 — no API moves,
/// installed in every mode); **A3** flips that one source to
/// [`Legacy`](elidex_plugin::WebApiSpecLevel::Legacy) (HTML §3.1.4) — a pure
/// one-source level-flip, no table re-extraction. `compat-webapi`-gated (A3): the
/// table + its natives compile out of `App` builds, the `CookieJar` stays.
#[cfg(feature = "compat-webapi")]
const DOCUMENT_COOKIE_RW_ACCESSOR: &[(&str, super::super::NativeFn, super::super::NativeFn)] = &[(
    "cookie",
    native_document_get_cookie,
    native_document_set_cookie,
)];
