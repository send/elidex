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
//! ## Properties installed
//!
//! | Property / Method | Source | Shape |
//! |-------------------|--------|-------|
//! | `type` | `event.event_type` | data, RO |
//! | `bubbles` | `event.bubbles` | data, RO |
//! | `cancelable` | `event.cancelable` | data, RO |
//! | `eventPhase` | `event.phase as u8` | data, RO |
//! | `target` | `target_wrapper_id` | data, RO |
//! | `currentTarget` | `current_target_id` | data, RO |
//! | `timeStamp` | `0.0` (TODO: PR3+) | data, RO |
//! | `composed` | `event.composed` | data, RO |
//! | `isTrusted` | `event.is_trusted` | data, RO |
//! | `defaultPrevented` | live getter → `ObjectKind::Event.default_prevented` | accessor, configurable |
//! | `preventDefault` | `natives_event::…prevent_default` | method |
//! | `stopPropagation` | `natives_event::…stop_propagation` | method |
//! | `stopImmediatePropagation` | `natives_event::…stop_immediate_propagation` | method |
//! | `composedPath` | `natives_event::…composed_path` | method |
//! | `<payload-specific>` | `event.payload` | data, RO |
//!
//! ## Deferred to later PRs
//!
//! - `returnValue` legacy accessor (boa implements; not spec-critical
//!   for WPT).  No major site depends on it; revisit in PR5a when
//!   Event constructor lands and this file gains a dedicated test
//!   pass against WPT `events/Event-*.html`.
//! - `initEvent` / `initXXXEvent` legacy initializers — rare, skipped.
//! - Timestamp population — currently a constant `0.0`; a proper
//!   monotonic-clock read arrives with `performance.now()` in PR4.

#![cfg(feature = "engine")]

use elidex_plugin::EventPayload;
use elidex_script_session::event_dispatch::DispatchEvent;

use super::super::natives_event::{
    native_event_composed_path, native_event_get_default_prevented, native_event_prevent_default,
    native_event_stop_immediate_propagation, native_event_stop_propagation,
};
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeFunction, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue,
};
use super::super::VmInner;

/// Attribute triple used for DOM event properties: `{¬W, E, C}` — per
/// WebIDL `[Reflect]` attributes are non-writable, enumerable, and
/// configurable by default.  Matches boa's `Attribute::READONLY` with
/// an explicit configurable flag.
const EVENT_RO: PropertyAttrs = PropertyAttrs {
    writable: false,
    enumerable: true,
    configurable: true,
    is_accessor: false,
};

const EVENT_ACCESSOR: PropertyAttrs = PropertyAttrs {
    writable: false,
    enumerable: true,
    configurable: true,
    is_accessor: true,
};

/// Well-known string IDs needed by event-object construction.  We resolve
/// them eagerly rather than re-interning on every event creation — event
/// dispatch is a hot path.
struct EventStrings {
    r#type: super::super::value::StringId,
    bubbles: super::super::value::StringId,
    cancelable: super::super::value::StringId,
    event_phase: super::super::value::StringId,
    target: super::super::value::StringId,
    current_target: super::super::value::StringId,
    time_stamp: super::super::value::StringId,
    composed: super::super::value::StringId,
    is_trusted: super::super::value::StringId,
    default_prevented: super::super::value::StringId,
    prevent_default: super::super::value::StringId,
    stop_propagation: super::super::value::StringId,
    stop_immediate_propagation: super::super::value::StringId,
    composed_path: super::super::value::StringId,
}

impl EventStrings {
    fn intern(vm: &mut VmInner) -> Self {
        Self {
            r#type: vm.strings.intern("type"),
            bubbles: vm.strings.intern("bubbles"),
            cancelable: vm.strings.intern("cancelable"),
            event_phase: vm.strings.intern("eventPhase"),
            target: vm.strings.intern("target"),
            current_target: vm.strings.intern("currentTarget"),
            time_stamp: vm.strings.intern("timeStamp"),
            composed: vm.strings.intern("composed"),
            is_trusted: vm.strings.intern("isTrusted"),
            default_prevented: vm.strings.intern("defaultPrevented"),
            prevent_default: vm.strings.intern("preventDefault"),
            stop_propagation: vm.strings.intern("stopPropagation"),
            stop_immediate_propagation: vm.strings.intern("stopImmediatePropagation"),
            composed_path: vm.strings.intern("composedPath"),
        }
    }
}

impl VmInner {
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
    /// GC is suppressed for the duration of construction — every
    /// intermediate alloc (native fn objects, the event itself) is
    /// only reachable via local Rust variables until the property
    /// writes complete, at which point the Event object is the root
    /// via the returned ObjectId.  Callers must root the returned id
    /// immediately (push to stack, store in a frame slot, etc.) before
    /// re-enabling GC.
    pub(crate) fn create_event_object(
        &mut self,
        event: &DispatchEvent,
        target_wrapper_id: ObjectId,
        current_target_wrapper_id: ObjectId,
        passive: bool,
    ) -> ObjectId {
        let saved_gc = self.gc_enabled;
        self.gc_enabled = false;

        let strings = EventStrings::intern(self);
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
            // No prototype — Event.prototype (with full accessors etc.)
            // ships in PR5a alongside `new Event(...)`.  For PR3 the
            // four methods + defaultPrevented accessor are installed
            // as own properties below.
            prototype: self.object_prototype,
            extensible: true,
        });

        // ---- Core properties ----
        let type_sid = self.strings.intern(&event.event_type);
        install_data_ro(self, event_id, strings.r#type, JsValue::String(type_sid));
        install_data_ro(
            self,
            event_id,
            strings.bubbles,
            JsValue::Boolean(event.bubbles),
        );
        install_data_ro(
            self,
            event_id,
            strings.cancelable,
            JsValue::Boolean(event.cancelable),
        );
        install_data_ro(
            self,
            event_id,
            strings.event_phase,
            JsValue::Number(f64::from(event.phase as u8)),
        );
        install_data_ro(
            self,
            event_id,
            strings.target,
            JsValue::Object(target_wrapper_id),
        );
        install_data_ro(
            self,
            event_id,
            strings.current_target,
            JsValue::Object(current_target_wrapper_id),
        );
        install_data_ro(
            self,
            event_id,
            strings.time_stamp,
            // Populated properly once `performance.now()` lands (PR4).
            JsValue::Number(0.0),
        );
        install_data_ro(
            self,
            event_id,
            strings.composed,
            JsValue::Boolean(event.composed),
        );
        install_data_ro(
            self,
            event_id,
            strings.is_trusted,
            JsValue::Boolean(event.is_trusted),
        );

        // ---- defaultPrevented accessor ----
        let getter_id = alloc_native_fn(
            self,
            "get defaultPrevented",
            native_event_get_default_prevented,
        );
        self.define_shaped_property(
            event_id,
            PropertyKey::String(strings.default_prevented),
            PropertyValue::Accessor {
                getter: Some(getter_id),
                setter: None,
            },
            EVENT_ACCESSOR,
        );

        // ---- Four native methods (own data properties) ----
        install_method(
            self,
            event_id,
            strings.prevent_default,
            "preventDefault",
            native_event_prevent_default,
        );
        install_method(
            self,
            event_id,
            strings.stop_propagation,
            "stopPropagation",
            native_event_stop_propagation,
        );
        install_method(
            self,
            event_id,
            strings.stop_immediate_propagation,
            "stopImmediatePropagation",
            native_event_stop_immediate_propagation,
        );
        install_method(
            self,
            event_id,
            strings.composed_path,
            "composedPath",
            native_event_composed_path,
        );

        // ---- Payload-specific properties ----
        set_payload_properties(self, event_id, &event.payload);

        self.gc_enabled = saved_gc;
        event_id
    }
}

fn install_data_ro(
    vm: &mut VmInner,
    obj: ObjectId,
    key: super::super::value::StringId,
    value: JsValue,
) {
    vm.define_shaped_property(
        obj,
        PropertyKey::String(key),
        PropertyValue::Data(value),
        EVENT_RO,
    );
}

fn install_method(
    vm: &mut VmInner,
    obj: ObjectId,
    key: super::super::value::StringId,
    name: &str,
    func: fn(
        &mut super::super::value::NativeContext<'_>,
        JsValue,
        &[JsValue],
    ) -> Result<JsValue, super::super::value::VmError>,
) {
    let fn_id = alloc_native_fn(vm, name, func);
    vm.define_shaped_property(
        obj,
        PropertyKey::String(key),
        PropertyValue::Data(JsValue::Object(fn_id)),
        PropertyAttrs::METHOD,
    );
}

fn alloc_native_fn(
    vm: &mut VmInner,
    name: &str,
    func: fn(
        &mut super::super::value::NativeContext<'_>,
        JsValue,
        &[JsValue],
    ) -> Result<JsValue, super::super::value::VmError>,
) -> ObjectId {
    let name_id = vm.strings.intern(name);
    vm.alloc_object(Object {
        kind: ObjectKind::NativeFunction(NativeFunction {
            name: name_id,
            func,
            constructable: false,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.function_prototype,
        extensible: true,
    })
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
                None => {
                    let key = vm.strings.intern("data");
                    install_data_ro(vm, event_id, key, JsValue::Null);
                }
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
            let key = vm.strings.intern("relatedTarget");
            install_data_ro(vm, event_id, key, related_val);
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
            // `source` / `ports` intentionally omitted — populated by
            // `postMessage` / MessagePort in PR5b; until then the
            // property is absent (undefined on read) rather than null,
            // matching WebKit's behaviour when the bridge has no
            // MessagePort backing.
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
            // `storageArea` omitted — populated when `localStorage` /
            // `sessionStorage` land in PR5a.
        }
        // `Scroll` / `None` carry no extra properties.  The `_` arm
        // catches future `#[non_exhaustive]` variants added upstream
        // without a matching setter here — a silent install of no
        // extra props, picked up in CI via explicit tests per variant
        // in PR3 C12 (integration tests).
        EventPayload::Scroll | EventPayload::None => {}
        _ => {}
    }
}

// ---------------------------------------------------------------------
// Small helpers for property installation.  Kept non-inlined so the
// per-variant code paths above stay readable.
// ---------------------------------------------------------------------

fn install_num(vm: &mut VmInner, obj: ObjectId, name: &str, value: f64) {
    let key = vm.strings.intern(name);
    install_data_ro(vm, obj, key, JsValue::Number(value));
}

fn install_bool(vm: &mut VmInner, obj: ObjectId, name: &str, value: bool) {
    let key = vm.strings.intern(name);
    install_data_ro(vm, obj, key, JsValue::Boolean(value));
}

fn install_str(vm: &mut VmInner, obj: ObjectId, name: &str, value: &str) {
    let key = vm.strings.intern(name);
    let sid = vm.strings.intern(value);
    install_data_ro(vm, obj, key, JsValue::String(sid));
}

fn install_optional_str(vm: &mut VmInner, obj: ObjectId, name: &str, value: Option<&str>) {
    let key = vm.strings.intern(name);
    let js_value = match value {
        Some(s) => JsValue::String(vm.strings.intern(s)),
        None => JsValue::Null,
    };
    install_data_ro(vm, obj, key, js_value);
}
