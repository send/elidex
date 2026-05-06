//! `StorageEvent` interface (WHATWG HTML §11.4.2) — VM thin binding.
//!
//! ## Shape
//!
//! ```text
//! StorageEvent instance (ObjectKind::StorageEvent)
//!   key / oldValue / newValue / url / storageArea  (own data, WEBIDL_RO)
//!   → StorageEvent.prototype
//!     → Event.prototype
//!       → Object.prototype
//! ```
//!
//! All five IDL attributes are own-data props on the
//! `precomputed_event_shapes.storage` shape and read directly without
//! a side table.
//!
//! ## Dispatch (out of scope for this slot)
//!
//! Per WHATWG §11.2.5 step 8, a `setItem` / `removeItem` / `clear`
//! mutation fires a `storage` event on **other** Documents in the
//! same origin — never on the originating document.  In a
//! single-VM world the event is unobservable from the originating
//! script; cross-VM fan-out via the shell broker is tracked at
//! `#11-storage-event-broker`.  This file ships only the
//! constructor + class definition; the dispatch path is left absent.

#![cfg(feature = "engine")]

use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyValue, StringId, VmError,
};
use super::super::VmInner;

use super::events::{check_construct, parse_event_init, type_arg};
use super::events_extras::register_event_subclass;

impl VmInner {
    /// Allocate `StorageEvent.prototype` chained to `Event.prototype`,
    /// install the constructor on `globalThis`.  Five readonly IDL
    /// attributes (key / oldValue / newValue / url / storageArea) are
    /// shape-resident on instances — no prototype accessors.
    ///
    /// Called from `register_globals()` after `register_event_global`.
    pub(in crate::vm) fn register_storage_event_global(&mut self) {
        register_event_subclass(
            self,
            "StorageEvent",
            native_storage_event_constructor,
            self.well_known.storage_event_global,
            |vm, proto_id| vm.storage_event_prototype = Some(proto_id),
        );
    }
}

/// Read a string init-dict member.  WHATWG `DOMString?` semantics:
/// missing / `undefined` → `null`; any other value coerces via
/// ToString.
fn read_string_or_null(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
    key: StringId,
) -> Result<JsValue, VmError> {
    let Some(opts_id) = opts_id else {
        return Ok(JsValue::Null);
    };
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined | JsValue::Null => Ok(JsValue::Null),
        _ => {
            let sid = super::super::coerce::to_string(ctx.vm, v)?;
            Ok(JsValue::String(sid))
        }
    }
}

/// Read the non-nullable `url` member — defaults to `""` when missing.
fn read_url(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
    key: StringId,
) -> Result<JsValue, VmError> {
    let Some(opts_id) = opts_id else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    if matches!(v, JsValue::Undefined) {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = super::super::coerce::to_string(ctx.vm, v)?;
    Ok(JsValue::String(sid))
}

/// Read the `storageArea` member — `null` default; non-null values
/// must be `Storage`-branded (`null` allowed for "not associated";
/// any other Object value is preserved verbatim per the WebIDL
/// `Storage?` nullable type).  Phase 2: pass-through with no brand
/// validation — Chrome accepts arbitrary Object values here too.
fn read_storage_area(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
    key: StringId,
) -> Result<JsValue, VmError> {
    let Some(opts_id) = opts_id else {
        return Ok(JsValue::Null);
    };
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    Ok(match v {
        JsValue::Undefined => JsValue::Null,
        _ => v,
    })
}

fn native_storage_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "StorageEvent")?;
    // Pre-`install_host_data` reachability: every `host()` access is
    // gated by `host_if_bound` further down.  The constructor itself
    // does not touch HostData (no DOM wrappers, no per-VM side table),
    // so it works pre-init the same way `new Event(...)` does.
    let type_sid = type_arg(ctx, args, "StorageEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "StorageEvent")?;
    let opts_id = match init_arg {
        JsValue::Object(id) => Some(id),
        _ => None,
    };
    let key_val = read_string_or_null(ctx, opts_id, ctx.vm.well_known.key)?;
    let old_val = read_string_or_null(ctx, opts_id, ctx.vm.well_known.old_value)?;
    let new_val = read_string_or_null(ctx, opts_id, ctx.vm.well_known.new_value)?;
    let url_val = read_url(ctx, opts_id, ctx.vm.well_known.url)?;
    let storage_area_val = read_storage_area(ctx, opts_id, ctx.vm.well_known.storage_area)?;

    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .storage;
    // Slot order matches `event_shapes.rs::storage`: key, oldValue,
    // newValue, url, storageArea.
    let slots = vec![
        PropertyValue::Data(key_val),
        PropertyValue::Data(old_val),
        PropertyValue::Data(new_val),
        PropertyValue::Data(url_val),
        PropertyValue::Data(storage_area_val),
    ];
    // Root all 5 attribute values across the (possibly GC-triggering)
    // shape allocation so a GC during `create_fresh_event_object` does
    // not collect the just-coerced JS strings before they reach the
    // shape slots.
    let mut g0 = ctx.vm.push_temp_root(key_val);
    let mut g1 = g0.push_temp_root(old_val);
    let mut g2 = g1.push_temp_root(new_val);
    let mut g3 = g2.push_temp_root(url_val);
    let mut g = g3.push_temp_root(storage_area_val);
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, slots, false);
    g.get_object_mut(id).kind = ObjectKind::StorageEvent;
    drop(g);
    drop(g3);
    drop(g2);
    drop(g1);
    drop(g0);
    Ok(JsValue::Object(id))
}
