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
    JsValue, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, PropertyValue, StringId,
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
    /// # GC safety
    ///
    /// The just-allocated event id is rooted internally via
    /// [`VmInner::push_temp_root`] across all property installs
    /// (Focus payloads' `relatedTarget` allocates a wrapper, which
    /// could otherwise trigger GC and reclaim the in-progress event
    /// obj).  The guard drops before return — so the returned
    /// `ObjectId` becomes vulnerable to collection from the next
    /// allocation by the caller.  Root it immediately
    /// (push to stack via [`VmInner::push_temp_root`], store in a
    /// frame slot, etc.) before any further VM operations that may
    /// allocate or run user JS.
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

        // Root the just-allocated event_id across all property installs.
        // `set_payload_properties` may call `create_element_wrapper`
        // (Focus payload's `relatedTarget`), which allocates and could
        // trigger GC — without rooting, the event obj would be the
        // only thing tying its prototype/payload to a root and would
        // be reclaimed mid-construction.  RAII guard restores the
        // stack on drop, including during panic unwinding.
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

        // ---- Core properties (cached property names from WellKnownStrings) ----
        let type_sid = g.strings.intern(&event.event_type);
        let phase = event.phase as u8;
        let composed = event.composed;
        let bubbles = event.bubbles;
        let cancelable = event.cancelable;
        let is_trusted = event.is_trusted;
        // Hoist the StringId reads so the compiler doesn't have to
        // re-borrow `g.well_known` interleaved with `&mut g` calls
        // to `define_shaped_property`.
        let wk = g.well_known_event_keys();

        install_ro(&mut g, event_id, wk.r#type, JsValue::String(type_sid));
        install_ro(&mut g, event_id, wk.bubbles, JsValue::Boolean(bubbles));
        install_ro(
            &mut g,
            event_id,
            wk.cancelable,
            JsValue::Boolean(cancelable),
        );
        install_ro(
            &mut g,
            event_id,
            wk.event_phase,
            JsValue::Number(f64::from(phase)),
        );
        install_ro(
            &mut g,
            event_id,
            wk.target,
            JsValue::Object(target_wrapper_id),
        );
        install_ro(
            &mut g,
            event_id,
            wk.current_target,
            JsValue::Object(current_target_wrapper_id),
        );
        // `timeStamp` is the wall-clock-ish DOM event timestamp.  Held
        // at 0.0 until `performance.now()` lands in PR4 — flagged in
        // module doc.
        install_ro(&mut g, event_id, wk.time_stamp, JsValue::Number(0.0));
        install_ro(&mut g, event_id, wk.composed, JsValue::Boolean(composed));
        install_ro(
            &mut g,
            event_id,
            wk.is_trusted,
            JsValue::Boolean(is_trusted),
        );

        // ---- Payload-specific properties ----
        set_payload_properties(&mut g, event_id, &event.payload);

        drop(g);
        event_id
    }

    /// Snapshot of the WellKnownStrings entries used per event-object
    /// construction.  All `Copy` (StringId is `u32`-newtype) — collected
    /// once at the top of `create_event_object` so the `&mut self`
    /// `install_ro` calls don't have to fight a `&self.well_known`
    /// borrow.
    fn well_known_event_keys(&self) -> EventKeys {
        EventKeys {
            r#type: self.well_known.event_type,
            bubbles: self.well_known.bubbles,
            cancelable: self.well_known.cancelable,
            event_phase: self.well_known.event_phase,
            target: self.well_known.target,
            current_target: self.well_known.current_target,
            time_stamp: self.well_known.time_stamp,
            composed: self.well_known.composed,
            is_trusted: self.well_known.is_trusted,
        }
    }
}

#[derive(Copy, Clone)]
struct EventKeys {
    r#type: StringId,
    bubbles: StringId,
    cancelable: StringId,
    event_phase: StringId,
    target: StringId,
    current_target: StringId,
    time_stamp: StringId,
    composed: StringId,
    is_trusted: StringId,
}

fn install_ro(vm: &mut VmInner, obj: ObjectId, key: StringId, value: JsValue) {
    vm.define_shaped_property(
        obj,
        PropertyKey::String(key),
        PropertyValue::Data(value),
        PropertyAttrs::WEBIDL_RO,
    );
}

/// Install payload-specific read-only properties per `EventPayload`
/// variant.  Matches the boa implementation in
/// `crates/script/elidex-js-boa/src/globals/events.rs::set_payload_properties`.
#[allow(clippy::too_many_lines)]
fn set_payload_properties(vm: &mut VmInner, event_id: ObjectId, payload: &EventPayload) {
    match payload {
        EventPayload::Mouse(m) => {
            install_num(vm, event_id, "clientX", m.client_x);
            install_num(vm, event_id, "clientY", m.client_y);
            install_num(vm, event_id, "button", f64::from(m.button));
            install_num(vm, event_id, "buttons", f64::from(m.buttons));
            install_bool(vm, event_id, "altKey", m.alt_key);
            install_bool(vm, event_id, "ctrlKey", m.ctrl_key);
            install_bool(vm, event_id, "metaKey", m.meta_key);
            install_bool(vm, event_id, "shiftKey", m.shift_key);
        }
        EventPayload::Keyboard(k) => {
            install_str(vm, event_id, "key", &k.key);
            install_str(vm, event_id, "code", &k.code);
            install_bool(vm, event_id, "altKey", k.alt_key);
            install_bool(vm, event_id, "ctrlKey", k.ctrl_key);
            install_bool(vm, event_id, "metaKey", k.meta_key);
            install_bool(vm, event_id, "shiftKey", k.shift_key);
            install_bool(vm, event_id, "repeat", k.repeat);
        }
        EventPayload::Transition(t) => {
            install_str(vm, event_id, "propertyName", &t.property_name);
            install_num(vm, event_id, "elapsedTime", t.elapsed_time);
            install_str(vm, event_id, "pseudoElement", &t.pseudo_element);
        }
        EventPayload::Animation(a) => {
            install_str(vm, event_id, "animationName", &a.animation_name);
            install_num(vm, event_id, "elapsedTime", a.elapsed_time);
            install_str(vm, event_id, "pseudoElement", &a.pseudo_element);
        }
        EventPayload::Input(i) => {
            install_str(vm, event_id, "inputType", &i.input_type);
            match &i.data {
                Some(s) => install_str(vm, event_id, "data", s),
                None => install_named(vm, event_id, "data", JsValue::Null),
            }
            install_bool(vm, event_id, "isComposing", i.is_composing);
        }
        EventPayload::Clipboard(c) => {
            install_str(vm, event_id, "dataType", &c.data_type);
            install_str(vm, event_id, "data", &c.data);
        }
        EventPayload::Composition(c) => {
            install_str(vm, event_id, "data", &c.data);
        }
        EventPayload::Focus(f) => {
            // `Entity::to_bits().get()` is NonZeroU64, so a `0` bits
            // value is a payload construction bug.  Fall back to
            // `null` rather than panic so a malformed payload still
            // produces a sensible JS value.
            let related_val = match f.related_target.and_then(elidex_ecs::Entity::from_bits) {
                Some(entity) => JsValue::Object(vm.create_element_wrapper(entity)),
                None => JsValue::Null,
            };
            install_named(vm, event_id, "relatedTarget", related_val);
        }
        EventPayload::Wheel(w) => {
            install_num(vm, event_id, "deltaX", w.delta_x);
            install_num(vm, event_id, "deltaY", w.delta_y);
            install_num(vm, event_id, "deltaMode", f64::from(w.delta_mode));
        }
        EventPayload::Message {
            data,
            origin,
            last_event_id,
        } => {
            install_str(vm, event_id, "data", data);
            install_str(vm, event_id, "origin", origin);
            install_str(vm, event_id, "lastEventId", last_event_id);
            // `source` / `ports` populated when MessagePort lands (PR5b).
        }
        EventPayload::CloseEvent(c) => {
            install_num(vm, event_id, "code", f64::from(c.code));
            install_str(vm, event_id, "reason", &c.reason);
            install_bool(vm, event_id, "wasClean", c.was_clean);
        }
        EventPayload::HashChange(h) => {
            install_str(vm, event_id, "oldURL", &h.old_url);
            install_str(vm, event_id, "newURL", &h.new_url);
        }
        EventPayload::PageTransition(p) => {
            install_bool(vm, event_id, "persisted", p.persisted);
        }
        EventPayload::Storage {
            key,
            old_value,
            new_value,
            url,
        } => {
            install_optional_str(vm, event_id, "key", key.as_deref());
            install_optional_str(vm, event_id, "oldValue", old_value.as_deref());
            install_optional_str(vm, event_id, "newValue", new_value.as_deref());
            install_str(vm, event_id, "url", url);
            // `storageArea` populated when localStorage / sessionStorage land (PR5a).
        }
        EventPayload::Scroll | EventPayload::None => {
            // No extra properties.
        }
        // `EventPayload` is `#[non_exhaustive]`.  A new upstream
        // variant landing without a matching arm here installs no
        // payload props.  Debug-trips so test runs surface the
        // omission; release silently no-ops to avoid hard-failing
        // dispatch on payloads we just don't display yet.
        _ => debug_assert!(
            false,
            "unhandled EventPayload variant in set_payload_properties"
        ),
    }
}

// ---------------------------------------------------------------------
// Per-payload installation helpers.  Each interns the property name on
// the StringPool — payload property names (`clientX`, `key`, etc.) are
// not yet in `WellKnownStrings`; promoting the hottest ones is a
// follow-up perf win once profiling identifies them.
// ---------------------------------------------------------------------

fn install_named(vm: &mut VmInner, obj: ObjectId, name: &str, value: JsValue) {
    let key = vm.strings.intern(name);
    install_ro(vm, obj, key, value);
}

fn install_num(vm: &mut VmInner, obj: ObjectId, name: &str, value: f64) {
    install_named(vm, obj, name, JsValue::Number(value));
}

fn install_bool(vm: &mut VmInner, obj: ObjectId, name: &str, value: bool) {
    install_named(vm, obj, name, JsValue::Boolean(value));
}

fn install_str(vm: &mut VmInner, obj: ObjectId, name: &str, value: &str) {
    let sid = vm.strings.intern(value);
    install_named(vm, obj, name, JsValue::String(sid));
}

fn install_optional_str(vm: &mut VmInner, obj: ObjectId, name: &str, value: Option<&str>) {
    let js_value = match value {
        Some(s) => JsValue::String(vm.strings.intern(s)),
        None => JsValue::Null,
    };
    install_named(vm, obj, name, js_value);
}
