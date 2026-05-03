//! Non-UIEvent specialized Event constructors.
//!
//! WebIDL interfaces covered here extend `Event` **directly** (not
//! `UIEvent`) and therefore chain `<Descendant>.prototype →
//! Event.prototype` without the UIEvent prefix:
//!
//! ```text
//! PromiseRejectionEvent : Event  (HTML §8.1.7.3.4)
//! ErrorEvent            : Event  (HTML §8.1.7.2)
//! HashChangeEvent       : Event  (HTML §8.1.3)
//! PopStateEvent         : Event  (HTML §8.8.1)
//! ```
//!
//! Each ctor goes through the shared
//! [`VmInner::create_fresh_event_object`] entry point with an explicit
//! `shape_id` drawn from [`PrecomputedEventShapes`] — shape selection
//! is detached from `EventPayload`, letting ctors pick any terminal
//! shape without round-tripping through the UA dispatch payload.
//! Prototype selection is intentionally left to the normal
//! construction path so `new.target.prototype` is preserved,
//! including for `class Sub extends …` subclasses; the resulting
//! chain therefore ends at the appropriate descendant `.prototype`
//! rather than being forcibly reset to `Event.prototype` (an
//! earlier revision did overwrite unconditionally, which silently
//! broke subclass inheritance — see `build_event_subclass_instance`
//! doc for the load-bearing `_descendant_proto` registration check).

#![cfg(feature = "engine")]

use super::super::shape::ShapeId;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::events::{check_construct, install_ctor, parse_event_init, type_arg, EventInit};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn opts_object_id(val: JsValue) -> Option<ObjectId> {
    match val {
        JsValue::Object(id) => Some(id),
        _ => None,
    }
}

/// Read a string init-dict member — missing / undefined → empty string
/// default.  Non-string values coerce via ToString (Symbol throws).
fn read_string(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
) -> Result<StringId, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(ctx.vm.strings.intern("")),
        _ => super::super::coerce::to_string(ctx.vm, v),
    }
}

/// Read a numeric init-dict member — `undefined` → `default`.
fn read_number(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
    default: f64,
) -> Result<f64, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(default),
        _ => super::super::coerce::to_number(ctx.vm, v),
    }
}

/// Read an `any`-typed init-dict member — missing / `undefined` →
/// supplied `default`; otherwise pass through unchanged (WebIDL `any`
/// preserves the value, including non-default `undefined` if the caller
/// passes `default = JsValue::Undefined`).
fn read_any(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
    default: JsValue,
) -> Result<JsValue, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    Ok(if matches!(v, JsValue::Undefined) {
        default
    } else {
        v
    })
}

/// Read a boolean init-dict member — `undefined` → `default`; otherwise
/// coerce via WebIDL `boolean` semantics (`ToBoolean`, never throws).
fn read_bool(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
    default: bool,
) -> Result<bool, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    Ok(match v {
        JsValue::Undefined => default,
        _ => super::super::coerce::to_boolean(ctx.vm, v),
    })
}

/// Read a `unsigned short` init-dict member — `undefined` → `default`;
/// otherwise coerce via WebIDL `unsigned short` semantics (ToNumber +
/// modulo-2^16 truncation, no `[EnforceRange]`).  Used by
/// `CloseEvent.code` (no `[EnforceRange]` in the WHATWG IDL).
fn read_uint16(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
    default: u16,
) -> Result<u16, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(default),
        _ => super::super::coerce::to_uint16(ctx.vm, v),
    }
}

// ---------------------------------------------------------------------------
// VmInner: registration glue
// ---------------------------------------------------------------------------

impl VmInner {
    pub(in crate::vm) fn register_promise_rejection_event_global(&mut self) {
        register_event_subclass(
            self,
            "PromiseRejectionEvent",
            native_promise_rejection_event_constructor,
            self.well_known.promise_rejection_event_global,
            |vm, id| vm.promise_rejection_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_error_event_global(&mut self) {
        register_event_subclass(
            self,
            "ErrorEvent",
            native_error_event_constructor,
            self.well_known.error_event_global,
            |vm, id| vm.error_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_hash_change_event_global(&mut self) {
        register_event_subclass(
            self,
            "HashChangeEvent",
            native_hash_change_event_constructor,
            self.well_known.hash_change_event_global,
            |vm, id| vm.hash_change_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_pop_state_event_global(&mut self) {
        register_event_subclass(
            self,
            "PopStateEvent",
            native_pop_state_event_constructor,
            self.well_known.pop_state_event_global,
            |vm, id| vm.pop_state_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_animation_event_global(&mut self) {
        register_event_subclass(
            self,
            "AnimationEvent",
            native_animation_event_constructor,
            self.well_known.animation_event_global,
            |vm, id| vm.animation_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_transition_event_global(&mut self) {
        register_event_subclass(
            self,
            "TransitionEvent",
            native_transition_event_constructor,
            self.well_known.transition_event_global,
            |vm, id| vm.transition_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_close_event_global(&mut self) {
        register_event_subclass(
            self,
            "CloseEvent",
            native_close_event_constructor,
            self.well_known.close_event_global,
            |vm, id| vm.close_event_prototype = Some(id),
        );
    }

    /// Finalise a freshly-built Event from `create_fresh_event_object`.
    /// Shared across the four non-UIEvent ctors (PromiseRejectionEvent
    /// / ErrorEvent / HashChangeEvent / PopStateEvent).
    ///
    /// The prototype is left as set by
    /// `create_fresh_event_object.ensure_instance_or_alloc` — in
    /// construct-mode that preserves `new.target.prototype` (i.e.
    /// `PromiseRejectionEvent.prototype` for direct construction,
    /// or `Sub.prototype` for `class Sub extends PromiseRejectionEvent
    /// ; new Sub()`).  A prior revision unconditionally overwrote
    /// the prototype to `descendant_proto`, which silently broke
    /// subclass chains.  `_descendant_proto` is retained in the
    /// signature as a load-bearing registration-check (callers
    /// `.expect()` their `VmInner::<name>_prototype` so a missed
    /// registration panics at ctor entry instead of producing a
    /// latent wrong-chain bug) but the value itself is no longer
    /// applied.
    fn build_event_subclass_instance(
        &mut self,
        this: JsValue,
        type_sid: StringId,
        base: EventInit,
        shape_id: ShapeId,
        _descendant_proto: ObjectId,
        payload_slots: Vec<PropertyValue>,
    ) -> ObjectId {
        self.create_fresh_event_object(this, type_sid, base, shape_id, payload_slots, false)
    }
}

fn register_event_subclass(
    vm: &mut VmInner,
    name: &str,
    func: NativeFn,
    global_sid: StringId,
    store: impl FnOnce(&mut VmInner, ObjectId),
) {
    let parent = vm
        .event_prototype
        .expect("Event.prototype must be registered first");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    store(vm, proto_id);
    install_ctor(vm, proto_id, name, func, global_sid);
}

// ---------------------------------------------------------------------------
// PromiseRejectionEvent (HTML §8.1.7.3.4)
// ---------------------------------------------------------------------------

fn native_promise_rejection_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "PromiseRejectionEvent")?;
    let type_sid = type_arg(ctx, args, "PromiseRejectionEvent")?;
    // WHATWG HTML §8.1.7.3.4 + WebIDL §3.10.23 dictionary coercion:
    //   - A missing second arg → arity error (dict with `required`
    //     member has no valid zero-arg form).
    //   - `null` / `undefined` → WebIDL treats as an empty
    //     dictionary; the subsequent `required promise` check
    //     reports "required member promise is undefined", matching
    //     Chrome.  Collapsing `null` into a coercion error would
    //     diverge from the spec.
    //   - Non-object primitive (number / string / bool) → WebIDL
    //     dictionary coercion error (`parameter 2 is not of type
    //     'PromiseRejectionEventInit'`).
    let init_arg = match args.get(1).copied() {
        Some(v) => v,
        None => {
            return Err(VmError::type_error(
                "Failed to construct 'PromiseRejectionEvent': \
                 2 arguments required, but only 1 present.",
            ));
        }
    };
    let opts_id = match init_arg {
        JsValue::Object(id) => Some(id),
        // `null` / `undefined` → WebIDL empty dict; fall through
        // so the required-member check drives the error text.
        JsValue::Null | JsValue::Undefined => None,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'PromiseRejectionEvent': \
                 parameter 2 is not of type 'PromiseRejectionEventInit'.",
            ));
        }
    };
    let base = parse_event_init(ctx, init_arg, "PromiseRejectionEvent")?;
    // `required Promise<any> promise` — absent key (empty-dict case
    // from null / undefined / `{}`) throws.  The check uses the raw
    // own-property lookup result: `undefined` (missing or explicit
    // `undefined`) is WebIDL-required-violating.  We don't validate
    // that the value is a Promise-shaped object (matches Chrome's
    // loose pass-through: any value is stored as-is).
    let k_promise = ctx.vm.well_known.promise;
    let k_reason = ctx.vm.well_known.reason;
    let promise_val = match opts_id {
        Some(id) => ctx
            .vm
            .get_property_value(id, PropertyKey::String(k_promise))?,
        None => JsValue::Undefined,
    };
    if matches!(promise_val, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to construct 'PromiseRejectionEvent': \
             required member promise is undefined.",
        ));
    }
    let reason_val = match opts_id {
        Some(id) => read_any(ctx, id, k_reason, JsValue::Undefined)?,
        None => JsValue::Undefined,
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .promise_rejection_event;
    // Root `promise` + `reason` across allocation (they may be Objects;
    // a GC during shape materialisation without these rooted would
    // leave them pointing at reclaimed slots).
    let mut g = ctx.vm.push_temp_root(promise_val);
    let mut g = g.push_temp_root(reason_val);
    let slots = vec![
        PropertyValue::Data(promise_val),
        PropertyValue::Data(reason_val),
    ];
    let proto = g
        .promise_rejection_event_prototype
        .expect("PromiseRejectionEvent.prototype must be registered before ctor");
    let id = g.build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// ErrorEvent (HTML §8.1.7.2)
// ---------------------------------------------------------------------------

fn native_error_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "ErrorEvent")?;
    let type_sid = type_arg(ctx, args, "ErrorEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "ErrorEvent")?;
    let opts_id = opts_object_id(init_arg);
    let (message_sid, filename_sid, lineno, colno, error_val) = if let Some(opts_id) = opts_id {
        let k_message = ctx.vm.well_known.message;
        let k_filename = ctx.vm.well_known.filename;
        let k_lineno = ctx.vm.well_known.lineno;
        let k_colno = ctx.vm.well_known.colno;
        let k_error = ctx.vm.well_known.error;
        let message_sid = read_string(ctx, opts_id, k_message)?;
        let filename_sid = read_string(ctx, opts_id, k_filename)?;
        // WebIDL `unsigned long` — ToUint32; default 0.
        let lineno_raw = read_number(ctx, opts_id, k_lineno, 0.0)?;
        let colno_raw = read_number(ctx, opts_id, k_colno, 0.0)?;
        let lineno = f64::from(super::super::coerce::f64_to_uint32(lineno_raw));
        let colno = f64::from(super::super::coerce::f64_to_uint32(colno_raw));
        // WebIDL `any error = null` — missing / undefined both collapse
        // to null per WHATWG default; explicit user value preserved.
        let error_val = read_any(ctx, opts_id, k_error, JsValue::Null)?;
        (message_sid, filename_sid, lineno, colno, error_val)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, empty, 0.0, 0.0, JsValue::Null)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .error_event;
    let slots = vec![
        PropertyValue::Data(JsValue::String(message_sid)),
        PropertyValue::Data(JsValue::String(filename_sid)),
        PropertyValue::Data(JsValue::Number(lineno)),
        PropertyValue::Data(JsValue::Number(colno)),
        PropertyValue::Data(error_val),
    ];
    let mut g = ctx.vm.push_temp_root(error_val);
    let proto = g
        .error_event_prototype
        .expect("ErrorEvent.prototype must be registered before ctor");
    let id = g.build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// HashChangeEvent (HTML §8.1.3)
// ---------------------------------------------------------------------------

fn native_hash_change_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "HashChangeEvent")?;
    let type_sid = type_arg(ctx, args, "HashChangeEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "HashChangeEvent")?;
    let opts_id = opts_object_id(init_arg);
    let (old_url_sid, new_url_sid) = if let Some(opts_id) = opts_id {
        let k_old = ctx.vm.well_known.old_url;
        let k_new = ctx.vm.well_known.new_url;
        let o = read_string(ctx, opts_id, k_old)?;
        let n = read_string(ctx, opts_id, k_new)?;
        (o, n)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, empty)
    };
    // Reuses the UA-dispatch `hash_change` shape — same key order
    // (oldURL, newURL) and both are DOMString → JsValue::String slots.
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .hash_change;
    let slots = vec![
        PropertyValue::Data(JsValue::String(old_url_sid)),
        PropertyValue::Data(JsValue::String(new_url_sid)),
    ];
    let proto = ctx
        .vm
        .hash_change_event_prototype
        .expect("HashChangeEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// PopStateEvent (HTML §8.8.1)
// ---------------------------------------------------------------------------

fn native_pop_state_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "PopStateEvent")?;
    let type_sid = type_arg(ctx, args, "PopStateEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "PopStateEvent")?;
    let opts_id = opts_object_id(init_arg);
    // `state: any = null` (WHATWG HTML §8.8.1.3): missing / undefined
    // both collapse to null (matches Chrome); user-supplied undefined
    // is also coerced to null for parity (not strict WebIDL `any`
    // pass-through, but observable-compatible).
    let state_val = if let Some(opts_id) = opts_id {
        read_any(ctx, opts_id, ctx.vm.well_known.state, JsValue::Null)?
    } else {
        JsValue::Null
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .pop_state_event;
    let slots = vec![PropertyValue::Data(state_val)];
    let mut g = ctx.vm.push_temp_root(state_val);
    let proto = g
        .pop_state_event_prototype
        .expect("PopStateEvent.prototype must be registered before ctor");
    let id = g.build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// AnimationEvent (CSS Animations Level 1 §4.2)
// ---------------------------------------------------------------------------

fn native_animation_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "AnimationEvent")?;
    let type_sid = type_arg(ctx, args, "AnimationEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "AnimationEvent")?;
    let opts_id = opts_object_id(init_arg);
    // Slot order matches `event_shapes.rs::Animation` payload installer:
    // animationName, elapsedTime, pseudoElement.
    let (animation_name_sid, elapsed_time, pseudo_element_sid) = if let Some(opts_id) = opts_id {
        let k_name = ctx.vm.well_known.animation_name;
        let k_elapsed = ctx.vm.well_known.elapsed_time;
        let k_pe = ctx.vm.well_known.pseudo_element;
        let n = read_string(ctx, opts_id, k_name)?;
        let t = read_number(ctx, opts_id, k_elapsed, 0.0)?;
        let pe = read_string(ctx, opts_id, k_pe)?;
        (n, t, pe)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, 0.0, empty)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .animation;
    let slots = vec![
        PropertyValue::Data(JsValue::String(animation_name_sid)),
        PropertyValue::Data(JsValue::Number(elapsed_time)),
        PropertyValue::Data(JsValue::String(pseudo_element_sid)),
    ];
    let proto = ctx
        .vm
        .animation_event_prototype
        .expect("AnimationEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// TransitionEvent (CSS Transitions Level 1 §6)
// ---------------------------------------------------------------------------

fn native_transition_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "TransitionEvent")?;
    let type_sid = type_arg(ctx, args, "TransitionEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "TransitionEvent")?;
    let opts_id = opts_object_id(init_arg);
    // Slot order matches `event_shapes.rs::Transition` payload installer:
    // propertyName, elapsedTime, pseudoElement.
    let (property_name_sid, elapsed_time, pseudo_element_sid) = if let Some(opts_id) = opts_id {
        let k_name = ctx.vm.well_known.property_name;
        let k_elapsed = ctx.vm.well_known.elapsed_time;
        let k_pe = ctx.vm.well_known.pseudo_element;
        let n = read_string(ctx, opts_id, k_name)?;
        let t = read_number(ctx, opts_id, k_elapsed, 0.0)?;
        let pe = read_string(ctx, opts_id, k_pe)?;
        (n, t, pe)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, 0.0, empty)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .transition;
    let slots = vec![
        PropertyValue::Data(JsValue::String(property_name_sid)),
        PropertyValue::Data(JsValue::Number(elapsed_time)),
        PropertyValue::Data(JsValue::String(pseudo_element_sid)),
    ];
    let proto = ctx
        .vm
        .transition_event_prototype
        .expect("TransitionEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// CloseEvent (WHATWG HTML §10.4 — paired with WebSocket / EventSource)
// ---------------------------------------------------------------------------

fn native_close_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "CloseEvent")?;
    let type_sid = type_arg(ctx, args, "CloseEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "CloseEvent")?;
    let opts_id = opts_object_id(init_arg);
    // Slot order matches `event_shapes.rs::CloseEvent` payload installer:
    // code, reason, wasClean.  WHATWG HTML §10.4 defaults: code=0 (no
    // status, ToUint16 of the IDL default), reason="", wasClean=false.
    let (code, reason_sid, was_clean) = if let Some(opts_id) = opts_id {
        let k_code = ctx.vm.well_known.code;
        let k_reason = ctx.vm.well_known.reason;
        let k_was_clean = ctx.vm.well_known.was_clean;
        let c = read_uint16(ctx, opts_id, k_code, 0)?;
        let r = read_string(ctx, opts_id, k_reason)?;
        let w = read_bool(ctx, opts_id, k_was_clean, false)?;
        (c, r, w)
    } else {
        let empty = ctx.vm.strings.intern("");
        (0, empty, false)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .close_event;
    let slots = vec![
        PropertyValue::Data(JsValue::Number(f64::from(code))),
        PropertyValue::Data(JsValue::String(reason_sid)),
        PropertyValue::Data(JsValue::Boolean(was_clean)),
    ];
    let proto = ctx
        .vm
        .close_event_prototype
        .expect("CloseEvent.prototype must be registered before ctor");
    let id = ctx
        .vm
        .build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    Ok(JsValue::Object(id))
}
