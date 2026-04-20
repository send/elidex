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
//! Prototype is then overridden to the per-ctor descendant so the
//! chain ends at its specific `.prototype`, not `Event.prototype`.

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

    /// Finalise a freshly-built Event from `create_fresh_event_object`
    /// by overriding its prototype to a specific descendant (per-ctor)
    /// and return the object id.  Shared across all four C4 ctors.
    fn build_event_subclass_instance(
        &mut self,
        this: JsValue,
        type_sid: StringId,
        base: EventInit,
        shape_id: ShapeId,
        descendant_proto: Option<ObjectId>,
        payload_slots: Vec<PropertyValue>,
    ) -> ObjectId {
        let id =
            self.create_fresh_event_object(this, type_sid, base, shape_id, payload_slots, false);
        if let Some(proto) = descendant_proto {
            self.get_object_mut(id).prototype = Some(proto);
        }
        id
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
    // WHATWG HTML §8.1.7.3.4: `eventInitDict` is required (the dict
    // itself has `required Promise<any> promise`).  A missing second
    // argument therefore fails before we even parse the dict.
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "PromiseRejectionEvent")?;
    let opts_id = match init_arg {
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'PromiseRejectionEvent': \
                 2 arguments required, but only 1 present.",
            ))
        }
    };
    // `required Promise<any> promise` — absent key throws.  The check
    // uses the raw own-property lookup result: `undefined` (missing or
    // explicit `undefined`) is WebIDL-required-violating.  We don't
    // validate that the value is a Promise-shaped object (matches
    // Chrome's loose pass-through: any value is stored as-is).
    let k_promise = ctx.vm.well_known.promise;
    let k_reason = ctx.vm.well_known.reason;
    let promise_val = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(k_promise))?;
    if matches!(promise_val, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to construct 'PromiseRejectionEvent': \
             required member promise is undefined.",
        ));
    }
    let reason_val = read_any(ctx, opts_id, k_reason, JsValue::Undefined)?;
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
    let proto = g.promise_rejection_event_prototype;
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
    let proto = g.error_event_prototype;
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
    let proto = ctx.vm.hash_change_event_prototype;
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
    let proto = g.pop_state_event_prototype;
    let id = g.build_event_subclass_instance(this, type_sid, base, shape_id, proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}
