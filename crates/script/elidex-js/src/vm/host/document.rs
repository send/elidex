//! Document host globals — the shared `document_receiver` brand check,
//! `document.getElementById` / `querySelector` / `querySelectorAll` /
//! `getElementsBy*`, and the install code + method/accessor tables.
//!
//! # Module layout
//!
//! The node factories (`createElement` / `createTextNode` /
//! `createComment` / `createDocumentFragment`) live in
//! [`super::document_factories`]; the property accessors (the full list
//! is in the [`super::document_accessors`] module doc) live in
//! [`super::document_accessors`].  This file keeps the shared
//! `document_receiver`, the id / selector / `getElementsBy*` lookups,
//! and the install code + method/accessor tables.
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
//! - `getElementsByTagName` / `ClassName` / `Name` — VM-side natives
//!   construct [`elidex_dom_api::LiveCollection`] directly; the walker /
//!   cache / case-insensitive matching live engine-independent in
//!   that crate.
//!
//! The create* natives and the property accessors (`title`, `doctype`,
//! `cookie`, `forms` / `images` / `links`, `activeElement`, ...) are
//! documented in their own modules (see Module layout above).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
    query_selector_all_snapshot,
};

use super::document_accessors::{
    native_document_get_active_element, native_document_get_base_uri, native_document_get_body,
    native_document_get_compat_mode, native_document_get_default_view, native_document_get_doctype,
    native_document_get_document_element, native_document_get_forms, native_document_get_head,
    native_document_get_hidden, native_document_get_images, native_document_get_links,
    native_document_get_ready_state, native_document_get_referrer, native_document_get_selection,
    native_document_get_title, native_document_get_url, native_document_get_visibility_state,
    native_document_has_focus, native_document_set_title,
};
#[cfg(feature = "compat-webapi")]
use super::document_accessors::{native_document_get_cookie, native_document_set_cookie};
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
/// with WebIDL branding.  The one brand check of every Document member
/// (PM 556 D1): Web IDL §3.7.6 *attribute getter* / §3.7.7 *create an
/// operation function* throw `TypeError` when `this` is not a Document
/// platform object, and no Document member but `onreadystatechange` has
/// `[LegacyLenientThis]`.
///
/// - `Err(TypeError)`: `this` is not a `HostObject` (`{}`, `undefined`, a
///   primitive, a function), bound or not — the `this_is_node_wrapper`
///   pre-check (`require_receiver`'s doc names it for this case) — or a
///   bound `HostObject` whose kind is NOT `Document` (e.g.
///   `docMethod.call(element)`).  WebIDL "Illegal invocation" brand check.
/// - `Ok(Some(entity))`: bound HostObject receiver whose kind is
///   `Document` — returned for the bound global and for cloned
///   Document wrappers (so `cloned.querySelector(...)` searches
///   the clone's subtree).
/// - `Ok(None)`: unbound VM with a `HostObject` receiver — the member
///   falls through to its existing default (PM 556 D2; a retained
///   wrapper cannot be classified once unbound, and post-unbind retained
///   references must not panic).
///
/// Callers must call this BEFORE any early return of their own (such as a
/// bound check), so the non-`HostObject` `TypeError` also fires while
/// unbound.
pub(super) fn document_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    if !super::event_target::this_is_node_wrapper(ctx.vm, this) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Document': Illegal invocation"
        )));
    }
    super::event_target::require_receiver(ctx, this, "Document", method, |k| {
        k == NodeKind::Document
    })
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_element_by_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
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
    let Some(doc) = document_receiver(ctx, this, "getElementsByTagName")? else {
        return Ok(JsValue::Null);
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
    let Some(doc) = document_receiver(ctx, this, "getElementsByClassName")? else {
        return Ok(JsValue::Null);
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
    let Some(doc) = document_receiver(ctx, this, "getElementsByName")? else {
        return Ok(JsValue::Null);
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
// adoptNode
// ---------------------------------------------------------------------------

/// `document.adoptNode(node)` — DOM §4.5 (`#dom-document-adoptnode`),
/// `[CEReactions]` (HTML §4.13.6, `#cereactions`). Marshalling only: the
/// receiver brand check, WebIDL's `Node` conversion of the argument (a
/// non-Node → `TypeError`), then the `adoptNode` handler
/// (`elidex_dom_api::AdoptNode`: steps 1–4, the Document /
/// shadow-root rejections, the adopt and its DOM §4.2.3 *remove* step 16
/// record), whose `DomApiError` the bridge maps to its DOMException.
pub(super) fn native_document_adopt_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = document_receiver(ctx, this, "adoptNode")? else {
        return Ok(JsValue::Null);
    };
    let node_arg = args.first().copied();
    super::node_proto::require_node_arg_required(ctx, node_arg, "Document", "adoptNode")?;
    invoke_dom_api(
        ctx,
        "adoptNode",
        doc,
        &[node_arg.unwrap_or(JsValue::Undefined)],
    )
}

// ---------------------------------------------------------------------------
// Installation — the one `Document.prototype` (WebIDL §3.7.3, §3.7.6, §3.7.7).
// ---------------------------------------------------------------------------

impl super::super::VmInner {
    /// Allocate `Document.prototype` (`[[Prototype]]` = `Node.prototype`,
    /// WebIDL §3.7.3 step 3) and install the whole Document member suite on
    /// it, once per Window VM.  `create_element_wrapper` chooses it for every
    /// wrapper of a `NodeKind::Document` entity (`PrototypeKind::Document`),
    /// so no path — the bound global, DOMParser, `cloneNode`,
    /// `ownerDocument`, the adopted-reaction flush — installs anything
    /// per wrapper.  Must run after `register_node_prototype`.
    ///
    /// The members install in three ordered slices so the gated
    /// live-collection getters land at their original ordinal position
    /// (between `querySelectorAll` and `createElement`): `install_methods`
    /// appends shape entries in call order, so install order IS the
    /// `Object.getOwnPropertyNames(Object.getPrototypeOf(document))` order,
    /// and a single trailing gated install would reorder the
    /// live-collection names after `getSelection` (an observable
    /// enumeration-order change; Codex R9).  The gates read
    /// `spec_level_policy`, fixed at VM construction, so the gated set
    /// equals the set a per-wrapper install would have produced.
    pub(in crate::vm) fn register_document_prototype(&mut self) {
        let node_proto = self
            .node_prototype
            .expect("register_document_prototype before register_node_prototype");
        let doc_wrapper = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(node_proto),
            extensible: true,
        });
        self.document_prototype = Some(doc_wrapper);
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
        self.install_event_handler_attrs(
            doc_wrapper,
            &[
                elidex_script_session::HandlerScope::Global,
                elidex_script_session::HandlerScope::DocumentElement,
                elidex_script_session::HandlerScope::DocumentOnly,
            ],
        );
    }
}

// Method + accessor tables are file-scope constants so they are not
// rebuilt on every bind and so `register_document_prototype`'s
// body reads top-down.
/// `Document.prototype` methods installed BEFORE the gated live-collection getters, kept
/// in their original `DOCUMENT_METHODS` order (id + selector lookups). Split from
/// [`DOCUMENT_METHODS_POST_LIVE_COLLECTION`] so the gated
/// [`DOCUMENT_LIVE_COLLECTION_METHODS`] install lands at its original ordinal
/// position — property enumeration order = install order (see the install
/// comment in `register_document_prototype`; Codex R9).
const DOCUMENT_METHODS_PRE_LIVE_COLLECTION: &[(&str, super::super::NativeFn)] = &[
    ("getElementById", native_document_get_element_by_id),
    ("querySelector", native_document_query_selector),
    ("querySelectorAll", native_document_query_selector_all),
];

/// `Document.prototype` methods installed AFTER the gated live-collection getters (node
/// factories + focus / selection readers + `adoptNode`), kept in their original
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
    ("getSelection", native_document_get_selection),
    // DOM §4.5 `adoptNode(node)`; appended so the existing names keep
    // their enumeration order.
    ("adoptNode", native_document_adopt_node),
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
    // `referrer` / `forms` / `images` / `links` — see the native-fn docstrings.
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
