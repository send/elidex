//! Precomputed shape table for [`ObjectKind::Event`] objects.
//!
//! A single event dispatch builds one JS event object per listener
//! invocation (see `events.rs` module doc for the per-listener rebuild
//! rationale).  Without a precomputed shape, each build walks the
//! property transition table 9 + N times — one hashmap lookup per
//! core property, one per payload property — so a mouse event with 5
//! listeners costs ~85 shape-transition hashmap probes + 40
//! `strings.intern` calls just to produce the event objects.
//!
//! Since the final property layout of an event object is fixed by its
//! payload variant (the set of payload keys never changes at runtime),
//! we can build one terminal [`ShapeId`] per variant at VM init time
//! and then allocate event objects directly at that terminal shape.
//! `create_event_object` becomes a single `define_with_precomputed_shape`
//! call with the slot values arranged in shape order.
//!
//! The transition chain is shared — all variants extend the common
//! **core-9** shape (type, bubbles, cancelable, eventPhase, target,
//! currentTarget, timeStamp, composed, isTrusted), so
//! `shape_add_transition`'s built-in deduplication reuses the same
//! prefix ShapeIds across every variant.
//!
//! Variants with no payload keys (`EventPayload::None`, `Scroll`)
//! share the `core` terminal shape directly.  Any non-exhaustive
//! variant the VM doesn't yet recognise also falls through to `core`
//! — those payloads install no extra properties (see the wildcard
//! arm in [`dispatch_payload`]).
//!
//! ## Shape + slot-writer coordination
//!
//! Shape selection and payload-slot assembly used to live in two
//! separate 16-arm matches (`PrecomputedEventShapes::shape_for` and
//! `events::append_payload_slots`) in two different modules that had
//! to be kept in lockstep; reordering one without the other silently
//! wrote payload values into the wrong JS-visible key slots.
//! [`dispatch_payload`] co-locates both operations in a single
//! function — in practice still a `match` for shape selection
//! followed by per-variant slot pushes within the same function body,
//! but adding a new variant now touches one arm in one file instead
//! of two arms across two files, which is what the SSOT unification
//! was after.
//!
//! ## Boa parallel writer is intentionally NOT mirrored
//!
//! `elidex-js-boa::globals::events::set_payload_properties` carries a
//! parallel UA payload writer with the same family-by-family slot
//! layout.  Per master roadmap §E-2 Round 20 PR7, the boa runtime
//! crate is scheduled for full deletion (~-15,000 LoC).  Shape
//! changes in this file are NOT mirrored to boa deliberately — the
//! work would be throwaway.  When `/external-review` flags the
//! parallel for not being updated, resolve-with-cite this paragraph +
//! `memory/project_boa_runtime_deletion.md`.

#![cfg(feature = "engine")]

use super::super::shape::{PropertyAttrs, ShapeId, ROOT_SHAPE};
use super::super::value::{JsValue, ObjectId, PropertyKey, PropertyValue, StringId};
use super::super::VmInner;
use elidex_plugin::EventPayload;

/// Number of core properties every Event shape extends from:
/// `type`, `bubbles`, `cancelable`, `eventPhase`, `target`,
/// `currentTarget`, `timeStamp`, `composed`, `isTrusted`.  All variant
/// shapes are built by `extend(core, &[...payload_keys...])` so
/// `shape.property_count() - CORE_KEY_COUNT` yields the payload key
/// count.  Hardcoded invariant — verified by `core_9_slot_order_is_locked`
/// in `tests_event_constructor.rs`.
pub(crate) const CORE_KEY_COUNT: usize = 9;

/// Terminal `ShapeId`s for every `EventPayload` variant.
///
/// Built once during `register_globals` (after the payload-key
/// `WellKnownStrings` are interned) and consulted by
/// `create_event_object` on every dispatch via
/// [`PrecomputedEventShapes::shape_for`].
pub(crate) struct PrecomputedEventShapes {
    /// Terminal shape for core-9 properties only.  Used for payload
    /// variants that install no extra properties (`None`, `Scroll`)
    /// and as the parent of every other variant's terminal shape.
    pub(crate) core: ShapeId,
    // The six former UIEvent-family UA-dispatch shapes (`mouse` /
    // `keyboard` / `wheel` / `focus` / `input` / `composition`) were
    // dropped by `#11-event-modern-extras-shape-fold` (2026-05-13).
    // UA-dispatched events for those families now land at the
    // corresponding `*_event_constructed` shape further down so they
    // share a single shape with the script-side ctor instances.
    pub(crate) transition: ShapeId,
    pub(crate) animation: ShapeId,
    pub(crate) clipboard: ShapeId,
    pub(crate) message: ShapeId,
    pub(crate) close_event: ShapeId,
    /// Terminal shape for `new MediaQueryListEvent(type, init)` (CSSOM-View
    /// §4.2). Extends `core` with `media` + `matches` (both WEBIDL_RO). Also
    /// the shape the Slice 2b-ii host-fire builds for the `change` event.
    pub(crate) media_query_list_event: ShapeId,
    pub(crate) hash_change: ShapeId,
    pub(crate) page_transition: ShapeId,
    pub(crate) storage: ShapeId,
    /// Terminal shape for `new CustomEvent(type, {detail})` instances.
    /// Extends `core` with a single `detail` slot (JS-visible own
    /// property, WEBIDL_RO).  Not used by UA-initiated dispatch —
    /// `shape_for` falls through to `core` for
    /// `EventPayload::None` / non-CustomEvent variants.
    pub(crate) custom_event: ShapeId,
    // -- UIEvent family constructor shapes --
    //
    // Every UIEvent-family ctor allocates at a shape that extends
    // `ui_event_constructed` (core-9 + `view` + `detail`), so the
    // inherited UIEvent attributes live as own-data props at slot 9 /
    // 10 — no prototype-accessor + HashMap side-channel needed.  The
    // transition chain shares the `core + view + detail` prefix for
    // every descendant.
    /// Terminal shape for `new UIEvent(type, init)`.  Layout: core +
    /// `view` + `detail` (11 slots total).  Also the parent shape of
    /// every descendant's constructor shape below.
    pub(crate) ui_event_constructed: ShapeId,
    /// `new MouseEvent(type, init)` and UA-dispatched mouse events —
    /// UIEvent base + 13 mouse keys (screenX/Y, clientX/Y,
    /// ctrlKey/shiftKey/altKey/metaKey, button, buttons,
    /// relatedTarget, movementX/Y).  Shape unified for ctor + UA
    /// paths by `#11-event-modern-extras-shape-fold` so
    /// `getOwnPropertyNames` matches on both.  UA payloads omit
    /// screen / movement / relatedTarget — defaults of 0 / 0 / null
    /// land in those slots.
    pub(crate) mouse_event_constructed: ShapeId,
    /// `new KeyboardEvent(type, init)` and UA-dispatched key events —
    /// UIEvent base + 9 keys (key, code, location,
    /// ctrlKey/shiftKey/altKey/metaKey, repeat, isComposing).
    /// Shape unified across ctor + UA paths.  UA payloads omit
    /// location / isComposing — defaults of 0 / false fill those
    /// slots.
    pub(crate) keyboard_event_constructed: ShapeId,
    /// `new FocusEvent(type, init)` and UA-dispatched focus events —
    /// UIEvent base + `relatedTarget`.  Shape unified across ctor +
    /// UA paths.
    pub(crate) focus_event_constructed: ShapeId,
    /// `new InputEvent(type, init)` and UA-dispatched input events —
    /// UIEvent base + `data` / `isComposing` / `inputType` /
    /// `dataTransfer`.  Shape unified across ctor + UA paths.  UA
    /// payload omits `dataTransfer`; it defaults to null until a
    /// future host-side input flow populates real clipboard data.
    pub(crate) input_event_constructed: ShapeId,
    // -- Non-UIEvent specialized constructor shapes --
    //
    // These chain directly to `core` (no UIEvent prefix) since their
    // WebIDL interfaces extend `Event`, not `UIEvent`.  Slot layout
    // per init-dict key order; HashChangeEvent reuses the existing
    // `hash_change` terminal shape (both have `oldURL` / `newURL` in
    // identical order).
    /// `new PromiseRejectionEvent(type, init)` — core + `promise` +
    /// `reason`.  The UA dispatch path doesn't use a separate payload
    /// variant for these (Promise rejections flow through
    /// `VmInner::dispatch_unhandled_rejection` which constructs the
    /// Event object directly), so this shape is only reached via the
    /// script-side ctor.
    pub(crate) promise_rejection_event: ShapeId,
    /// `new ErrorEvent(type, init)` — core + `message` + `filename`
    /// + `lineno` + `colno` + `error`.  Separate from any UA error
    ///   reporting path.
    pub(crate) error_event: ShapeId,
    /// `new PopStateEvent(type, init)` — core + `state`.
    pub(crate) pop_state_event: ShapeId,
    // -- D-10 events-misc constructor shapes --
    //
    // All chain to `core` directly except `composition_event_constructed`
    // (chains through `ui_event_constructed`) and `wheel_event_constructed`
    // (chains through `mouse_event_constructed`).  MessageEvent +
    // PageTransitionEvent reuse the existing `message` / `page_transition`
    // UA-dispatch shapes (init-dict order matches UA payload).
    /// `new SubmitEvent(type, init)` — core + `submitter`.
    pub(crate) submit_event: ShapeId,
    /// `new FormDataEvent(type, init)` — core + `formData`.
    pub(crate) formdata_event: ShapeId,
    /// `new ToggleEvent(type, init)` — core + `newState` + `oldState`.
    /// Slot order matches `dispatch_toggle_event` (newState before
    /// oldState — DevTools enumeration / spec attr-order).
    pub(crate) toggle_event: ShapeId,
    /// `new CompositionEvent(type, init)` and UA-dispatched
    /// composition events — UIEvent base + `data`.  Shape unified
    /// across ctor + UA paths.
    pub(crate) composition_event_constructed: ShapeId,
    /// `new ClipboardEvent(type, init)` — core + `clipboardData`.
    pub(crate) clipboard_event_constructed: ShapeId,
    /// `new ProgressEvent(type, init)` — core + `lengthComputable` +
    /// `loaded` + `total`.
    pub(crate) progress_event: ShapeId,
    /// `new BeforeUnloadEvent(...)` — core only (no constructable
    /// surface). `returnValue` is a mutable prototype accessor backed
    /// by an internal slot rather than an own-data shape slot, so the
    /// shape stays at the core-9 layout.  Reserved for future
    /// UA-dispatch path; currently unread because `BeforeUnloadEvent`
    /// always throws on construct + no UA-dispatch path fires
    /// `beforeunload` yet (deferred to `#11-event-dispatch-extra`).
    /// Kept on the struct so the precomputed-shape registry is
    /// 1-1 with the prototype set.
    #[allow(dead_code)]
    pub(crate) before_unload_event: ShapeId,
    /// `new WheelEvent(type, init)` and UA-dispatched wheel events —
    /// MouseEvent base + `deltaX` / `deltaY` / `deltaZ` / `deltaMode`.
    /// Shape unified across ctor + UA paths.  The engine-indep
    /// `WheelEventInit` struct only carries `delta_x` / `delta_y` /
    /// `delta_mode`; UA dispatch fills the 13 inherited mouse slots
    /// (screenX/Y, clientX/Y, modifier flags, button/buttons,
    /// relatedTarget, movementX/Y) and `deltaZ` with WebIDL default
    /// values (0 / false / null).
    pub(crate) wheel_event_constructed: ShapeId,
    // -- D-9 events-modern-input constructor shapes --
    /// `new PointerEvent(type, init)` — MouseEvent base + 12
    /// pointer-specific keys (pointerId / width / height / pressure /
    /// tangentialPressure / tiltX / tiltY / twist / altitudeAngle /
    /// azimuthAngle / pointerType / isPrimary).  Chains through
    /// `mouse_event_constructed`.
    pub(crate) pointer_event_constructed: ShapeId,
    /// `new DragEvent(type, init)` — MouseEvent base + `dataTransfer`.
    /// Chains through `mouse_event_constructed`.
    pub(crate) drag_event_constructed: ShapeId,
    /// `new TouchEvent(type, init)` — UIEvent base + 3 TouchList
    /// references (touches / targetTouches / changedTouches) + 4
    /// modifier flags (ctrlKey / shiftKey / altKey / metaKey).
    /// Chains through `ui_event_constructed`.
    pub(crate) touch_event_constructed: ShapeId,
    /// `new Touch(init)` — Touch is NOT an Event, so this shape
    /// chains directly to ROOT (Touch.prototype → Object.prototype).
    /// All 12 IDL members of Touch live on the side table
    /// (VmInner::touch_states) — the shape stays empty, accessors
    /// route through the prototype.  Kept here for symmetry with
    /// the other ctor shapes; D-9 instances allocate with
    /// `ROOT_SHAPE` directly.
    #[allow(dead_code)]
    pub(crate) touch_constructed: ShapeId,
}

// Local helpers for [`dispatch_payload`] — keep each variant arm
// readable by wrapping the repetitive
// `slots.push(PropertyValue::Data(JsValue::X(v)))` call.  Inlined
// at `#[inline]` by the optimiser; measured neutral vs. direct
// pushes at -O3.
fn push_num(slots: &mut Vec<PropertyValue>, v: f64) {
    slots.push(PropertyValue::Data(JsValue::Number(v)));
}
fn push_bool(slots: &mut Vec<PropertyValue>, v: bool) {
    slots.push(PropertyValue::Data(JsValue::Boolean(v)));
}
fn push_str(slots: &mut Vec<PropertyValue>, sid: StringId) {
    slots.push(PropertyValue::Data(JsValue::String(sid)));
}
fn push_val(slots: &mut Vec<PropertyValue>, v: JsValue) {
    slots.push(PropertyValue::Data(v));
}

/// Push the UIEvent-prefix slots (`view`, `detail`) for every UA-side
/// payload writer whose terminal shape extends `ui_event_constructed`.
///
/// Structural mirror of [`super::events_ui::VmInner::build_ui_event_instance`]'s
/// `view` / `detail` prepend (slots 9 / 10 of the
/// `ui_event_constructed` shape transition chain).  Both helpers MUST
/// emit slots at identical positions — drift between them re-creates
/// the D-10 R9 UA-shape gap (UA-fired `instanceof MouseEvent` returns
/// true but `event.view` returns `undefined`).  Six call sites consume
/// this helper today (`dispatch_payload` Mouse / Keyboard / Wheel /
/// Focus / Input / Composition arms); the slot-order TDD locks in
/// `tests_event_shape_fold.rs` are the mechanical safety net against
/// reorderings that bypass this helper.
///
/// **Value asymmetry vs ctor (deliberate Chrome-parity divergence)**:
/// the ctor path's `view` defaults to `JsValue::Null` per the
/// WebIDL declaration `attribute Window? view = null` on
/// `UIEventInit` (UI Events L3 attribute table).  The UA-fire path
/// defaults to `JsValue::Object(global_object)` — a UA convention
/// (NOT spec-mandated): the "fire an event" / "fire a trusted
/// event" algorithms are silent on `view`, but Chrome and Firefox
/// both surface `event.view === window` for UA-dispatched UI
/// events by populating it from the target node's owner document's
/// `defaultView`.  Symmetry tests in `tests_event_shape_fold.rs`
/// assert NAME equality on `getOwnPropertyNames(uaEvent)` vs
/// `getOwnPropertyNames(ctorEvent)`, not value equality — checking
/// values would falsely flag this deliberate asymmetry.
///
/// `detail` is symmetric: both paths default to `0` per WebIDL §3.2
/// `attribute long detail = 0`.
///
/// Iframe milestone note: today there is exactly one realm per VM, so
/// `vm.global_object` and `event.target.ownerDocument.defaultView` are
/// the same `ObjectId`.  Once iframes ship, `view` should resolve to
/// the target's owner document's `defaultView` rather than the current
/// realm's `globalThis` — at that point this helper grows a target
/// argument and the value derivation moves into the caller.
fn push_ui_prefix(slots: &mut Vec<PropertyValue>, vm_global: ObjectId) {
    push_val(slots, JsValue::Object(vm_global));
    push_num(slots, 0.0);
}

/// Single source of truth for `EventPayload` ↔
/// `(ShapeId, payload-slot sequence)`.  Picks the terminal shape
/// and appends the variant-specific slot values to `slots` in a
/// single match — adding a new variant touches only this function,
/// [`VmInner::build_precomputed_event_shapes`], and the
/// [`PrecomputedEventShapes`] struct.
///
/// `slots` must already contain the core-9 values in canonical
/// order before this call; `dispatch_payload` appends exactly
/// `<terminal_shape>.property_count() - CORE_KEY_COUNT` entries.
/// Debug builds verify that delta via [`payload_key_count`].
///
/// `vm` is needed because some variants intern payload strings
/// (`Keyboard.key`, `Message.origin`, etc.) or allocate element
/// wrappers (`Focus.relatedTarget`).
#[allow(clippy::too_many_lines)]
pub(super) fn dispatch_payload(
    vm: &mut VmInner,
    slots: &mut Vec<PropertyValue>,
    payload: &EventPayload,
) -> ShapeId {
    // Pull the shape_id first as a `Copy` value so the rest of the
    // function can borrow `vm` mutably for interning / wrapper
    // allocation without conflicting with the shapes borrow.
    let shape_id: ShapeId = {
        let shapes = vm
            .precomputed_event_shapes
            .as_ref()
            .expect("precomputed_event_shapes must be built before dispatch_payload");
        match payload {
            // Six families share their UA shape with the corresponding
            // ctor shape (`#11-event-modern-extras-shape-fold`,
            // 2026-05-13): UA-fired Mouse / Keyboard / Wheel / Focus /
            // Input / Composition events now carry the same own-data
            // surface as their `new XEvent(type, init)` counterparts.
            // `getOwnPropertyNames(uaEvent)` matches
            // `getOwnPropertyNames(ctorEvent)` per family — values may
            // differ (UA `view = window`, ctor `view = null`).
            EventPayload::Mouse(_) => shapes.mouse_event_constructed,
            EventPayload::Keyboard(_) => shapes.keyboard_event_constructed,
            EventPayload::Transition(_) => shapes.transition,
            EventPayload::Animation(_) => shapes.animation,
            EventPayload::Input(_) => shapes.input_event_constructed,
            EventPayload::Clipboard(_) => shapes.clipboard,
            EventPayload::Composition(_) => shapes.composition_event_constructed,
            EventPayload::Focus(_) => shapes.focus_event_constructed,
            EventPayload::Wheel(_) => shapes.wheel_event_constructed,
            EventPayload::Message { .. } => shapes.message,
            EventPayload::CloseEvent(_) => shapes.close_event,
            EventPayload::HashChange(_) => shapes.hash_change,
            EventPayload::PageTransition(_) => shapes.page_transition,
            EventPayload::Storage { .. } => shapes.storage,
            // `Scroll` / `None` / unknown non-exhaustive variants
            // install no payload properties → the core-9 shape.
            _ => shapes.core,
        }
    };

    let len_before = slots.len();

    match payload {
        EventPayload::Mouse(m) => {
            // Order matches `mouse_event_constructed` (UI prefix +
            // 13 mouse slots: screenX, screenY, clientX, clientY,
            // ctrlKey, shiftKey, altKey, metaKey, button, buttons,
            // relatedTarget, movementX, movementY).  Within the
            // 13-slot block the modifier order swaps from the prior
            // UA layout (alt/ctrl/meta/shift → ctrl/shift/alt/meta)
            // and button/buttons move from the leading mouse slots
            // to positions 8/9.  TDD slot-order locks in
            // `tests_event_shape_fold.rs` are the safety net.
            //
            // UA payloads omit screenX/Y / movementX/Y / relatedTarget;
            // they default to spec values (0 / 0 / null) — host shells
            // can populate real values via this same writer once they
            // start carrying them in `MouseEventInit`.
            push_ui_prefix(slots, vm.global_object);
            push_num(slots, 0.0); // screenX
            push_num(slots, 0.0); // screenY
            push_num(slots, m.client_x);
            push_num(slots, m.client_y);
            push_bool(slots, m.ctrl_key);
            push_bool(slots, m.shift_key);
            push_bool(slots, m.alt_key);
            push_bool(slots, m.meta_key);
            push_num(slots, f64::from(m.button));
            push_num(slots, f64::from(m.buttons));
            push_val(slots, JsValue::Null); // relatedTarget
            push_num(slots, 0.0); // movementX
            push_num(slots, 0.0); // movementY
        }
        EventPayload::Keyboard(k) => {
            // Order matches `keyboard_event_constructed` (UI prefix +
            // 9 keyboard slots: key, code, location, ctrlKey,
            // shiftKey, altKey, metaKey, repeat, isComposing).  UA
            // payload omits location / isComposing — defaults to 0 /
            // false per WebIDL §3.4.
            let key_sid = vm.strings.intern(&k.key);
            let code_sid = vm.strings.intern(&k.code);
            push_ui_prefix(slots, vm.global_object);
            push_str(slots, key_sid);
            push_str(slots, code_sid);
            push_num(slots, 0.0); // location
            push_bool(slots, k.ctrl_key);
            push_bool(slots, k.shift_key);
            push_bool(slots, k.alt_key);
            push_bool(slots, k.meta_key);
            push_bool(slots, k.repeat);
            push_bool(slots, false); // isComposing
        }
        EventPayload::Transition(t) => {
            // propertyName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&t.property_name);
            let pe_sid = vm.strings.intern(&t.pseudo_element);
            push_str(slots, name_sid);
            push_num(slots, t.elapsed_time);
            push_str(slots, pe_sid);
        }
        EventPayload::Animation(a) => {
            // animationName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&a.animation_name);
            let pe_sid = vm.strings.intern(&a.pseudo_element);
            push_str(slots, name_sid);
            push_num(slots, a.elapsed_time);
            push_str(slots, pe_sid);
        }
        EventPayload::Input(i) => {
            // Order matches `input_event_constructed` (UI prefix +
            // data, isComposing, inputType, dataTransfer).  Note this
            // is NOT the historical UA order (inputType, data,
            // isComposing) — the ctor shape lists data first per the
            // WebIDL declaration in Input Events L2 §5.  UA payload
            // omits `dataTransfer`; defaults to null until a future
            // host-side input flow populates real clipboard /
            // composition DataTransfer payloads.
            let type_sid = vm.strings.intern(&i.input_type);
            let data_val = match &i.data {
                Some(str_) => JsValue::String(vm.strings.intern(str_)),
                None => JsValue::Null,
            };
            push_ui_prefix(slots, vm.global_object);
            push_val(slots, data_val);
            push_bool(slots, i.is_composing);
            push_str(slots, type_sid);
            push_val(slots, JsValue::Null); // dataTransfer
        }
        EventPayload::Clipboard(c) => {
            // dataType, data
            let type_sid = vm.strings.intern(&c.data_type);
            let data_sid = vm.strings.intern(&c.data);
            push_str(slots, type_sid);
            push_str(slots, data_sid);
        }
        EventPayload::Composition(c) => {
            // Order matches `composition_event_constructed`
            // (UI prefix + `data`).
            let data_sid = vm.strings.intern(&c.data);
            push_ui_prefix(slots, vm.global_object);
            push_str(slots, data_sid);
        }
        EventPayload::Focus(f) => {
            // Order matches `focus_event_constructed`
            // (UI prefix + `relatedTarget`).
            // `Entity::to_bits().get()` is NonZeroU64, so a `0` bits
            // value is a payload construction bug.  Fall back to
            // `null` rather than panic so a malformed payload still
            // produces a sensible JS value.
            let related_val = match f.related_target.and_then(elidex_ecs::Entity::from_bits) {
                Some(entity) => JsValue::Object(vm.create_element_wrapper(entity)),
                None => JsValue::Null,
            };
            push_ui_prefix(slots, vm.global_object);
            push_val(slots, related_val);
        }
        EventPayload::Wheel(w) => {
            // Order matches `wheel_event_constructed` (extends
            // `mouse_event_constructed`): UI prefix + 13 mouse slots
            // (all defaults — UA `WheelEventInit` carries none of the
            // mouse keys) + 4 wheel slots (deltaX, deltaY, deltaZ,
            // deltaMode).  UA payload omits deltaZ; defaults to 0.0
            // per WebIDL §5.5.
            push_ui_prefix(slots, vm.global_object);
            push_num(slots, 0.0); // screenX
            push_num(slots, 0.0); // screenY
            push_num(slots, 0.0); // clientX
            push_num(slots, 0.0); // clientY
            push_bool(slots, false); // ctrlKey
            push_bool(slots, false); // shiftKey
            push_bool(slots, false); // altKey
            push_bool(slots, false); // metaKey
            push_num(slots, 0.0); // button
            push_num(slots, 0.0); // buttons
            push_val(slots, JsValue::Null); // relatedTarget
            push_num(slots, 0.0); // movementX
            push_num(slots, 0.0); // movementY
            push_num(slots, w.delta_x);
            push_num(slots, w.delta_y);
            push_num(slots, 0.0); // deltaZ
            push_num(slots, f64::from(w.delta_mode));
        }
        EventPayload::Message {
            data,
            origin,
            last_event_id,
        } => {
            // data, origin, lastEventId, source, ports — full
            // payload-slot extension matching the `message` shape
            // declared by `build_precomputed_event_shapes`.  The
            // shell-side payload struct doesn't carry `source` or
            // `ports` yet (MessagePort lands with the M4-12
            // cutover-residual transferable-objects work), so they
            // surface as `null` / fresh empty Array — the JS-visible
            // `MessageEvent` shape stays in lockstep with the
            // `dispatch_post_message` path that does carry them.
            let data_sid = vm.strings.intern(data);
            let origin_sid = vm.strings.intern(origin);
            let last_id_sid = vm.strings.intern(last_event_id);
            push_str(slots, data_sid);
            push_str(slots, origin_sid);
            push_str(slots, last_id_sid);
            push_val(slots, JsValue::Null);
            let ports_arr = vm.create_array_object(Vec::new());
            push_val(slots, JsValue::Object(ports_arr));
        }
        EventPayload::CloseEvent(c) => {
            // code, reason, wasClean
            let reason_sid = vm.strings.intern(&c.reason);
            push_num(slots, f64::from(c.code));
            push_str(slots, reason_sid);
            push_bool(slots, c.was_clean);
        }
        EventPayload::HashChange(h) => {
            // oldURL, newURL
            let old_sid = vm.strings.intern(&h.old_url);
            let new_sid = vm.strings.intern(&h.new_url);
            push_str(slots, old_sid);
            push_str(slots, new_sid);
        }
        EventPayload::PageTransition(p) => {
            // persisted
            push_bool(slots, p.persisted);
        }
        EventPayload::Storage {
            key,
            old_value,
            new_value,
            url,
        } => {
            // key, oldValue, newValue, url, storageArea
            let opt = |vm: &mut VmInner, str_: &Option<String>| match str_ {
                Some(x) => JsValue::String(vm.strings.intern(x)),
                None => JsValue::Null,
            };
            let key_val = opt(vm, key);
            let old_val = opt(vm, old_value);
            let new_val = opt(vm, new_value);
            let url_sid = vm.strings.intern(url);
            push_val(slots, key_val);
            push_val(slots, old_val);
            push_val(slots, new_val);
            push_str(slots, url_sid);
            // `storageArea` is `null` on UA-dispatched events:
            // cross-VM dispatch is shell-driven and no
            // backing Storage object is associated with the
            // dispatched event in the receiving VM.  Same as
            // `set_storage_payload` in the boa precedent.
            push_val(slots, JsValue::Null);
        }
        EventPayload::Scroll | EventPayload::None => {
            // No extra slots.  Terminal shape = `core`.
        }
        // `EventPayload` is `#[non_exhaustive]`.  A new upstream
        // variant landing without a matching arm here installs no
        // payload slots — matches the `core` terminal shape returned
        // by the shape-selection match above.  Debug-trips so test
        // runs surface the omission; release silently no-ops to
        // avoid hard-failing dispatch on payloads we just don't
        // display yet.
        _ => debug_assert!(
            false,
            "unhandled EventPayload variant in dispatch_payload — \
             add a matching arm to BOTH the shape selection above \
             and the payload-slot writer below, plus an entry in \
             build_precomputed_event_shapes",
        ),
    }

    // Variant-count invariant: every payload writer pushes exactly
    // the number of slots its terminal shape expects.  Catches
    // "writer forgot a push" or "shape added a key without writer"
    // drift in debug runs; release builds still pass through to
    // `define_with_precomputed_shape`'s own count assertion.
    debug_assert_eq!(
        slots.len() - len_before,
        payload_key_count(vm, shape_id),
        "dispatch_payload: writer and shape disagree on payload key count"
    );

    shape_id
}

/// Number of payload-specific keys in `shape_id` — that is, total
/// properties minus the core-9.  Panics if `shape_id` is bogus
/// (out-of-bounds into `vm.shapes`); callers always pass a shape
/// returned by `build_precomputed_event_shapes`, so this shouldn't
/// fire outside test code.
pub(crate) fn payload_key_count(vm: &VmInner, shape_id: ShapeId) -> usize {
    vm.shapes[shape_id as usize]
        .ordered_entries
        .len()
        .saturating_sub(CORE_KEY_COUNT)
}

impl VmInner {
    /// Walk shape-add transitions for the core 9 event properties
    /// followed by each `EventPayload` variant's payload keys, caching
    /// the terminal `ShapeId` per variant.
    ///
    /// Called exactly once from `register_globals` after the payload
    /// `WellKnownStrings` are interned.  Every `shape_add_transition`
    /// call permanently adds a Shape to `VmInner.shapes` but the cost
    /// is paid once at VM creation (~30 shapes × negligible per-shape
    /// memory) in exchange for eliminating ~17 transition lookups and
    /// ~8 intern calls **per dispatched event** at runtime.
    #[allow(clippy::too_many_lines)]
    pub(in crate::vm) fn build_precomputed_event_shapes(&mut self) -> PrecomputedEventShapes {
        // Core-9 properties installed on every event object.  Order
        // matches `events::create_event_object` → the slot Vec handed
        // to `define_with_precomputed_shape` at runtime must follow
        // the same sequence.
        let core_keys = [
            self.well_known.event_type,
            self.well_known.bubbles,
            self.well_known.cancelable,
            self.well_known.event_phase,
            self.well_known.target,
            self.well_known.current_target,
            self.well_known.time_stamp,
            self.well_known.composed,
            self.well_known.is_trusted,
        ];
        let core = extend(self, ROOT_SHAPE, &core_keys);

        // Payload-specific keys per variant.  Order matches the
        // sibling `dispatch_payload` per-variant slot pushes — if
        // the writer is reordered, this table must be updated in
        // lockstep (or the slot values end up in the wrong
        // positions).
        //
        // The six former UA-only Mouse / Keyboard / Wheel / Focus /
        // Input / Composition shapes were removed by
        // `#11-event-modern-extras-shape-fold`.  Their UA-dispatch
        // arms in `dispatch_payload` now share the corresponding
        // `*_event_constructed` shape declared further below, so
        // `getOwnPropertyNames(uaEvent)` matches
        // `getOwnPropertyNames(ctorEvent)` per family.
        let transition = extend(
            self,
            core,
            &[
                self.well_known.property_name,
                self.well_known.elapsed_time,
                self.well_known.pseudo_element,
            ],
        );
        let animation = extend(
            self,
            core,
            &[
                self.well_known.animation_name,
                self.well_known.elapsed_time,
                self.well_known.pseudo_element,
            ],
        );
        let clipboard = extend(
            self,
            core,
            &[self.well_known.data_type, self.well_known.data],
        );
        let message = extend(
            self,
            core,
            &[
                self.well_known.data,
                self.well_known.origin,
                self.well_known.last_event_id,
                self.well_known.source,
                self.well_known.ports,
            ],
        );
        // CloseEvent's numeric `code` shares the JS-visible name with
        // Keyboard's `code` → same StringId (StringPool canonicalises);
        // the shared `well_known.code` field is used for both.
        let close_event = extend(
            self,
            core,
            &[
                self.well_known.code,
                self.well_known.reason,
                self.well_known.was_clean,
            ],
        );
        // MediaQueryListEvent (CSSOM-View §4.2): `media` + `matches`, in
        // IDL dictionary order (the ctor + 2b-ii host-fire write slots in
        // this order).
        let media_query_list_event = extend(
            self,
            core,
            &[self.well_known.media, self.well_known.matches],
        );
        let hash_change = extend(
            self,
            core,
            &[self.well_known.old_url, self.well_known.new_url],
        );
        let page_transition = extend(self, core, &[self.well_known.persisted]);
        let storage = extend(
            self,
            core,
            &[
                self.well_known.key,
                self.well_known.old_value,
                self.well_known.new_value,
                self.well_known.url,
                self.well_known.storage_area,
            ],
        );
        // CustomEvent.prototype: core + `detail`.
        let custom_event = extend(self, core, &[self.well_known.detail]);

        // UIEvent family constructor shapes.  Every descendant
        // chains through `ui_event_constructed` so its 11
        // leading slots — core-9 + `view` + `detail` — are identical
        // across the tree.  `shape_add_transition` deduplication means
        // MouseEvent's transition chain reuses the UIEvent prefix
        // without allocating duplicate intermediate shapes.
        let ui_event_constructed =
            extend(self, core, &[self.well_known.view, self.well_known.detail]);
        let mouse_event_constructed = extend(
            self,
            ui_event_constructed,
            &[
                self.well_known.screen_x,
                self.well_known.screen_y,
                self.well_known.client_x,
                self.well_known.client_y,
                self.well_known.ctrl_key,
                self.well_known.shift_key,
                self.well_known.alt_key,
                self.well_known.meta_key,
                self.well_known.button,
                self.well_known.buttons,
                self.well_known.related_target,
                self.well_known.movement_x,
                self.well_known.movement_y,
            ],
        );
        let keyboard_event_constructed = extend(
            self,
            ui_event_constructed,
            &[
                self.well_known.key,
                self.well_known.code,
                self.well_known.location,
                self.well_known.ctrl_key,
                self.well_known.shift_key,
                self.well_known.alt_key,
                self.well_known.meta_key,
                self.well_known.repeat,
                self.well_known.is_composing,
            ],
        );
        let focus_event_constructed = extend(
            self,
            ui_event_constructed,
            &[self.well_known.related_target],
        );
        // `input_event_constructed`: data, isComposing, inputType,
        // dataTransfer — also reused by the UA-dispatch Input arm
        // since the shape-fold (`#11-event-modern-extras-shape-fold`).
        let input_event_constructed = extend(
            self,
            ui_event_constructed,
            &[
                self.well_known.data,
                self.well_known.is_composing,
                self.well_known.input_type,
                self.well_known.data_transfer,
            ],
        );

        // Non-UIEvent specialized constructor shapes.  Chain to
        // `core` directly — these don't inherit `view` / `detail`
        // since their WebIDL interfaces extend Event, not UIEvent.
        let promise_rejection_event = extend(
            self,
            core,
            &[self.well_known.promise, self.well_known.reason],
        );
        let error_event = extend(
            self,
            core,
            &[
                self.well_known.message,
                self.well_known.filename,
                self.well_known.lineno,
                self.well_known.colno,
                self.well_known.error,
            ],
        );
        let pop_state_event = extend(self, core, &[self.well_known.state]);

        // D-10 events-misc constructor shapes.
        let submit_event = extend(self, core, &[self.well_known.submitter]);
        let formdata_event = extend(self, core, &[self.well_known.form_data]);
        // ToggleEvent slot order: newState before oldState (matches
        // dispatch_toggle_event hand-rolled allocation + Chrome
        // DevTools enumeration order).
        let toggle_event = extend(
            self,
            core,
            &[self.well_known.new_state, self.well_known.old_state],
        );
        let composition_event_constructed =
            extend(self, ui_event_constructed, &[self.well_known.data]);
        let clipboard_event_constructed = extend(self, core, &[self.well_known.clipboard_data]);
        let progress_event = extend(
            self,
            core,
            &[
                self.well_known.length_computable,
                self.well_known.loaded,
                self.well_known.total,
            ],
        );
        // BeforeUnloadEvent shape stays at core-9 — `returnValue` is a
        // mutable prototype accessor backed by an internal slot, not an
        // own-data shape slot.
        let before_unload_event = core;
        // WheelEvent shape extends mouse_event_constructed with the 4
        // wheel-specific slots.  Slot order: deltaX, deltaY, deltaZ,
        // deltaMode (matches WebIDL declaration order in UI Events §5.5).
        let wheel_event_constructed = extend(
            self,
            mouse_event_constructed,
            &[
                self.well_known.delta_x,
                self.well_known.delta_y,
                self.well_known.delta_z,
                self.well_known.delta_mode,
            ],
        );

        // D-9 events-modern-input constructor shapes.
        // PointerEvent extends MouseEvent with 12 pointer-specific
        // slots.  Order matches `pointer.rs::parse_pointer_event_members`.
        let pointer_event_constructed = extend(
            self,
            mouse_event_constructed,
            &[
                self.well_known.pointer_id,
                self.well_known.width,
                self.well_known.height,
                self.well_known.pressure,
                self.well_known.tangential_pressure,
                self.well_known.tilt_x,
                self.well_known.tilt_y,
                self.well_known.twist,
                self.well_known.altitude_angle,
                self.well_known.azimuth_angle,
                self.well_known.pointer_type,
                self.well_known.is_primary,
            ],
        );
        // DragEvent extends MouseEvent with `dataTransfer`.
        let drag_event_constructed = extend(
            self,
            mouse_event_constructed,
            &[self.well_known.data_transfer],
        );
        // TouchEvent extends UIEvent with 3 TouchLists + 4 modifier
        // flags.  Order matches `touch.rs::native_touch_event_constructor`.
        let touch_event_constructed = extend(
            self,
            ui_event_constructed,
            &[
                self.well_known.touches,
                self.well_known.target_touches,
                self.well_known.changed_touches,
                self.well_known.ctrl_key,
                self.well_known.shift_key,
                self.well_known.alt_key,
                self.well_known.meta_key,
            ],
        );
        // Touch.constructed shape — empty (all state on side table).
        let touch_constructed = ROOT_SHAPE;

        PrecomputedEventShapes {
            core,
            transition,
            animation,
            clipboard,
            message,
            close_event,
            media_query_list_event,
            hash_change,
            page_transition,
            storage,
            custom_event,
            ui_event_constructed,
            mouse_event_constructed,
            keyboard_event_constructed,
            focus_event_constructed,
            input_event_constructed,
            promise_rejection_event,
            error_event,
            pop_state_event,
            submit_event,
            formdata_event,
            toggle_event,
            composition_event_constructed,
            clipboard_event_constructed,
            progress_event,
            before_unload_event,
            wheel_event_constructed,
            pointer_event_constructed,
            drag_event_constructed,
            touch_event_constructed,
            touch_constructed,
        }
    }
}

/// Walk `shape_add_transition` for each key under WEBIDL_RO attrs,
/// returning the terminal ShapeId.  Free function (not a closure)
/// because the borrow checker rejects reusing an `&mut self`-capturing
/// closure across sibling calls.
fn extend(vm: &mut VmInner, parent: ShapeId, keys: &[StringId]) -> ShapeId {
    let attrs = PropertyAttrs::WEBIDL_RO;
    let mut s = parent;
    for &k in keys {
        s = vm.shape_add_transition(s, PropertyKey::String(k), attrs);
    }
    s
}
