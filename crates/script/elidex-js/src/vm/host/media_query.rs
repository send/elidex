//! `MediaQueryList` interface + `window.matchMedia` (CSSOM-View §4.2 /
//! §4 Extensions to the Window Interface).
//!
//! `MediaQueryList` is an `EventTarget` that is *not* a `Node`, so its
//! prototype chain mirrors `Window` / `AbortSignal`:
//!
//! ```text
//! MediaQueryList instance (ObjectKind::MediaQueryList)
//!   → MediaQueryList.prototype   (this module)
//!     → EventTarget.prototype    (no Node members)
//!       → Object.prototype
//! ```
//!
//! ## State storage
//!
//! Per-MQL state ([`MediaQueryEntry`]) lives **out of band** in
//! [`VmInner::media_query_list_registry`], keyed by the MQL's own
//! `ObjectId`, so [`ObjectKind::MediaQueryList`] stays payload-free
//! (per-variant size discipline, matching `AbortSignal`). The entry is a
//! plain `{parsed: MediaQueryList, last_matches: bool}` — `ObjectId`- and
//! `JsValue`-free — so GC needs only a sweep-prune (no trace pass) and the
//! registry survives `Vm::unbind` (it binds to no DOM entity); see the
//! `ObjectKind::MediaQueryList` doc for the full canonical contract.
//!
//! ## Evaluator (engine-independent SSoT)
//!
//! Parse / evaluate / serialize all live in `elidex_css::media` (Slice 1
//! #360 + Slice 2a #364); this module only **marshals**: JS string ↔ query,
//! build the MQL wrapper, snapshot `last_matches`, and surface the
//! interface over the unified EventTarget core. No media-query algorithm
//! runs here (Layering mandate).
//!
//! ## Listener model
//!
//! `MediaQueryList` is a full member of the unified EventTarget dispatch
//! core: `addEventListener('change', …)` / `removeEventListener` /
//! `dispatchEvent` are **inherited** from `EventTarget.prototype` (routed
//! to its `vm_event_listeners` home via `DispatchTarget::VmObject`).
//! `onchange` is an event-handler IDL attribute bound to the `'change'`
//! type. The legacy `addListener` / `removeListener` (CSSOM-View §4.2, kept
//! "for web compat") are thin aliases over `addEventListener('change')` /
//! `removeEventListener('change')` — One-issue-one-way, no duplicate
//! listener bookkeeping.
//!
//! The `change` event delivered on a media-state flip is a
//! `MediaQueryListEvent`; that dispatched subclass + the host-driven
//! report-changes fire land with the `HostDriver` transport in Slice 2b-ii
//! (this slice wires the interface; delivery is exercised here via
//! `dispatchEvent`).

#![cfg(feature = "engine")]

use elidex_css::media::{evaluate, parse_media_query_list, MediaEnvironment, MediaQueryList};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

/// Per-`MediaQueryList` state, owned by
/// [`VmInner::media_query_list_registry`] and looked up via the MQL's
/// `ObjectId`.
///
/// `matches` is *derived* (`evaluate(&parsed, &env)`); `last_matches` is
/// only the snapshot the `.matches` getter reads and the flip-detection
/// prior for the Slice 2b-ii report-changes algorithm — never a competing
/// source of truth.
#[derive(Debug)]
pub(crate) struct MediaQueryEntry {
    /// The parsed query (engine-independent AST, #360). Serialized on
    /// demand by the `.media` getter via `Display` (#364).
    pub(crate) parsed: MediaQueryList,
    /// Snapshot of the last evaluation result. Seeded at `matchMedia`
    /// time; updated only on a flip by the 2b-ii transport.
    pub(crate) last_matches: bool,
}

// ---------------------------------------------------------------------------
// Registration (called from register_globals)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `MediaQueryList.prototype`, install its accessors /
    /// methods, and expose the non-constructable `MediaQueryList` global.
    ///
    /// Called from `register_globals()` **after**
    /// [`Self::register_event_target_prototype`] (the prototype chains
    /// directly to `event_target_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` is `None` — would mean
    /// `register_event_target_prototype` was skipped or run out of order.
    pub(in crate::vm) fn register_media_query_list_global(&mut self) {
        let event_target_proto = self.event_target_prototype.expect(
            "register_media_query_list_global called before register_event_target_prototype",
        );

        // ---- MediaQueryList.prototype ----
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_media_query_list_accessors(proto_id);
        self.install_media_query_list_methods(proto_id);
        self.media_query_list_prototype = Some(proto_id);

        // ---- MediaQueryList global ----
        // WebIDL: `MediaQueryList` declares NO constructor (instances come
        // only from `window.matchMedia()`), so `new MediaQueryList()` /
        // `MediaQueryList()` throw a TypeError. Registered as an
        // illegal-constructor so `mql instanceof MediaQueryList` and
        // `MediaQueryList.prototype` parity still work — the `AbortSignal`
        // precedent.
        let ctor = self.create_illegal_constructor_function(
            "MediaQueryList",
            super::super::value::native_illegal_constructor_unreachable,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name = self.strings.intern("MediaQueryList");
        self.globals.insert(name, JsValue::Object(ctor));
    }

    fn install_media_query_list_accessors(&mut self, proto_id: ObjectId) {
        // `matches` / `media` are RO accessors (CSSOM-View §4.2). Reuse the
        // shared `matches` well-known; `media` is media-specific.
        for (name_sid, getter) in [
            (
                self.well_known.matches,
                native_media_query_list_get_matches as NativeFn,
            ),
            (
                self.well_known.media,
                native_media_query_list_get_media as NativeFn,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None::<NativeFn>,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        // `onchange` event-handler IDL attribute over the shared VmObject
        // event-handler backend, bound key = the `'change'` event-type SID
        // (mirrors `AbortSignal::onabort`).
        let onchange_sid = self.well_known.onchange;
        let change_event_sid = self.well_known.change;
        self.install_bound_accessor_pair(
            proto_id,
            onchange_sid,
            super::event_handler_attrs::native_vm_event_handler_get as NativeFn,
            Some(super::event_handler_attrs::native_vm_event_handler_set as NativeFn),
            change_event_sid,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_media_query_list_methods(&mut self, proto_id: ObjectId) {
        // `addEventListener` / `removeEventListener` / `dispatchEvent` are
        // INHERITED from `EventTarget.prototype`. Only the legacy
        // `addListener` / `removeListener` aliases live here.
        let add_sid = self.strings.intern("addListener");
        let remove_sid = self.strings.intern("removeListener");
        self.install_native_method(
            proto_id,
            add_sid,
            native_media_query_list_add_listener,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            remove_sid,
            native_media_query_list_remove_listener,
            PropertyAttrs::METHOD,
        );
    }

    /// Allocate a fresh `MediaQueryList` instance with its state row in
    /// [`Self::media_query_list_registry`]. Used by `matchMedia` — never
    /// directly callable from JS (`new MediaQueryList()` throws TypeError).
    pub(in crate::vm) fn create_media_query_list(
        &mut self,
        parsed: MediaQueryList,
        last_matches: bool,
    ) -> ObjectId {
        let proto = self.media_query_list_prototype;
        let id = self.alloc_object(Object {
            kind: ObjectKind::MediaQueryList,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        self.media_query_list_registry.insert(
            id,
            MediaQueryEntry {
                parsed,
                last_matches,
            },
        );
        id
    }

    /// Build the [`MediaEnvironment`] the evaluator reads, derived from the
    /// `VmInner::viewport` SoT (the single transported device-facts struct).
    ///
    /// Shared by the `matchMedia` initial-`matches` path and (Slice 2b-ii)
    /// the report-changes re-eval, so there is one env-builder + one
    /// evaluator (#360). `medium` is always `Screen` (matchMedia is a screen
    /// document); `color_scheme` / `reduced_motion` use the #360 defaults
    /// until the 2b-ii `ViewportState` extension sources them from the
    /// transport.
    pub(in crate::vm) fn media_environment(&self) -> MediaEnvironment {
        MediaEnvironment {
            viewport_width: self.viewport.inner_width,
            viewport_height: self.viewport.inner_height,
            resolution_dppx: self.viewport.device_pixel_ratio,
            ..MediaEnvironment::default()
        }
    }
}

// ---------------------------------------------------------------------------
// window.matchMedia
// ---------------------------------------------------------------------------

/// `window.matchMedia(query)` — CSSOM-View §4
/// (`#dom-window-matchmedia`). Parses `query` (total parser, #360), builds a
/// live `MediaQueryList`, snapshots the initial `matches` against the
/// current environment, and returns the MQL. Marshalling only — parse /
/// evaluate are `elidex_css::media` calls.
pub(super) fn native_window_match_media(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL: `query` is a *required* argument, so a 0-arg call throws
    // (arity convention shared with `structuredClone` / `CSS.supports`),
    // rather than coercing a missing arg to the `"undefined"` query.
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to execute 'matchMedia' on 'Window': 1 argument required, but only 0 present.",
        ));
    }
    // `query` is a `CSSOMString` → ToString-coerced at the IDL boundary.
    let arg = args[0];
    let query_sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let query = ctx.vm.strings.get_utf8(query_sid);

    // Engine-independent parse + evaluate (#360). `parse_media_query_list`
    // is total (malformed → `not all`; unknown feature → Kleene-unknown →
    // false), so there is no throw path.
    let parsed = parse_media_query_list(&query);
    let env = ctx.vm.media_environment();
    let matches = evaluate(&parsed, &env);

    let id = ctx.vm.create_media_query_list(parsed, matches);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// MediaQueryList accessors
// ---------------------------------------------------------------------------

/// Resolve `this` to a `MediaQueryList` `ObjectId`, or `TypeError`.
fn require_media_query_list_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "MediaQueryList.prototype.{member} called on non-MediaQueryList"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::MediaQueryList) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "MediaQueryList.prototype.{member} called on non-MediaQueryList"
        )))
    }
}

/// `mql.matches` (RO) — the last evaluation snapshot. Absent entry →
/// `false` (defensive-by-construction; matches the `AbortSignal.aborted`
/// safe-default for a cleared/collected side-table slot).
fn native_media_query_list_get_matches(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_media_query_list_this(ctx, this, "matches")?;
    let matches = ctx
        .vm
        .media_query_list_registry
        .get(&id)
        .is_some_and(|e| e.last_matches);
    Ok(JsValue::Boolean(matches))
}

/// `mql.media` (RO) — the serialized/canonical query text (#364 `Display`).
/// Absent entry → `""` (safe default, `AbortSignal.reason` precedent).
fn native_media_query_list_get_media(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_media_query_list_this(ctx, this, "media")?;
    let serialized = ctx
        .vm
        .media_query_list_registry
        .get(&id)
        .map(|e| e.parsed.to_string());
    let sid = match serialized {
        Some(s) => ctx.vm.strings.intern(&s),
        None => ctx.vm.strings.intern(""),
    };
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// Legacy MediaQueryList.addListener / removeListener (CSSOM-View §4.2)
// ---------------------------------------------------------------------------

/// `mql.addListener(callback)` — legacy alias for
/// `addEventListener('change', callback)` (CSSOM-View §4.2, kept for web
/// compat). Routes through the unified EventTarget core so dedupe /
/// registration order match `addEventListener` exactly.
fn native_media_query_list_add_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let change_sid = ctx.vm.well_known.change;
    super::event_target::native_event_target_add_event_listener(
        ctx,
        this,
        &[JsValue::String(change_sid), callback],
    )
}

/// `mql.removeListener(callback)` — legacy alias for
/// `removeEventListener('change', callback)`.
fn native_media_query_list_remove_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let change_sid = ctx.vm.well_known.change;
    super::event_target::native_event_target_remove_event_listener(
        ctx,
        this,
        &[JsValue::String(change_sid), callback],
    )
}
