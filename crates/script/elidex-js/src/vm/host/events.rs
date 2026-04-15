//! Event object construction — the JS-side view of a `DispatchEvent`
//! that gets passed to every listener.
//!
//! Per design decision D4 (see `m4-12-pr3-plan.md`), the event object is
//! rebuilt **per listener invocation** — this mirrors boa's behaviour
//! and sidesteps `currentTarget` mutation between capture / target /
//! bubble phases.  The flag fields are threaded through
//! `ObjectKind::Event`'s internal slots; `DispatchFlags` is synced
//! **in** (at construction) and **out** (in PR3 C5 `call_listener`) so
//! accumulated state (e.g. a prior listener's `preventDefault`)
//! propagates correctly.
//!
//! ## Per-instance vs prototype
//!
//! Methods (`preventDefault` / `stopPropagation` /
//! `stopImmediatePropagation` / `composedPath`) and the
//! `defaultPrevented` accessor live on a single shared internal
//! prototype (`VmInner::event_methods_prototype`, populated once at
//! `register_globals` time).  Per-event allocation is therefore just
//! the data-property writes — no fresh `NativeFunction` objects per
//! dispatch, no fresh shape transitions for the method properties.
//!
//! This is NOT exposed as the spec `Event.prototype` global; the
//! constructor + visible `Event` global ship in PR5a alongside
//! `new Event(...)`.  At that point this intrinsic can become the
//! parent of (or be replaced by) the spec prototype.
//!
//! ## Properties installed on each event
//!
//! | Property | Source | Shape |
//! |----------|--------|-------|
//! | `type` | `event.event_type` | data, RO |
//! | `bubbles` | `event.bubbles` | data, RO |
//! | `cancelable` | `event.cancelable` | data, RO |
//! | `eventPhase` | `event.phase as u8` | data, RO |
//! | `target` | `target_wrapper_id` | data, RO |
//! | `currentTarget` | `current_target_id` | data, RO |
//! | `timeStamp` | `0.0` (TODO: PR4 `performance.now()`) | data, RO |
//! | `composed` | `event.composed` | data, RO |
//! | `isTrusted` | `event.is_trusted` | data, RO |
//! | `<payload-specific>` | `event.payload` | data, RO |
//!
//! ## Deferred to later PRs
//!
//! - `returnValue` legacy accessor → revisit when WPT
//!   `events/Event-*.html` runs.
//! - `initEvent` / `initXXXEvent` legacy initializers → rare, skipped.
//! - Timestamp population → `performance.now()` lands in PR4.

#![cfg(feature = "engine")]

use elidex_plugin::EventPayload;
use elidex_script_session::event_dispatch::DispatchEvent;

use super::super::natives_event::{
    native_event_composed_path, native_event_get_default_prevented, native_event_prevent_default,
    native_event_stop_immediate_propagation, native_event_stop_propagation,
};
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Populate `event_methods_prototype` with the four event methods
    /// + `defaultPrevented` accessor.
    ///
    /// Called once from `register_globals` after `Object.prototype`
    /// exists; the resulting object is the prototype every event
    /// instance inherits from.
    pub(in crate::vm) fn register_event_methods_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("preventDefault", native_event_prevent_default as NativeFn),
            ("stopPropagation", native_event_stop_propagation),
            (
                "stopImmediatePropagation",
                native_event_stop_immediate_propagation,
            ),
            ("composedPath", native_event_composed_path),
        ]);
        // `defaultPrevented` is an accessor (live getter), not a data
        // property — WHATWG DOM §2.9 requires it to reflect the current
        // canceled flag including writes from `preventDefault()` made
        // inside the same listener body.
        let getter =
            self.create_native_function("get defaultPrevented", native_event_get_default_prevented);
        let dp_key = PropertyKey::String(self.well_known.default_prevented);
        self.define_shaped_property(
            proto_id,
            dp_key,
            PropertyValue::Accessor {
                getter: Some(getter),
                setter: None,
            },
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.event_methods_prototype = Some(proto_id);
    }

    /// Build the JS event object for a single listener invocation.
    ///
    /// `target_wrapper_id` and `current_target_wrapper_id` are the
    /// pre-resolved `HostObject` wrappers for the event's target and
    /// currentTarget entities — built by the caller via
    /// `create_element_wrapper`.  Keeping wrapper resolution out of
    /// this function lets the caller share target wrappers across
    /// phases (target wrapper is constant across capture / at-target /
    /// bubble; only `currentTarget` changes per phase).
    ///
    /// `passive` threads through from the listener's registration; the
    /// `Event` variant stores it so `preventDefault()` can no-op
    /// without looking it up from `HostData`.
    ///
    /// Property installation goes through the precomputed-shape fast
    /// path — see `host/event_shapes.rs` module doc for the layout
    /// and [`VmInner::define_with_precomputed_shape`] for the
    /// single-operation slot publish.
    ///
    /// # GC safety
    ///
    /// The just-allocated event id is rooted internally via
    /// [`VmInner::push_temp_root`] across all subsequent allocations
    /// (Focus payloads' `relatedTarget` allocates a wrapper; the
    /// `composedPath` array allocation does too).  Without rooting,
    /// the event obj would be the only thing tying its
    /// prototype/payload to a root and would be reclaimed
    /// mid-construction.  The guard drops before return — so the
    /// returned `ObjectId` becomes vulnerable to collection from the
    /// next allocation by the caller.  Root it immediately (push to
    /// stack via [`VmInner::push_temp_root`], store in a frame slot,
    /// etc.) before any further VM operations that may allocate or
    /// run user JS.
    pub(crate) fn create_event_object(
        &mut self,
        event: &DispatchEvent,
        target_wrapper_id: ObjectId,
        current_target_wrapper_id: ObjectId,
        passive: bool,
    ) -> ObjectId {
        let event_id = self.alloc_object(Object {
            kind: ObjectKind::Event {
                default_prevented: event.flags.default_prevented,
                propagation_stopped: event.flags.propagation_stopped,
                immediate_propagation_stopped: event.flags.immediate_propagation_stopped,
                cancelable: event.cancelable,
                passive,
                composed_path: None,
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            // Methods + `defaultPrevented` accessor inherited from the
            // shared prototype — no per-event method install.
            prototype: self.event_methods_prototype,
            extensible: true,
        });

        // Root the just-allocated event_id across composed-path /
        // relatedTarget wrapper allocations below.
        let mut g = self.push_temp_root(JsValue::Object(event_id));

        // ---- composedPath internal slot ----
        // If the dispatch path populated `event.composed_path` (the
        // ECS-side propagation list), translate each Entity into its
        // HostObject wrapper and seed the Event's `composed_path`
        // slot with the resulting Array.  `composedPath()` returns
        // this Array directly (identity-preserving).  Empty
        // `composed_path` leaves the slot None — `composedPath()`'s
        // lazy-allocate path then provides an empty Array on first
        // call and caches it (per WHATWG DOM §2.9 identity rule).
        if !event.composed_path.is_empty() {
            let elements: Vec<JsValue> = event
                .composed_path
                .iter()
                .map(|&entity| JsValue::Object(g.create_element_wrapper(entity)))
                .collect();
            let arr_id = g.create_array_object(elements);
            if let ObjectKind::Event { composed_path, .. } = &mut g.get_object_mut(event_id).kind {
                *composed_path = Some(arr_id);
            }
        }

        // ---- Assemble slot Vec in shape order ----
        // Core 9 first, then payload — matching
        // `build_precomputed_event_shapes`'s transition chain.  Any
        // reordering here must be mirrored there or slot values land
        // under the wrong JS-visible keys.
        //
        // Built as `Vec<PropertyValue>` directly (not `Vec<JsValue>`
        // with a later `.map(Data).collect()`) so
        // `define_with_precomputed_shape` can *move* the vector into
        // the object's slot storage — saves one heap allocation per
        // dispatch.
        let type_sid = g.strings.intern(&event.event_type);
        // 9 core + up to 8 payload (Mouse is the largest).  All 16 payload
        // variants fit in this capacity with no reallocation.
        let mut slots: Vec<PropertyValue> = Vec::with_capacity(17);
        slots.push(PropertyValue::Data(JsValue::String(type_sid)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.bubbles)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.cancelable)));
        slots.push(PropertyValue::Data(JsValue::Number(f64::from(
            event.phase as u8,
        ))));
        slots.push(PropertyValue::Data(JsValue::Object(target_wrapper_id)));
        slots.push(PropertyValue::Data(JsValue::Object(
            current_target_wrapper_id,
        )));
        // `timeStamp` held at 0.0 until `performance.now()` lands in
        // PR4 (tracked in zesty-inventing-galaxy.md PR4 section).
        slots.push(PropertyValue::Data(JsValue::Number(0.0)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.composed)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.is_trusted)));

        // Payload-specific slot values.  May allocate (Focus's
        // relatedTarget via `create_element_wrapper`); the returned
        // wrapper ObjectId is immediately rooted in `HostData::wrapper_cache`
        // inside `create_element_wrapper` before we push it here.
        // The existing `slots` Vec holds only primitives and already-rooted
        // wrappers (target/currentTarget, composed-path wrappers, Focus
        // relatedTarget) — no JsValue in the Vec would be reclaimed if a
        // GC ran during the Focus allocation.
        append_payload_slots(&mut g, &mut slots, &event.payload);

        // ---- Publish shape + slots in one operation ----
        // `precomputed_event_shapes` is `Some` after register_globals;
        // `shape_for` picks the terminal ShapeId whose property
        // insertion order matches the slot Vec above.
        let shape_id = g
            .precomputed_event_shapes
            .as_ref()
            .expect("precomputed_event_shapes missing — register_globals must run before create_event_object")
            .shape_for(&event.payload);
        g.define_with_precomputed_shape(event_id, shape_id, slots);

        drop(g);
        event_id
    }
}

// ---------------------------------------------------------------------
// Payload slot assembly.  Each variant writes its values into the
// output Vec in the order the corresponding precomputed shape was
// built — see `host/event_shapes.rs::build_precomputed_event_shapes`.
// Reordering either side without the other will silently produce
// wrong-key writes (the debug_assert in define_with_precomputed_shape
// only verifies the *count*, not the order).
// ---------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn append_payload_slots(vm: &mut VmInner, slots: &mut Vec<PropertyValue>, payload: &EventPayload) {
    // Local helpers — keep each variant arm readable by wrapping the
    // repetitive `slots.push(PropertyValue::Data(JsValue::X(v)))` call.
    fn num(slots: &mut Vec<PropertyValue>, v: f64) {
        slots.push(PropertyValue::Data(JsValue::Number(v)));
    }
    fn b(slots: &mut Vec<PropertyValue>, v: bool) {
        slots.push(PropertyValue::Data(JsValue::Boolean(v)));
    }
    fn s(slots: &mut Vec<PropertyValue>, sid: super::super::value::StringId) {
        slots.push(PropertyValue::Data(JsValue::String(sid)));
    }
    fn v(slots: &mut Vec<PropertyValue>, v: JsValue) {
        slots.push(PropertyValue::Data(v));
    }

    match payload {
        EventPayload::Mouse(m) => {
            // clientX, clientY, button, buttons, altKey, ctrlKey, metaKey, shiftKey
            num(slots, m.client_x);
            num(slots, m.client_y);
            num(slots, f64::from(m.button));
            num(slots, f64::from(m.buttons));
            b(slots, m.alt_key);
            b(slots, m.ctrl_key);
            b(slots, m.meta_key);
            b(slots, m.shift_key);
        }
        EventPayload::Keyboard(k) => {
            // key, code, altKey, ctrlKey, metaKey, shiftKey, repeat
            let key_sid = vm.strings.intern(&k.key);
            let code_sid = vm.strings.intern(&k.code);
            s(slots, key_sid);
            s(slots, code_sid);
            b(slots, k.alt_key);
            b(slots, k.ctrl_key);
            b(slots, k.meta_key);
            b(slots, k.shift_key);
            b(slots, k.repeat);
        }
        EventPayload::Transition(t) => {
            // propertyName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&t.property_name);
            let pe_sid = vm.strings.intern(&t.pseudo_element);
            s(slots, name_sid);
            num(slots, t.elapsed_time);
            s(slots, pe_sid);
        }
        EventPayload::Animation(a) => {
            // animationName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&a.animation_name);
            let pe_sid = vm.strings.intern(&a.pseudo_element);
            s(slots, name_sid);
            num(slots, a.elapsed_time);
            s(slots, pe_sid);
        }
        EventPayload::Input(i) => {
            // inputType, data, isComposing
            let type_sid = vm.strings.intern(&i.input_type);
            let data_val = match &i.data {
                Some(str_) => JsValue::String(vm.strings.intern(str_)),
                None => JsValue::Null,
            };
            s(slots, type_sid);
            v(slots, data_val);
            b(slots, i.is_composing);
        }
        EventPayload::Clipboard(c) => {
            // dataType, data
            let type_sid = vm.strings.intern(&c.data_type);
            let data_sid = vm.strings.intern(&c.data);
            s(slots, type_sid);
            s(slots, data_sid);
        }
        EventPayload::Composition(c) => {
            // data
            let data_sid = vm.strings.intern(&c.data);
            s(slots, data_sid);
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
            v(slots, related_val);
        }
        EventPayload::Wheel(w) => {
            // deltaX, deltaY, deltaMode
            num(slots, w.delta_x);
            num(slots, w.delta_y);
            num(slots, f64::from(w.delta_mode));
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
            s(slots, data_sid);
            s(slots, origin_sid);
            s(slots, last_id_sid);
            // `source` / `ports` populated when MessagePort lands (PR5b).
        }
        EventPayload::CloseEvent(c) => {
            // code, reason, wasClean
            let reason_sid = vm.strings.intern(&c.reason);
            num(slots, f64::from(c.code));
            s(slots, reason_sid);
            b(slots, c.was_clean);
        }
        EventPayload::HashChange(h) => {
            // oldURL, newURL
            let old_sid = vm.strings.intern(&h.old_url);
            let new_sid = vm.strings.intern(&h.new_url);
            s(slots, old_sid);
            s(slots, new_sid);
        }
        EventPayload::PageTransition(p) => {
            // persisted
            b(slots, p.persisted);
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
            v(slots, key_val);
            v(slots, old_val);
            v(slots, new_val);
            s(slots, url_sid);
            // `storageArea` populated when localStorage / sessionStorage land (PR5a).
        }
        EventPayload::Scroll | EventPayload::None => {
            // No extra slots.  Terminal shape = `core`.
        }
        // `EventPayload` is `#[non_exhaustive]`.  A new upstream
        // variant landing without a matching arm here installs no
        // payload slots (matching the `core` terminal shape returned
        // by `PrecomputedEventShapes::shape_for`'s wildcard arm).
        // Debug-trips so test runs surface the omission; release
        // silently no-ops to avoid hard-failing dispatch on payloads
        // we just don't display yet.
        _ => debug_assert!(
            false,
            "unhandled EventPayload variant in append_payload_slots — \
             also add a matching entry in PrecomputedEventShapes::shape_for \
             and build_precomputed_event_shapes",
        ),
    }
}
