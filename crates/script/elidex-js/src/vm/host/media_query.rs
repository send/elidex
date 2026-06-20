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
//! (per-variant size discipline, matching `AbortSignal`). The entry holds
//! ONLY the parsed query (`ObjectId`- and `JsValue`-free) — so GC needs only
//! a sweep-prune (no trace pass) and the registry survives `Vm::unbind` (it
//! binds to no DOM entity); see the `ObjectKind::MediaQueryList` doc for the
//! full canonical contract.
//!
//! ## Evaluator (engine-independent SSoT)
//!
//! Parse / evaluate / serialize all live in `elidex_css::media` (Slice 1
//! #360 + Slice 2a #364); this module only **marshals**: JS string ↔ query,
//! build the MQL wrapper, and surface the interface over the unified
//! EventTarget core. `.matches` is evaluated **live** on each read (derived,
//! never stored — §6), so it always reflects the current environment. No
//! media-query algorithm runs here (Layering mandate).
//!
//! ## Listener model
//!
//! `MediaQueryList` is a full member of the unified EventTarget dispatch
//! core: `addEventListener('change', …)` / `removeEventListener` /
//! `dispatchEvent` are **inherited** from `EventTarget.prototype` (routed
//! to its `vm_event_listeners` home via `DispatchTarget::VmObject`).
//! `onchange` is an event-handler IDL attribute bound to the `'change'`
//! type. The legacy `addListener` / `removeListener` (CSSOM-View §4.2,
//! "basically aliases for `addEventListener`/`removeEventListener`" kept "for
//! backwards compatibility") are **out-of-core** per the core/compat/
//! deprecated tiering (docs/design §14.1.1 / §14.4.2): superseded-by-modern
//! web APIs live in a future compat layer, not the strict core. Modern
//! `addEventListener` / `onchange` ARE the core surface.
//!
//! `MediaQueryListEvent` (CSSOM-View §4.2) — the `change` event type — IS
//! exposed here (Window-only, constructible: `new MediaQueryListEvent(type,
//! {matches, media})`), built as `ObjectKind::Event` + a precomputed shape
//! (no own brand, MessageEvent precedent — lesson #276). The host-driven
//! report-changes *fire* (transport → flip → dispatch) is
//! [`VmInner::deliver_media_query_changes`] (Slice 2b-ii), driven from the
//! shell's update-the-rendering step after a `set_media_environment` push.

#![cfg(feature = "engine")]

use elidex_css::media::{evaluate, parse_media_query_list, MediaEnvironment, MediaQueryList};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::event_target_dispatch_vm::fire_vm_event;
use super::events::EventInit;

/// Per-`MediaQueryList` state, owned by
/// [`VmInner::media_query_list_registry`] and looked up via the MQL's
/// `ObjectId`.
///
/// Holds the parsed query plus the **last reported** match result. `.matches`
/// is still **derived live** on each get (`evaluate(&parsed,
/// &media_environment())`), never read from `last_matches` — so the getter
/// always reflects the current environment by construction (§6 "matches
/// derived, never stored as truth"; Codex R2). `last_matches` is a **separate
/// concern**: the prior-state the report-changes algorithm (CSSOM-View §4.2
/// "evaluate media queries and report changes") compares against to fire
/// `change` **only on a flip**. It is the *last value delivered to listeners*,
/// not a `.matches` cache — the two intentionally diverge between deliver
/// turns (a mid-turn env change moves `.matches` immediately while
/// `last_matches` holds until the next `deliver_media_query_changes`).
#[derive(Debug)]
pub(crate) struct MediaQueryEntry {
    /// The parsed query (engine-independent AST, #360). Evaluated live by
    /// the `.matches` getter; serialized on demand by `.media` via `Display`
    /// (#364).
    pub(crate) parsed: MediaQueryList,
    /// The match result last delivered to `change` listeners — the
    /// flip-detection prior for `deliver_media_query_changes`. Seeded at
    /// `create_media_query_list` to the initial evaluation so the first
    /// deliver after a no-op env change fires nothing.
    pub(crate) last_matches: bool,
    /// Per-registry **monotonic creation sequence** (from
    /// [`VmInner::media_query_list_next_seq`]), assigned once at
    /// `create_media_query_list` and never reused.
    ///
    /// The MQL's `ObjectId` is NOT a stable creation identity: the GC
    /// free-list (`VmInner::alloc_object` → `free_objects.pop()`) recycles
    /// a collected MQL's slot index for the next allocation, so `ObjectId`
    /// order can invert creation order and a recycled slot can masquerade
    /// as a still-live entry. `seq` is the recycle-immune identity (the
    /// `observer_id`/`ws_next_conn_id` monotonic-`u64` precedent), used for
    /// two things in `deliver_media_query_changes`:
    /// - **report order** (CSSOM-View §4.2 requires creation order) — the
    ///   flip set sorts by `seq`, not `ObjectId`;
    /// - **liveness identity** — phase B re-checks that the entry still
    ///   carries the snapshotted `seq` before firing, so a slot recycled
    ///   into a *different* MQL mid-dispatch is skipped rather than fired at.
    pub(crate) seq: u64,
    /// The [`HostData::bind_epoch`] captured at construction — the MQL's
    /// **associated-document tag** (CSSOM-View §4.2 reports changes only for
    /// `MediaQueryList`s whose associated document is the target document).
    ///
    /// The registry intentionally survives `Vm::unbind` (the value is
    /// DOM-free, `abort_signal_states` parity), but a retained MQL belongs
    /// to the document it was created in. `bind_epoch` is bumped on every
    /// `unbind`, so `deliver_media_query_changes` filters to entries whose
    /// `bind_epoch` equals the current epoch — a prior-document MQL is
    /// inert for the new document's report-changes pass instead of being
    /// re-evaluated against (and firing its old listener into) a foreign
    /// document. This is the canonical "the `bind_epoch` mechanism
    /// invalidates stale retained wrappers instead of dropping them"
    /// contract (`vm_api.rs` unbind, `StaticRange` precedent).
    pub(crate) bind_epoch: u32,
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
        // `addEventListener` / `removeEventListener` / `dispatchEvent` are
        // INHERITED from `EventTarget.prototype`; `onchange` is installed by
        // the accessor pass above. The legacy `addListener` / `removeListener`
        // aliases (CSSOM-View §4.2 "for backwards compatibility", superseded
        // by `addEventListener`) are OUT-OF-CORE per the core/compat/deprecated
        // tiering (docs/design §14.1.1 / §14.4.2; Codex R2) — they'd land in a
        // future compat layer, not here. So there are no own methods to install.
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

        // `MediaQueryListEvent` (CSSOM-View §4.2) — the sibling `change`
        // event type. Window-only (this fn is Window-gated); chains to
        // `Event.prototype`. Constructible independently of the 2b-ii
        // host-fire, so the exposed surface is consistent (Codex R2).
        self.register_media_query_list_event_global();
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

    /// Allocate a fresh `MediaQueryList` instance with its state row in
    /// [`Self::media_query_list_registry`]. Used by `matchMedia` — never
    /// directly callable from JS (`new MediaQueryList()` throws TypeError).
    pub(in crate::vm) fn create_media_query_list(&mut self, parsed: MediaQueryList) -> ObjectId {
        // Seed the flip-prior with the initial evaluation (against the current
        // env) so the first `deliver_media_query_changes` only fires `change`
        // if the environment has actually moved since construction.
        let last_matches = evaluate(&parsed, &self.media_environment());
        // Recycle-immune creation identity + associated-document tag —
        // captured BEFORE `alloc_object` (which may recycle a collected
        // `ObjectId` slot, the very hazard `seq` defends against). `matchMedia`
        // only runs in bound JS, so `host_data` is `Some`; `map_or(0, …)` is a
        // defensive default for the unreachable unbound path.
        let seq = self.media_query_list_next_seq;
        self.media_query_list_next_seq = self.media_query_list_next_seq.wrapping_add(1);
        let bind_epoch = self
            .host_data
            .as_deref()
            .map_or(0, super::super::host_data::HostData::bind_epoch);
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
                seq,
                bind_epoch,
            },
        );
        id
    }

    /// Build the [`MediaEnvironment`] the evaluator reads, derived entirely
    /// from the `VmInner::viewport` SoT (the single transported device-facts
    /// struct — viewport geometry + dppx + `color_scheme` + `reduced_motion`).
    ///
    /// Shared by the `matchMedia` initial-`matches` path, the live `.matches`
    /// getter, AND the report-changes re-eval (`deliver_media_query_changes`),
    /// so there is one env-builder + one evaluator (#360). `medium` is always
    /// `Screen` (matchMedia is a screen document; the `Print` medium is the
    /// Slice 3 `@media` cascade, `#11-css-at-media-cascade`); `root_font_size_px`
    /// / `color_bits` keep the #360 defaults (no transport producer yet — em/rem
    /// fidelity `#11-media-css-values-fidelity`, color-depth
    /// `#11-media-extended-features`).
    pub(in crate::vm) fn media_environment(&self) -> MediaEnvironment {
        MediaEnvironment {
            viewport_width: self.viewport.inner_width,
            viewport_height: self.viewport.inner_height,
            resolution_dppx: self.viewport.device_pixel_ratio,
            color_scheme: self.viewport.color_scheme,
            reduced_motion: self.viewport.reduced_motion,
            ..MediaEnvironment::default()
        }
    }

    /// CSSOM-View §4.2 "evaluate media queries and report changes" — the
    /// per-turn report-changes pass. Re-evaluates every live `MediaQueryList`
    /// **created for the current document** (`bind_epoch` filter — the
    /// registry survives unbind, so it can hold prior-document MQLs; §4.2
    /// scopes the pass to associated-document MQLs) against the current
    /// [`Self::media_environment`] and, for each whose result has **flipped**
    /// since its last delivery, updates `last_matches` and fires a trusted
    /// `change` ([`MediaQueryListEvent`]) at the MQL.
    ///
    /// Mirrors the `deliver_resize_observations` / `deliver_sw_client_update`
    /// host→VM delivery shape: a no-op while unbound (no JS context to fire
    /// into — the registry itself SURVIVES unbind, only the firing is gated),
    /// a `NativeContext` for the dispatch, and a trailing microtask
    /// checkpoint. The shell drives this from its update-the-rendering step
    /// after pushing new device facts via
    /// [`set_media_environment`](crate::vm::Vm::set_media_environment) (the
    /// shell producer wiring is carved to S5 / `#11-media-prefers-features`;
    /// VM tests drive it directly).
    ///
    /// Flip detection is snapshotted up front (phase A, **`seq` order** —
    /// CSSOM-View §4.2 creation order; the `ObjectId` key is recycle-prone)
    /// so a listener that calls `matchMedia` or mutates listeners during
    /// dispatch (phase B) cannot perturb this turn's set — a newly-created
    /// MQL is seeded to the current env and simply isn't in the flip list,
    /// and a slot recycled mid-dispatch is caught by the phase-B `seq`
    /// identity re-check.
    pub(in crate::vm) fn deliver_media_query_changes(&mut self) {
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }

        // CSSOM-View §4.2 reports changes only for `MediaQueryList`s whose
        // associated document is the target document. The registry survives
        // unbind (DOM-free), so it can hold MQLs from a *prior* document; the
        // `bind_epoch` captured at creation tags each entry's document, and
        // only current-epoch entries participate (the `StaticRange`
        // invalidate-don't-drop contract). `is_bound` passed above, so
        // `host_data` is `Some`.
        let current_epoch = self
            .host_data
            .as_deref()
            .map_or(0, super::super::host_data::HostData::bind_epoch);

        // Phase A: snapshot the flip set. Evaluation is side-effect-free, so
        // iterate the `HashMap` in any order and sort only the (usually empty)
        // flip set by the **monotonic `seq`** — CSSOM-View §4.2 reports in
        // creation order, and the `ObjectId` key cannot serve that role (the
        // GC free-list recycles it, so it can invert creation order). The
        // common no-flip turn pays no sort.
        let env = self.media_environment();
        let mut flips: Vec<(ObjectId, u64, bool, String)> = Vec::new();
        for (&id, entry) in &self.media_query_list_registry {
            if entry.bind_epoch != current_epoch {
                continue;
            }
            let now = evaluate(&entry.parsed, &env);
            if now != entry.last_matches {
                flips.push((id, entry.seq, now, entry.parsed.to_string()));
            }
        }
        flips.sort_unstable_by_key(|&(_, seq, _, _)| seq);
        if !flips.is_empty() {
            // Update each flipped entry's reported prior BEFORE firing
            // (CSSOM-View §4.2 sets the MQL's value to `now` before queuing the
            // `change`), so a listener that re-reads is consistent and a
            // re-entrant deliver is a no-op for these entries. (No JS runs in
            // this loop, so the `seq` guard is belt-and-suspenders, kept for
            // symmetry with the phase-B identity re-check.)
            for &(id, seq, now, _) in &flips {
                if let Some(e) = self.media_query_list_registry.get_mut(&id) {
                    if e.seq == seq {
                        e.last_matches = now;
                    }
                }
            }

            // Phase B: fire `change` (a trusted `MediaQueryListEvent`) at each
            // flipped MQL through the unified EventTarget dispatch core.
            let mut ctx = NativeContext::new_call(self);
            let shape = ctx
                .vm
                .precomputed_event_shapes
                .as_ref()
                .expect("precomputed_event_shapes built during VM init")
                .media_query_list_event;
            let proto = ctx.vm.media_query_list_event_prototype;
            let change_sid = ctx.vm.well_known.change;
            for (id, seq, now, media) in flips {
                // Per-id liveness + identity re-check (mirrors
                // `deliver_to_observer_callbacks`' per-id binding lookup): an
                // earlier `change` listener this turn may have dropped this MQL,
                // and a GC during dispatch may then have collected its
                // `ObjectId` AND the free-list recycled the slot into a *new*
                // `matchMedia()` object. `contains_key` alone would then be true
                // for the recycled entry and fire this stale snapshot at the
                // wrong MQL — so re-check the snapshotted `seq` (recycle-immune)
                // and skip unless it is still the *same* entry.
                if ctx.vm.media_query_list_registry.get(&id).map(|e| e.seq) != Some(seq) {
                    continue;
                }
                // `media` is author-bounded (a parsed query string), so
                // interning it before the listener gate grows the pool by at
                // most one entry per distinct query — and `.media` interns the
                // identical string, so it is a dedup in practice.
                let media_sid = ctx.vm.strings.intern(&media);
                // Slot order matches `event_shapes.rs::media_query_list_event`
                // + the constructor: `media`, then `matches`.
                let payload = vec![
                    PropertyValue::Data(JsValue::String(media_sid)),
                    PropertyValue::Data(JsValue::Boolean(now)),
                ];
                let init = EventInit {
                    bubbles: false,
                    cancelable: false,
                    composed: false,
                };
                // `fire_vm_event` allocates nothing if the MQL has no `change`
                // listener — a flip on an unobserved MQL just updates
                // `last_matches` (done above) and costs nothing.
                let _ = fire_vm_event(&mut ctx, id, change_sid, init, shape, proto, payload);
            }
        }

        // Each report-changes pass is its own microtask checkpoint (parity with
        // the other `deliver_*` members), even when nothing flipped — so a
        // pending microtask is never deferred past this turn.
        self.drain_microtasks();
    }
}

// ---------------------------------------------------------------------------
// window.matchMedia
// ---------------------------------------------------------------------------

/// `window.matchMedia(query)` — CSSOM-View §4
/// (`#dom-window-matchmedia`). Parses `query` (total parser, #360) and
/// returns a live `MediaQueryList`; `.matches` is evaluated live on read
/// (no stored snapshot). Marshalling only — parse / evaluate are
/// `elidex_css::media` calls.
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

    // Engine-independent parse (#360). `parse_media_query_list` is total
    // (malformed → `not all`; unknown feature → Kleene-unknown → false), so
    // there is no throw path. `.matches` is evaluated live by the getter.
    let parsed = parse_media_query_list(&query);
    let id = ctx.vm.create_media_query_list(parsed);
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

/// `mql.matches` (RO) — evaluated **live** against the current environment
/// (CSSOM-View §4.2), so it always reflects the current viewport/media facts
/// by construction (no stored snapshot to go stale; Codex R2). Absent entry
/// → `false` (defensive-by-construction; `AbortSignal.aborted` safe-default
/// for a cleared/collected side-table slot).
fn native_media_query_list_get_matches(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_media_query_list_this(ctx, this, "matches")?;
    let env = ctx.vm.media_environment();
    let matches = ctx
        .vm
        .media_query_list_registry
        .get(&id)
        .is_some_and(|e| evaluate(&e.parsed, &env));
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
