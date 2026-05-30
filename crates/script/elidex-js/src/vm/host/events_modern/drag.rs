//! `DragEvent` constructor + prototype (HTML DnD §6.4).
//!
//! `DragEvent.prototype` chains to `MouseEvent.prototype`.  The
//! constructor accepts a [`DragEventInit`] dictionary; instances
//! carry the 13 mouse-event own slots PLUS the `dataTransfer`
//! own slot (a `DataTransfer?` reference; the IDL coercion accepts
//! a DataTransfer brand OR null/undefined and throws TypeError on
//! any other Object value).

#![cfg(feature = "engine")]

use super::super::super::value::{
    JsValue, NativeContext, ObjectKind, PropertyKey, PropertyValue, VmError,
};
use super::super::super::VmInner;
use super::super::events::type_arg;
use super::super::events_ui::{
    opts_object_id, parse_mouse_event_members, parse_ui_event_init, register_descendant,
    MouseEventMembers,
};

pub(in crate::vm) fn register_drag_event_global(vm: &mut VmInner) {
    register_descendant(
        vm,
        vm.mouse_event_prototype,
        "DragEvent",
        native_drag_event_constructor,
        vm.well_known.drag_event_global,
        |vm, id| vm.drag_event_prototype = Some(id),
    );
}

fn native_drag_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let mode = ctx.mode;
    let type_sid = type_arg(ctx, args, "DragEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "DragEvent")?;
    let opts_id = opts_object_id(init_arg);
    let MouseEventMembers {
        related_target,
        mut slots,
    } = parse_mouse_event_members(ctx, opts_id)?;
    // `dataTransfer: DataTransfer? = null` — IDL coercion accepts
    // DataTransfer brand OR null/undefined; any other Object throws
    // TypeError per WebIDL §3.10.21 interface-type coercion.
    let dt_val = if let Some(opts_id) = opts_id {
        let k = ctx.vm.well_known.data_transfer;
        let raw = ctx.vm.get_property_value(opts_id, PropertyKey::String(k))?;
        coerce_data_transfer_nullable(ctx.vm, raw, "DragEvent", "dataTransfer")?
    } else {
        JsValue::Null
    };
    slots.push(PropertyValue::Data(dt_val));

    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .drag_event_constructed;
    let mut g0 = ctx.vm.push_temp_root(related_target);
    let mut g = g0.push_temp_root(dt_val);
    let drag_proto = g
        .drag_event_prototype
        .expect("DragEvent.prototype must be registered before ctor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, drag_proto, slots, mode);
    drop(g);
    drop(g0);
    Ok(JsValue::Object(id))
}

/// WebIDL `DataTransfer?` coercion.  Accepts:
/// - `undefined` / `null` → JS `null`
/// - DataTransfer-brand Object → pass through
/// - any other Object / primitive → TypeError
///
/// `interface` / `member` parameterise the WebIDL error message so
/// callers report the correct attribute name (DragEvent's
/// `dataTransfer` / ClipboardEvent's `clipboardData` /
/// InputEvent's `dataTransfer`) — Copilot R2 caught the hardcoded
/// `dataTransfer` leaking into ClipboardEvent's error text.
pub(in crate::vm) fn coerce_data_transfer_nullable(
    vm: &VmInner,
    val: JsValue,
    interface: &str,
    member: &str,
) -> Result<JsValue, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(JsValue::Null),
        JsValue::Object(id) => match vm.get_object(id).kind {
            ObjectKind::DataTransfer => Ok(val),
            _ => Err(VmError::type_error(format!(
                "Failed to construct '{interface}': \
                 member {member} is not of type 'DataTransfer'."
            ))),
        },
        _ => Err(VmError::type_error(format!(
            "Failed to construct '{interface}': \
             member {member} is not of type 'DataTransfer'."
        ))),
    }
}
