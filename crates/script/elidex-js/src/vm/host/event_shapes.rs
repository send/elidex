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
//! ## Shape + slot-writer unification
//!
//! Shape selection and payload-slot assembly used to live in two
//! separate 16-arm matches (`PrecomputedEventShapes::shape_for` and
//! `events::append_payload_slots`) that had to be kept in lockstep;
//! reordering one without the other silently wrote payload values
//! into the wrong JS-visible key slots.  [`dispatch_payload`]
//! consolidates both into a single match that picks the shape AND
//! writes the payload slots in one pass — adding a new variant
//! touches exactly one arm.

#![cfg(feature = "engine")]

use super::super::shape::{PropertyAttrs, ShapeId, ROOT_SHAPE};
use super::super::value::{JsValue, PropertyKey, PropertyValue, StringId};
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
    pub(crate) mouse: ShapeId,
    pub(crate) keyboard: ShapeId,
    pub(crate) transition: ShapeId,
    pub(crate) animation: ShapeId,
    pub(crate) input: ShapeId,
    pub(crate) clipboard: ShapeId,
    pub(crate) composition: ShapeId,
    pub(crate) focus: ShapeId,
    pub(crate) wheel: ShapeId,
    pub(crate) message: ShapeId,
    pub(crate) close_event: ShapeId,
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
    /// `new MouseEvent(type, init)` — UIEvent base + 13 mouse keys
    /// (clientX/Y, button, buttons, altKey/ctrlKey/metaKey/shiftKey,
    /// screenX/Y, movementX/Y, relatedTarget).  Distinct from the
    /// UA-dispatch `mouse` shape because constructed MouseEvents carry
    /// the full WebIDL mouse attribute surface (the UA dispatch
    /// variant trims to the 8 keys `DispatchEvent`'s payload populates).
    pub(crate) mouse_event_constructed: ShapeId,
    /// `new KeyboardEvent(type, init)` — UIEvent base + 9 keys
    /// (key, code, altKey/ctrlKey/metaKey/shiftKey, repeat, location,
    /// isComposing).  Separate from the UA-dispatch `keyboard` shape
    /// because the ctor exposes `location` / `isComposing`, which the
    /// 7-key UA payload omits.
    pub(crate) keyboard_event_constructed: ShapeId,
    /// `new FocusEvent(type, init)` — UIEvent base + `relatedTarget`.
    /// Separate from the UA-dispatch `focus` shape so the chain stays
    /// anchored at `ui_event_constructed`, preserving `view` / `detail`
    /// own-data on instances.
    pub(crate) focus_event_constructed: ShapeId,
    /// `new InputEvent(type, init)` — UIEvent base + `inputType` /
    /// `data` / `isComposing`.
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
    /// reporting path.
    pub(crate) error_event: ShapeId,
    /// `new PopStateEvent(type, init)` — core + `state`.
    pub(crate) pop_state_event: ShapeId,
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
            EventPayload::Mouse(_) => shapes.mouse,
            EventPayload::Keyboard(_) => shapes.keyboard,
            EventPayload::Transition(_) => shapes.transition,
            EventPayload::Animation(_) => shapes.animation,
            EventPayload::Input(_) => shapes.input,
            EventPayload::Clipboard(_) => shapes.clipboard,
            EventPayload::Composition(_) => shapes.composition,
            EventPayload::Focus(_) => shapes.focus,
            EventPayload::Wheel(_) => shapes.wheel,
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
            // clientX, clientY, button, buttons, altKey, ctrlKey, metaKey, shiftKey
            push_num(slots, m.client_x);
            push_num(slots, m.client_y);
            push_num(slots, f64::from(m.button));
            push_num(slots, f64::from(m.buttons));
            push_bool(slots, m.alt_key);
            push_bool(slots, m.ctrl_key);
            push_bool(slots, m.meta_key);
            push_bool(slots, m.shift_key);
        }
        EventPayload::Keyboard(k) => {
            // key, code, altKey, ctrlKey, metaKey, shiftKey, repeat
            let key_sid = vm.strings.intern(&k.key);
            let code_sid = vm.strings.intern(&k.code);
            push_str(slots, key_sid);
            push_str(slots, code_sid);
            push_bool(slots, k.alt_key);
            push_bool(slots, k.ctrl_key);
            push_bool(slots, k.meta_key);
            push_bool(slots, k.shift_key);
            push_bool(slots, k.repeat);
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
            // inputType, data, isComposing
            let type_sid = vm.strings.intern(&i.input_type);
            let data_val = match &i.data {
                Some(str_) => JsValue::String(vm.strings.intern(str_)),
                None => JsValue::Null,
            };
            push_str(slots, type_sid);
            push_val(slots, data_val);
            push_bool(slots, i.is_composing);
        }
        EventPayload::Clipboard(c) => {
            // dataType, data
            let type_sid = vm.strings.intern(&c.data_type);
            let data_sid = vm.strings.intern(&c.data);
            push_str(slots, type_sid);
            push_str(slots, data_sid);
        }
        EventPayload::Composition(c) => {
            // data
            let data_sid = vm.strings.intern(&c.data);
            push_str(slots, data_sid);
        }
        EventPayload::Focus(f) => {
            // relatedTarget
            // `Entity::to_bits().get()` is NonZeroU64, so a `0` bits
            // value is a payload construction bug.  Fall back to
            // `null` rather than panic so a malformed payload still
            // produces a sensible JS value.
            let related_val = match f.related_target.and_then(elidex_ecs::Entity::from_bits) {
                Some(entity) => JsValue::Object(vm.create_element_wrapper(entity)),
                None => JsValue::Null,
            };
            push_val(slots, related_val);
        }
        EventPayload::Wheel(w) => {
            // deltaX, deltaY, deltaMode
            push_num(slots, w.delta_x);
            push_num(slots, w.delta_y);
            push_num(slots, f64::from(w.delta_mode));
        }
        EventPayload::Message {
            data,
            origin,
            last_event_id,
        } => {
            // data, origin, lastEventId
            let data_sid = vm.strings.intern(data);
            let origin_sid = vm.strings.intern(origin);
            let last_id_sid = vm.strings.intern(last_event_id);
            push_str(slots, data_sid);
            push_str(slots, origin_sid);
            push_str(slots, last_id_sid);
            // `source` / `ports` populated when MessagePort lands (PR5b).
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
            // key, oldValue, newValue, url
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
            // `storageArea` populated when localStorage / sessionStorage land (PR5a).
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

        // Payload-specific keys per variant.  Order matches
        // `events::append_payload_slots` — if the payload-slot
        // appender is reordered, this table must be updated in
        // lockstep (or the slot values end up in the wrong
        // positions).
        let mouse = extend(
            self,
            core,
            &[
                self.well_known.client_x,
                self.well_known.client_y,
                self.well_known.button,
                self.well_known.buttons,
                self.well_known.alt_key,
                self.well_known.ctrl_key,
                self.well_known.meta_key,
                self.well_known.shift_key,
            ],
        );
        let keyboard = extend(
            self,
            core,
            &[
                self.well_known.key,
                self.well_known.code,
                self.well_known.alt_key,
                self.well_known.ctrl_key,
                self.well_known.meta_key,
                self.well_known.shift_key,
                self.well_known.repeat,
            ],
        );
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
        let input = extend(
            self,
            core,
            &[
                self.well_known.input_type,
                self.well_known.data,
                self.well_known.is_composing,
            ],
        );
        let clipboard = extend(
            self,
            core,
            &[self.well_known.data_type, self.well_known.data],
        );
        let composition = extend(self, core, &[self.well_known.data]);
        let focus = extend(self, core, &[self.well_known.related_target]);
        let wheel = extend(
            self,
            core,
            &[
                self.well_known.delta_x,
                self.well_known.delta_y,
                self.well_known.delta_mode,
            ],
        );
        let message = extend(
            self,
            core,
            &[
                self.well_known.data,
                self.well_known.origin,
                self.well_known.last_event_id,
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
        let input_event_constructed = extend(
            self,
            ui_event_constructed,
            &[
                self.well_known.data,
                self.well_known.is_composing,
                self.well_known.input_type,
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

        PrecomputedEventShapes {
            core,
            mouse,
            keyboard,
            transition,
            animation,
            input,
            clipboard,
            composition,
            focus,
            wheel,
            message,
            close_event,
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
