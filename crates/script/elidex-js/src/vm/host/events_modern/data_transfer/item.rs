//! The `DataTransferItem` interface (HTML §6.11.3.2): registration and
//! prototype natives.

use super::super::super::super::shape;
use super::super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::super::VmInner;

use super::super::super::events::install_ctor;
use super::super::DataTransferEntry;

pub(in crate::vm) fn register_data_transfer_item_global(vm: &mut VmInner) {
    let parent = vm
        .object_prototype
        .expect("register_data_transfer_item_global requires object_prototype");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    vm.data_transfer_item_prototype = Some(proto_id);
    install_ctor(
        vm,
        proto_id,
        "DataTransferItem",
        super::super::super::super::value::native_illegal_constructor_unreachable,
        vm.well_known.data_transfer_item_global,
        super::super::super::super::value::CallShape::IllegalConstructor,
    );
    // `kind` / `type` readonly accessors + `getAsString` /
    // `getAsFile` methods.  StringId fields are Copy — snapshot
    // them first so the install calls can take `&mut vm` without
    // holding an outstanding borrow of `well_known`.
    let k_kind = vm.well_known.kind;
    let k_type = vm.well_known.event_type;
    let k_get_as_string = vm.well_known.get_as_string;
    let k_get_as_file = vm.well_known.get_as_file;
    vm.install_accessor_pair(
        proto_id,
        k_kind,
        native_dt_item_get_kind,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        k_type,
        native_dt_item_get_type,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_native_method(
        proto_id,
        k_get_as_string,
        native_dt_item_get_as_string,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_get_as_file,
        native_dt_item_get_as_file,
        shape::PropertyAttrs::METHOD,
    );
}

fn require_dt_item_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
    op_kind: &str,
) -> Result<(ObjectId, u32), VmError> {
    match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItem {
                parent_dt_id,
                index,
            } => Ok((parent_dt_id, index)),
            _ => Err(VmError::type_error(format!(
                "Failed to {op_kind} the '{member}' property from \
                 'DataTransferItem': Illegal invocation."
            ))),
        },
        _ => Err(VmError::type_error(format!(
            "Failed to {op_kind} the '{member}' property from \
             'DataTransferItem': Illegal invocation."
        ))),
    }
}

fn native_dt_item_get_kind(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (parent_dt_id, index) = require_dt_item_receiver(ctx, this, "kind", "read")?;
    let entry = ctx
        .vm
        .data_transfer_states
        .get(&parent_dt_id)
        .and_then(|s| s.items.get(index as usize));
    let sid = match entry {
        Some(DataTransferEntry::String { .. }) => ctx.vm.well_known.kind_string,
        Some(DataTransferEntry::File { .. }) => ctx.vm.well_known.kind_file,
        None => ctx.vm.well_known.empty,
    };
    Ok(JsValue::String(sid))
}

fn native_dt_item_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (parent_dt_id, index) = require_dt_item_receiver(ctx, this, "type", "read")?;
    let entry = ctx
        .vm
        .data_transfer_states
        .get(&parent_dt_id)
        .and_then(|s| s.items.get(index as usize));
    let sid = match entry {
        Some(DataTransferEntry::String { format, .. }) => *format,
        Some(DataTransferEntry::File { type_sid, .. }) => *type_sid,
        None => ctx.vm.well_known.empty,
    };
    Ok(JsValue::String(sid))
}

/// `DataTransferItem.prototype.getAsString(callback)` (HTML §6.11.3.2).
/// Enqueues a microtask invoking the callback with the stored
/// string; null callback → no-op.  Per-spec semantics.
fn native_dt_item_get_as_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (parent_dt_id, index) = match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItem {
                parent_dt_id,
                index,
            } => (parent_dt_id, index),
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'getAsString' on 'DataTransferItem': \
                     Illegal invocation.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getAsString' on 'DataTransferItem': \
                 Illegal invocation.",
            ));
        }
    };
    let cb_arg = args.first().copied().unwrap_or(JsValue::Null);
    // null / undefined callback → no-op per WebIDL nullable
    // callback handling.
    let cb_id = match cb_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getAsString' on 'DataTransferItem': \
                 parameter 1 is not a function.",
            ));
        }
    };
    // Resolve string entry (File entries: no-op per spec —
    // "kind is not text", returns immediately).
    let entry = ctx
        .vm
        .data_transfer_states
        .get(&parent_dt_id)
        .and_then(|s| s.items.get(index as usize));
    let data_sid = match entry {
        Some(DataTransferEntry::String { data, .. }) => *data,
        _ => return Ok(JsValue::Undefined),
    };
    // Enqueue microtask via a one-shot BoundFunction wrapper.  The
    // microtask path (`Microtask::Callback { func }`) invokes the
    // function with `this=undefined` and no args; the BoundFunction
    // wrapper prepends `bound_args` so the user callback observes
    // its data string argument as expected.
    let bound = ctx.vm.alloc_object(Object {
        kind: ObjectKind::BoundFunction {
            target: cb_id,
            bound_this: JsValue::Undefined,
            bound_args: vec![JsValue::String(data_sid)],
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: ctx.vm.function_prototype,
        extensible: true,
    });
    ctx.vm.microtask_queue.push_back(
        super::super::super::super::natives_promise::Microtask::Callback { func: bound },
    );
    Ok(JsValue::Undefined)
}

/// `DataTransferItem.prototype.getAsFile()` (HTML §6.11.3.2).
///
/// D-14 `#11-file-api` Phase 5: returns the File wrapper for File-kind
/// entries; returns `null` for String-kind entries (per spec) or out-
/// of-range indices.  Slot `#11-data-transfer-file-paired` resolved.
fn native_dt_item_get_as_file(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (parent_dt_id, index) = match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItem {
                parent_dt_id,
                index,
            } => (parent_dt_id, index),
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'getAsFile' on 'DataTransferItem': \
                     Illegal invocation.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getAsFile' on 'DataTransferItem': \
                 Illegal invocation.",
            ));
        }
    };
    let file_id = ctx
        .vm
        .data_transfer_states
        .get(&parent_dt_id)
        .and_then(|s| s.items.get(index as usize))
        .and_then(|entry| match entry {
            DataTransferEntry::File { file_id, .. } => Some(*file_id),
            DataTransferEntry::String { .. } => None,
        });
    Ok(file_id.map_or(JsValue::Null, JsValue::Object))
}
