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
    /// # Property installation (PR3.6 fast path)
    ///
    /// Every own property of the event object is a data property
    /// whose name and attributes are fixed by the payload variant.
    /// We exploit this by:
    ///
    /// 1. Allocating at `ROOT_SHAPE` with an empty slot vec.
    /// 2. Assembling a single `Vec<JsValue>` holding the core-9 values
    ///    followed by the payload's values, in the exact order the
    ///    precomputed shape was built with (see
    ///    [`PrecomputedEventShapes`] / `build_precomputed_event_shapes`).
    /// 3. Calling [`VmInner::define_with_precomputed_shape`] once to
    ///    publish the final shape + slots in a single operation.
    ///
    /// This replaces 9 + N `define_shaped_property` calls (each doing
    /// a transition-cache hashmap lookup + insertion-order clones
    /// inside `shape_add_transition`) with a single storage
    /// replacement, plus it eliminates the per-payload-property
    /// `strings.intern(name)` calls — payload keys are pre-interned
    /// into `WellKnownStrings` at `Vm::new` time.
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
        let type_sid = g.strings.intern(&event.event_type);
        let mut slots: Vec<JsValue> = Vec::with_capacity(9 + 8); // 9 core + up to 8 payload (Mouse)
        slots.push(JsValue::String(type_sid));
        slots.push(JsValue::Boolean(event.bubbles));
        slots.push(JsValue::Boolean(event.cancelable));
        slots.push(JsValue::Number(f64::from(event.phase as u8)));
        slots.push(JsValue::Object(target_wrapper_id));
        slots.push(JsValue::Object(current_target_wrapper_id));
        // `timeStamp` held at 0.0 until `performance.now()` lands in
        // PR4 (tracked in zesty-inventing-galaxy.md PR4 section).
        slots.push(JsValue::Number(0.0));
        slots.push(JsValue::Boolean(event.composed));
        slots.push(JsValue::Boolean(event.is_trusted));

        // Payload-specific slot values.  May allocate (Focus's
        // relatedTarget via `create_element_wrapper`); the returned
        // wrapper ObjectId is rooted via wrapper_cache before we
        // push it into `slots`.
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
        g.define_with_precomputed_shape(event_id, shape_id, &slots);

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
fn append_payload_slots(vm: &mut VmInner, slots: &mut Vec<JsValue>, payload: &EventPayload) {
    match payload {
        EventPayload::Mouse(m) => {
            // clientX, clientY, button, buttons, altKey, ctrlKey, metaKey, shiftKey
            slots.push(JsValue::Number(m.client_x));
            slots.push(JsValue::Number(m.client_y));
            slots.push(JsValue::Number(f64::from(m.button)));
            slots.push(JsValue::Number(f64::from(m.buttons)));
            slots.push(JsValue::Boolean(m.alt_key));
            slots.push(JsValue::Boolean(m.ctrl_key));
            slots.push(JsValue::Boolean(m.meta_key));
            slots.push(JsValue::Boolean(m.shift_key));
        }
        EventPayload::Keyboard(k) => {
            // key, code, altKey, ctrlKey, metaKey, shiftKey, repeat
            let key_sid = vm.strings.intern(&k.key);
            let code_sid = vm.strings.intern(&k.code);
            slots.push(JsValue::String(key_sid));
            slots.push(JsValue::String(code_sid));
            slots.push(JsValue::Boolean(k.alt_key));
            slots.push(JsValue::Boolean(k.ctrl_key));
            slots.push(JsValue::Boolean(k.meta_key));
            slots.push(JsValue::Boolean(k.shift_key));
            slots.push(JsValue::Boolean(k.repeat));
        }
        EventPayload::Transition(t) => {
            // propertyName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&t.property_name);
            let pe_sid = vm.strings.intern(&t.pseudo_element);
            slots.push(JsValue::String(name_sid));
            slots.push(JsValue::Number(t.elapsed_time));
            slots.push(JsValue::String(pe_sid));
        }
        EventPayload::Animation(a) => {
            // animationName, elapsedTime, pseudoElement
            let name_sid = vm.strings.intern(&a.animation_name);
            let pe_sid = vm.strings.intern(&a.pseudo_element);
            slots.push(JsValue::String(name_sid));
            slots.push(JsValue::Number(a.elapsed_time));
            slots.push(JsValue::String(pe_sid));
        }
        EventPayload::Input(i) => {
            // inputType, data, isComposing
            let type_sid = vm.strings.intern(&i.input_type);
            let data_val = match &i.data {
                Some(s) => JsValue::String(vm.strings.intern(s)),
                None => JsValue::Null,
            };
            slots.push(JsValue::String(type_sid));
            slots.push(data_val);
            slots.push(JsValue::Boolean(i.is_composing));
        }
        EventPayload::Clipboard(c) => {
            // dataType, data
            let type_sid = vm.strings.intern(&c.data_type);
            let data_sid = vm.strings.intern(&c.data);
            slots.push(JsValue::String(type_sid));
            slots.push(JsValue::String(data_sid));
        }
        EventPayload::Composition(c) => {
            // data
            let data_sid = vm.strings.intern(&c.data);
            slots.push(JsValue::String(data_sid));
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
            slots.push(related_val);
        }
        EventPayload::Wheel(w) => {
            // deltaX, deltaY, deltaMode
            slots.push(JsValue::Number(w.delta_x));
            slots.push(JsValue::Number(w.delta_y));
            slots.push(JsValue::Number(f64::from(w.delta_mode)));
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
            slots.push(JsValue::String(data_sid));
            slots.push(JsValue::String(origin_sid));
            slots.push(JsValue::String(last_id_sid));
            // `source` / `ports` populated when MessagePort lands (PR5b).
        }
        EventPayload::CloseEvent(c) => {
            // code, reason, wasClean
            let reason_sid = vm.strings.intern(&c.reason);
            slots.push(JsValue::Number(f64::from(c.code)));
            slots.push(JsValue::String(reason_sid));
            slots.push(JsValue::Boolean(c.was_clean));
        }
        EventPayload::HashChange(h) => {
            // oldURL, newURL
            let old_sid = vm.strings.intern(&h.old_url);
            let new_sid = vm.strings.intern(&h.new_url);
            slots.push(JsValue::String(old_sid));
            slots.push(JsValue::String(new_sid));
        }
        EventPayload::PageTransition(p) => {
            // persisted
            slots.push(JsValue::Boolean(p.persisted));
        }
        EventPayload::Storage {
            key,
            old_value,
            new_value,
            url,
        } => {
            // key, oldValue, newValue, url
            let key_val = match key {
                Some(s) => JsValue::String(vm.strings.intern(s)),
                None => JsValue::Null,
            };
            let old_val = match old_value {
                Some(s) => JsValue::String(vm.strings.intern(s)),
                None => JsValue::Null,
            };
            let new_val = match new_value {
                Some(s) => JsValue::String(vm.strings.intern(s)),
                None => JsValue::Null,
            };
            let url_sid = vm.strings.intern(url);
            slots.push(key_val);
            slots.push(old_val);
            slots.push(new_val);
            slots.push(JsValue::String(url_sid));
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
