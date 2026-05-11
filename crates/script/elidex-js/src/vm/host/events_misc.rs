//! D-10 modern + miscellaneous Event constructor classes (slot
//! `#11-events-misc`).
//!
//! Ten new constructable Event interfaces grouped here because they all
//! follow either the [`super::events_extras::register_event_subclass`]
//! pattern (Event base) or the [`super::events_ui::register_descendant`]
//! pattern (UIEvent / MouseEvent base) and share the same init-dict
//! coercion helpers.  Splitting per-interface would multiply the
//! globals.rs register block + well_known SID + scaffold churn for ~80
//! LoC files each — the grouped layout matches `events_extras.rs`
//! precedent (7 ctors / 1 file).
//!
//! WebIDL inheritance:
//!
//! ```text
//! SubmitEvent           : Event
//! FormDataEvent         : Event
//! ToggleEvent           : Event
//! ClipboardEvent        : Event
//! ProgressEvent         : Event
//! BeforeUnloadEvent     : Event   (no constructor — `new` throws)
//! MessageEvent          : Event
//! PageTransitionEvent   : Event
//! CompositionEvent      : UIEvent
//! WheelEvent            : MouseEvent
//! ```
//!
//! ## Layering
//!
//! Engine-bound only — VM-side init-dict coercion + shape-resident
//! own-data slot installation.  No DOM mutation / selector walking
//! happens here; the only DOM-side concern is ToggleEvent dispatch
//! which lives in `event_target_dispatch.rs::dispatch_toggle_event`
//! and consumes the engine-indep `collect_open_details_by_name`
//! walker for `<details>.name` multi-disclosure exclusion.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;
use super::events::{check_construct, parse_event_init, type_arg};
use super::events_extras::{
    opts_object_id, read_any, read_bool, read_number, read_string, register_event_subclass,
};
use super::events_ui::{
    parse_mouse_event_members, parse_ui_event_init, register_descendant, MouseEventMembers,
};

// ---------------------------------------------------------------------------
// VmInner: registration glue
// ---------------------------------------------------------------------------

impl VmInner {
    pub(in crate::vm) fn register_submit_event_global(&mut self) {
        register_event_subclass(
            self,
            "SubmitEvent",
            native_submit_event_constructor,
            self.well_known.submit_event_global,
            |vm, id| vm.submit_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_formdata_event_global(&mut self) {
        register_event_subclass(
            self,
            "FormDataEvent",
            native_formdata_event_constructor,
            self.well_known.formdata_event_global,
            |vm, id| vm.formdata_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_toggle_event_global(&mut self) {
        register_event_subclass(
            self,
            "ToggleEvent",
            native_toggle_event_constructor,
            self.well_known.toggle_event_global,
            |vm, id| vm.toggle_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_composition_event_global(&mut self) {
        register_descendant(
            self,
            self.ui_event_prototype,
            "CompositionEvent",
            native_composition_event_constructor,
            self.well_known.composition_event_global,
            |vm, id| vm.composition_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_clipboard_event_global(&mut self) {
        register_event_subclass(
            self,
            "ClipboardEvent",
            native_clipboard_event_constructor,
            self.well_known.clipboard_event_global,
            |vm, id| vm.clipboard_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_progress_event_global(&mut self) {
        register_event_subclass(
            self,
            "ProgressEvent",
            native_progress_event_constructor,
            self.well_known.progress_event_global,
            |vm, id| vm.progress_event_prototype = Some(id),
        );
    }

    /// `BeforeUnloadEvent` (HTML §9.10.2).  Per spec the interface
    /// has no `[Constructor]`, so `new BeforeUnloadEvent(...)` throws
    /// `TypeError("Illegal constructor")`.  The global is still
    /// registered so that UA-dispatched instances pass `instanceof
    /// BeforeUnloadEvent` (after the §C7 UA-brand fix).
    ///
    /// `returnValue` is a mutable accessor pair installed on
    /// `BeforeUnloadEvent.prototype`; the slot value lives on a
    /// per-instance side table (`return_value_states`) keyed by the
    /// receiver `ObjectId`.  Lazy: instances start with no entry, and
    /// the getter returns `""` for a missing entry to match
    /// `String(returnValue) === ""` for a freshly-fired event.
    pub(in crate::vm) fn register_before_unload_event_global(&mut self) {
        register_event_subclass(
            self,
            "BeforeUnloadEvent",
            native_before_unload_event_constructor,
            self.well_known.before_unload_event_global,
            |vm, id| vm.before_unload_event_prototype = Some(id),
        );
        // Install `returnValue` mutable accessor pair on the prototype.
        let proto_id = self.before_unload_event_prototype.expect(
            "register_before_unload_event_global just stored before_unload_event_prototype",
        );
        let return_value_sid = self.well_known.return_value;
        self.install_accessor_pair(
            proto_id,
            return_value_sid,
            native_before_unload_get_return_value,
            Some(native_before_unload_set_return_value),
            // WEBIDL_RO_ACCESSOR is the read/write accessor variant —
            // see PropertyAttrs::WEBIDL_RO_ACCESSOR docs: writability of
            // an accessor is determined by setter presence, so the
            // same constant is reused for the RW case.
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    pub(in crate::vm) fn register_message_event_global(&mut self) {
        register_event_subclass(
            self,
            "MessageEvent",
            native_message_event_constructor,
            self.well_known.message_event_global,
            |vm, id| vm.message_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_wheel_event_global(&mut self) {
        register_descendant(
            self,
            self.mouse_event_prototype,
            "WheelEvent",
            native_wheel_event_constructor,
            self.well_known.wheel_event_global,
            |vm, id| vm.wheel_event_prototype = Some(id),
        );
        // Install DOM_DELTA_* numeric constants on `WheelEvent.prototype`
        // (UI Events §5.5).  Property attrs match the WebIDL `const`
        // declaration: enumerable but neither writable nor
        // configurable — `BUILTIN` matches the existing pattern used
        // for `Event.NONE` / `CAPTURING_PHASE` etc. (which the engine
        // currently does NOT install, so this is the first set).
        let proto_id = self
            .wheel_event_prototype
            .expect("register_wheel_event_global just stored wheel_event_prototype");
        let constants = [
            (self.well_known.dom_delta_pixel, 0.0_f64),
            (self.well_known.dom_delta_line, 1.0_f64),
            (self.well_known.dom_delta_page, 2.0_f64),
        ];
        for (sid, value) in constants {
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(sid),
                PropertyValue::Data(JsValue::Number(value)),
                shape::PropertyAttrs::BUILTIN,
            );
        }
    }

    pub(in crate::vm) fn register_page_transition_event_global(&mut self) {
        register_event_subclass(
            self,
            "PageTransitionEvent",
            native_page_transition_event_constructor,
            self.well_known.page_transition_event_global,
            |vm, id| vm.page_transition_event_prototype = Some(id),
        );
    }

    /// Reverse map: `EventPayload` variant → subclass prototype that
    /// UA-dispatched events of that payload should chain through.
    ///
    /// Pre-D-10 the engine hardcoded `prototype: self.event_prototype`
    /// in `events::create_event_object`, which meant
    /// `el.addEventListener('click', e => e instanceof MouseEvent)`
    /// returned `false` for all UA-dispatched events.  D-10 §C-12
    /// replaces that hardcode with a call to this method.
    ///
    /// Returns `event_prototype` as a fallback for any variant whose
    /// subclass prototype isn't yet registered (registration ordering
    /// safety — `register_globals` installs every subclass before any
    /// UA dispatch could fire, but the `.or(event_prototype)` belt-and-
    /// suspenders avoids a panic if the order changes).
    pub(crate) fn prototype_for_payload(
        &self,
        payload: &elidex_plugin::EventPayload,
    ) -> Option<ObjectId> {
        use elidex_plugin::EventPayload as P;
        match payload {
            P::Mouse(_) => self.mouse_event_prototype.or(self.event_prototype),
            P::Keyboard(_) => self.keyboard_event_prototype.or(self.event_prototype),
            P::Transition(_) => self.transition_event_prototype.or(self.event_prototype),
            P::Animation(_) => self.animation_event_prototype.or(self.event_prototype),
            P::Input(_) => self.input_event_prototype.or(self.event_prototype),
            P::Clipboard(_) => self.clipboard_event_prototype.or(self.event_prototype),
            P::Composition(_) => self.composition_event_prototype.or(self.event_prototype),
            P::Focus(_) => self.focus_event_prototype.or(self.event_prototype),
            P::Wheel(_) => self.wheel_event_prototype.or(self.event_prototype),
            P::Message { .. } => self.message_event_prototype.or(self.event_prototype),
            P::CloseEvent(_) => self.close_event_prototype.or(self.event_prototype),
            P::HashChange(_) => self.hash_change_event_prototype.or(self.event_prototype),
            P::PageTransition(_) => self
                .page_transition_event_prototype
                .or(self.event_prototype),
            P::Storage { .. } => self.storage_event_prototype.or(self.event_prototype),
            // `Scroll` has no dedicated subclass; `None` (default) is
            // for generic Event-typed dispatches.  Wildcard arm covers
            // the `#[non_exhaustive]` gap — defensive against a future
            // variant added in elidex-plugin without a paired subclass
            // here.  All three paths fall back to `Event.prototype`,
            // collapsed via the wildcard so a new variant added
            // upstream silently inherits the safe default.
            _ => self.event_prototype,
        }
    }
}

// ---------------------------------------------------------------------------
// SubmitEvent (HTML §4.10.21.5.5)
// ---------------------------------------------------------------------------

fn native_submit_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "SubmitEvent")?;
    let type_sid = type_arg(ctx, args, "SubmitEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "SubmitEvent")?;
    let opts_id = opts_object_id(init_arg);
    // `submitter: HTMLElement? = null` — pass-through.  No brand
    // check (Chrome / Firefox accept arbitrary objects, returning the
    // value verbatim from the getter).
    let submitter = match opts_id {
        Some(id) => read_any(ctx, id, ctx.vm.well_known.submitter, JsValue::Null)?,
        None => JsValue::Null,
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .submit_event;
    let slots = vec![PropertyValue::Data(submitter)];
    let mut g = ctx.vm.push_temp_root(submitter);
    let _proto = g
        .submit_event_prototype
        .expect("SubmitEvent.prototype must be registered before ctor");
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// FormDataEvent (HTML §4.10.21.5.4)
// ---------------------------------------------------------------------------

fn native_formdata_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "FormDataEvent")?;
    let type_sid = type_arg(ctx, args, "FormDataEvent")?;
    // FormDataEventInit has `required formData` — missing 2nd arg or
    // empty dict produces a TypeError per WebIDL §3.10.23 (matches
    // Chrome wording, mirrors PromiseRejectionEvent precedent).
    let Some(init_arg) = args.get(1).copied() else {
        return Err(VmError::type_error(
            "Failed to construct 'FormDataEvent': 2 arguments required, but only 1 present.",
        ));
    };
    let opts_id = match init_arg {
        JsValue::Object(id) => Some(id),
        // null / undefined → empty dict (the required-member check
        // below drives the error message).
        JsValue::Null | JsValue::Undefined => None,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'FormDataEvent': \
                 parameter 2 is not of type 'FormDataEventInit'.",
            ));
        }
    };
    let base = parse_event_init(ctx, init_arg, "FormDataEvent")?;
    let k_form_data = ctx.vm.well_known.form_data;
    let form_data_val = match opts_id {
        Some(id) => ctx
            .vm
            .get_property_value(id, PropertyKey::String(k_form_data))?,
        None => JsValue::Undefined,
    };
    if matches!(form_data_val, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to construct 'FormDataEvent': \
             required member formData is undefined.",
        ));
    }
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .formdata_event;
    let slots = vec![PropertyValue::Data(form_data_val)];
    let mut g = ctx.vm.push_temp_root(form_data_val);
    let _proto = g
        .formdata_event_prototype
        .expect("FormDataEvent.prototype must be registered before ctor");
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// ToggleEvent (HTML §4.11.1.5)
// ---------------------------------------------------------------------------

fn native_toggle_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "ToggleEvent")?;
    let type_sid = type_arg(ctx, args, "ToggleEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "ToggleEvent")?;
    let opts_id = opts_object_id(init_arg);
    let (old_state_sid, new_state_sid) = if let Some(opts_id) = opts_id {
        let k_old = ctx.vm.well_known.old_state;
        let k_new = ctx.vm.well_known.new_state;
        let o = read_string(ctx, opts_id, k_old)?;
        let n = read_string(ctx, opts_id, k_new)?;
        (o, n)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, empty)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .toggle_event;
    // Slot order matches the shape transition: newState, oldState
    // (matches DevTools enumeration + dispatch_toggle_event).
    let slots = vec![
        PropertyValue::Data(JsValue::String(new_state_sid)),
        PropertyValue::Data(JsValue::String(old_state_sid)),
    ];
    let _proto = ctx
        .vm
        .toggle_event_prototype
        .expect("ToggleEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// CompositionEvent (UI Events §5.6)
// ---------------------------------------------------------------------------

fn native_composition_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "CompositionEvent")?;
    let type_sid = type_arg(ctx, args, "CompositionEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "CompositionEvent")?;
    let opts_id = opts_object_id(init_arg);
    let data_sid = if let Some(opts_id) = opts_id {
        super::events_ui::read_string(ctx, opts_id, ctx.vm.well_known.data)?
    } else {
        ctx.vm.strings.intern("")
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .composition_event_constructed;
    let slots = vec![PropertyValue::Data(JsValue::String(data_sid))];
    let proto = ctx
        .vm
        .composition_event_prototype
        .expect("CompositionEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .build_ui_event_instance(this, type_sid, ui, shape_id, proto, slots);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// ClipboardEvent (Clipboard API §3)
// ---------------------------------------------------------------------------

fn native_clipboard_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "ClipboardEvent")?;
    let type_sid = type_arg(ctx, args, "ClipboardEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "ClipboardEvent")?;
    let opts_id = opts_object_id(init_arg);
    // `clipboardData: DataTransfer? = null` — pass-through (no brand
    // check; DataTransfer wrapper deferred to D-9
    // `#11-events-modern-input` via `#11-event-modern-extras`).
    let clipboard_data = match opts_id {
        Some(id) => read_any(ctx, id, ctx.vm.well_known.clipboard_data, JsValue::Null)?,
        None => JsValue::Null,
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .clipboard_event_constructed;
    let slots = vec![PropertyValue::Data(clipboard_data)];
    let mut g = ctx.vm.push_temp_root(clipboard_data);
    let _proto = g
        .clipboard_event_prototype
        .expect("ClipboardEvent.prototype must be registered before ctor");
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// ProgressEvent (XHR §10)
// ---------------------------------------------------------------------------

fn native_progress_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "ProgressEvent")?;
    let type_sid = type_arg(ctx, args, "ProgressEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "ProgressEvent")?;
    let opts_id = opts_object_id(init_arg);
    let (length_computable, loaded, total) = if let Some(opts_id) = opts_id {
        let k_lc = ctx.vm.well_known.length_computable;
        let k_loaded = ctx.vm.well_known.loaded;
        let k_total = ctx.vm.well_known.total;
        let lc = read_bool(ctx, opts_id, k_lc, false)?;
        // `loaded` / `total` are WebIDL `unsigned long long` (§3.10.10).
        // ToNumber + clamp-to-non-negative-integer; for storage we keep
        // the raw `f64` (the ULLong→f64 round-trip preserves up to 2^53
        // exactly, beyond which precision degrades — matches Chrome).
        let l = read_number(ctx, opts_id, k_loaded, 0.0)?;
        let t = read_number(ctx, opts_id, k_total, 0.0)?;
        (lc, l, t)
    } else {
        (false, 0.0, 0.0)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .progress_event;
    let slots = vec![
        PropertyValue::Data(JsValue::Boolean(length_computable)),
        PropertyValue::Data(JsValue::Number(loaded)),
        PropertyValue::Data(JsValue::Number(total)),
    ];
    let _proto = ctx
        .vm
        .progress_event_prototype
        .expect("ProgressEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// BeforeUnloadEvent (HTML §9.10.2)
// ---------------------------------------------------------------------------

/// `new BeforeUnloadEvent(...)` always throws — per WHATWG IDL the
/// interface declares no `[Constructor]`.  The thrown `TypeError` text
/// matches Chrome's "Illegal constructor" wording (also used by other
/// constructable-only WebIDL interfaces in the workspace).
fn native_before_unload_event_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error("Illegal constructor"))
}

/// `BeforeUnloadEvent.prototype.returnValue` getter — reads the
/// per-instance `returnValue` slot from the side table, defaulting to
/// the empty string when no entry exists (matches a freshly-fired UA
/// event before any handler writes the slot).
fn native_before_unload_get_return_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    Ok(ctx
        .vm
        .before_unload_return_values
        .get(&id)
        .map_or(JsValue::String(ctx.vm.well_known.empty), |&sid| {
            JsValue::String(sid)
        }))
}

/// `BeforeUnloadEvent.prototype.returnValue` setter — coerces the
/// new value via WebIDL `DOMString` (`ToString`, throws on Symbol) and
/// stores it in the per-instance side table.
fn native_before_unload_set_return_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    ctx.vm.before_unload_return_values.insert(id, sid);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// MessageEvent (HTML §9.4.4)
// ---------------------------------------------------------------------------

fn native_message_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "MessageEvent")?;
    let type_sid = type_arg(ctx, args, "MessageEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "MessageEvent")?;
    let opts_id = opts_object_id(init_arg);
    let (data_val, origin_sid, last_event_id_sid, source_val, ports_val) =
        if let Some(opts_id) = opts_id {
            let k_data = ctx.vm.well_known.data;
            let k_origin = ctx.vm.well_known.origin;
            let k_last = ctx.vm.well_known.last_event_id;
            let k_source = ctx.vm.well_known.source;
            let k_ports = ctx.vm.well_known.ports;
            let data_val = read_any(ctx, opts_id, k_data, JsValue::Null)?;
            let origin_sid = read_string(ctx, opts_id, k_origin)?;
            let last_sid = read_string(ctx, opts_id, k_last)?;
            // `source: MessageEventSource? = null` — `any` pass-through
            // (no brand check; MessagePort wrapper deferred to `#11b`).
            let source_val = read_any(ctx, opts_id, k_source, JsValue::Null)?;
            // `ports: sequence<MessagePort> = []` — `any` pass-through;
            // missing key materialises a fresh empty Array via
            // `create_array_object` so `e.ports` is always Array-shaped.
            let ports_raw = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(k_ports))?;
            let ports_val = if matches!(ports_raw, JsValue::Undefined) {
                let arr = ctx.vm.create_array_object(Vec::new());
                JsValue::Object(arr)
            } else {
                ports_raw
            };
            (data_val, origin_sid, last_sid, source_val, ports_val)
        } else {
            let empty = ctx.vm.strings.intern("");
            let arr = ctx.vm.create_array_object(Vec::new());
            (
                JsValue::Null,
                empty,
                empty,
                JsValue::Null,
                JsValue::Object(arr),
            )
        };
    // Reuses the UA-dispatch `message` shape — slot order
    // (data, origin, lastEventId, source, ports) matches both the
    // init dict order and the existing UA dispatch path.
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .message;
    let slots = vec![
        PropertyValue::Data(data_val),
        PropertyValue::Data(JsValue::String(origin_sid)),
        PropertyValue::Data(JsValue::String(last_event_id_sid)),
        PropertyValue::Data(source_val),
        PropertyValue::Data(ports_val),
    ];
    // Root the Object-flavoured slot values across allocation so a GC
    // mid-`create_fresh_event_object` doesn't sweep them before they
    // land in their slots.
    let mut g0 = ctx.vm.push_temp_root(data_val);
    let mut g1 = g0.push_temp_root(source_val);
    let mut g = g1.push_temp_root(ports_val);
    let _proto = g
        .message_event_prototype
        .expect("MessageEvent.prototype must be registered before ctor");
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    drop(g);
    drop(g1);
    drop(g0);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// WheelEvent (UI Events §5.5)
// ---------------------------------------------------------------------------

// `dx` / `dy` / `dz` / `dm` are spec-mandated WebIDL field names
// (deltaX / deltaY / deltaZ / deltaMode); single-letter shorthand is
// the natural binding name for them.  Same suppression as
// `native_mouse_event_constructor` historically used.
#[allow(clippy::similar_names)]
fn native_wheel_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "WheelEvent")?;
    let type_sid = type_arg(ctx, args, "WheelEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "WheelEvent")?;
    let opts_id = opts_object_id(init_arg);
    let MouseEventMembers {
        related_target,
        mut slots,
    } = parse_mouse_event_members(ctx, opts_id)?;
    // 4 wheel-specific slots: deltaX, deltaY, deltaZ, deltaMode.
    // `delta_mode` is WebIDL `unsigned long` (§3.10.10) — ToNumber +
    // ToUint32 clamp.
    let (delta_x, delta_y, delta_z, delta_mode) = if let Some(opts_id) = opts_id {
        let k_dx = ctx.vm.well_known.delta_x;
        let k_dy = ctx.vm.well_known.delta_y;
        let k_dz = ctx.vm.well_known.delta_z;
        let k_dm = ctx.vm.well_known.delta_mode;
        let dx = read_number(ctx, opts_id, k_dx, 0.0)?;
        let dy = read_number(ctx, opts_id, k_dy, 0.0)?;
        let dz = read_number(ctx, opts_id, k_dz, 0.0)?;
        let dm_raw = read_number(ctx, opts_id, k_dm, 0.0)?;
        let dm = f64::from(super::super::coerce::f64_to_uint32(dm_raw));
        (dx, dy, dz, dm)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };
    slots.push(PropertyValue::Data(JsValue::Number(delta_x)));
    slots.push(PropertyValue::Data(JsValue::Number(delta_y)));
    slots.push(PropertyValue::Data(JsValue::Number(delta_z)));
    slots.push(PropertyValue::Data(JsValue::Number(delta_mode)));
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .wheel_event_constructed;
    let mut g = ctx.vm.push_temp_root(related_target);
    let wheel_proto = g
        .wheel_event_prototype
        .expect("WheelEvent.prototype must be registered before ctor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, wheel_proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// PageTransitionEvent (HTML §7.10.1.7.4)
// ---------------------------------------------------------------------------

fn native_page_transition_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "PageTransitionEvent")?;
    let type_sid = type_arg(ctx, args, "PageTransitionEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "PageTransitionEvent")?;
    let opts_id = opts_object_id(init_arg);
    let persisted = if let Some(opts_id) = opts_id {
        read_bool(ctx, opts_id, ctx.vm.well_known.persisted, false)?
    } else {
        false
    };
    // Reuses the UA-dispatch `page_transition` shape (single
    // `persisted` slot).
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .page_transition;
    let slots = vec![PropertyValue::Data(JsValue::Boolean(persisted))];
    let _proto = ctx
        .vm
        .page_transition_event_prototype
        .expect("PageTransitionEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    Ok(JsValue::Object(id))
}
