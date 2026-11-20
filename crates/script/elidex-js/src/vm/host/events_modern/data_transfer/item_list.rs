//! `DataTransferItemList` constructor + prototype (HTML DnD §6.2 / §6.3).

use crate::vm::shape;
use crate::vm::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use crate::vm::wrapper_intern::{WrapperKey, WrapperKind, WrapperOwner, WrapperSubkey};
use crate::vm::VmInner;

use super::super::DataTransferEntry;
use super::dt_state_mut;
use crate::vm::host::events::install_ctor;

pub(in crate::vm) fn register_data_transfer_item_list_global(vm: &mut VmInner) {
    let parent = vm
        .object_prototype
        .expect("register_data_transfer_item_list_global requires object_prototype");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    vm.data_transfer_item_list_prototype = Some(proto_id);
    install_ctor(
        vm,
        proto_id,
        "DataTransferItemList",
        crate::vm::value::native_illegal_constructor_unreachable,
        vm.well_known.data_transfer_item_list_global,
        crate::vm::value::CallShape::IllegalConstructor,
    );
    let k_length = vm.well_known.length;
    let k_add = vm.well_known.add;
    let k_remove = vm.well_known.remove;
    let k_clear = vm.well_known.clear_method;
    vm.install_accessor_pair(
        proto_id,
        k_length,
        native_dt_item_list_get_length,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_native_method(
        proto_id,
        k_add,
        native_dt_item_list_add,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_remove,
        native_dt_item_list_remove,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_clear,
        native_dt_item_list_clear,
        shape::PropertyAttrs::METHOD,
    );
}

// ---------------------------------------------------------------------------
// DataTransferItemList
// ---------------------------------------------------------------------------

fn require_dt_item_list_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
    op_kind: &str,
) -> Result<ObjectId, VmError> {
    match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItemList { parent_dt_id } => Ok(parent_dt_id),
            _ => Err(VmError::type_error(format!(
                "Failed to {op_kind} the '{member}' property from \
                 'DataTransferItemList': Illegal invocation."
            ))),
        },
        _ => Err(VmError::type_error(format!(
            "Failed to {op_kind} the '{member}' property from \
             'DataTransferItemList': Illegal invocation."
        ))),
    }
}

fn native_dt_item_list_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parent_dt_id = require_dt_item_list_receiver(ctx, this, "length", "read")?;
    let len: u32 = ctx
        .vm
        .data_transfer_states
        .get(&parent_dt_id)
        .map_or(0, |s| u32::try_from(s.items.len()).unwrap_or(u32::MAX));
    Ok(JsValue::Number(f64::from(len)))
}

fn native_dt_item_list_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parent_dt_id = match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItemList { parent_dt_id } => parent_dt_id,
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'add' on 'DataTransferItemList': \
                     Illegal invocation.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'add' on 'DataTransferItemList': \
                 Illegal invocation.",
            ));
        }
    };
    let data_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // `add(File)` overload — WebIDL overload resolution picks this
    // path when the first arg is a File brand.  Blob is a sibling
    // (per File API §11.1), NOT a subclass at the IDL overload-
    // resolution level — but in our impl File IS-A Blob via
    // prototype chain.  For overload resolution we accept ONLY
    // ObjectKind::File here (Blob and other Objects fall through to
    // the (DOMString, DOMString) overload via ToString below).
    //
    // D-14 `#11-file-api` Phase 5 — slot `#11-data-transfer-file-paired`
    // resolved.
    if let JsValue::Object(file_obj_id) = data_arg {
        if matches!(ctx.vm.get_object(file_obj_id).kind, ObjectKind::File) {
            let type_sid = crate::vm::host::blob::blob_type(ctx.vm, file_obj_id);
            let new_index = {
                let state = dt_state_mut(ctx.vm, parent_dt_id);
                let i = state.items.len() as u32;
                state.items.push(DataTransferEntry::File {
                    file_id: file_obj_id,
                    type_sid,
                });
                state.file_entries.push(file_obj_id);
                // Invalidate cached files_wrapper so the next
                // `.files` read sees the updated entry list.
                state.files_wrapper = None;
                i
            };
            let item_wrapper = item_wrapper_for(ctx.vm, parent_dt_id, new_index);
            return Ok(JsValue::Object(item_wrapper));
        }
    }
    let type_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    if matches!(type_arg, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to execute 'add' on 'DataTransferItemList': \
             2 arguments required.",
        ));
    }
    let data_sid = crate::vm::coerce::to_string(ctx.vm, data_arg)?;
    let type_sid = crate::vm::coerce::to_string(ctx.vm, type_arg)?;
    let new_index = {
        let state = dt_state_mut(ctx.vm, parent_dt_id);
        let i = state.items.len() as u32;
        state.items.push(DataTransferEntry::String {
            format: type_sid,
            data: data_sid,
        });
        i
    };
    // Return the newly-added DataTransferItem wrapper.
    let item_wrapper = item_wrapper_for(ctx.vm, parent_dt_id, new_index);
    Ok(JsValue::Object(item_wrapper))
}

fn native_dt_item_list_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parent_dt_id = match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItemList { parent_dt_id } => parent_dt_id,
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'remove' on 'DataTransferItemList': \
                     Illegal invocation.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'remove' on 'DataTransferItemList': \
                 Illegal invocation.",
            ));
        }
    };
    let idx_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let idx_num = crate::vm::coerce::to_number(ctx.vm, idx_arg)?;
    let idx = crate::vm::coerce::f64_to_uint32(idx_num) as usize;
    let state = dt_state_mut(ctx.vm, parent_dt_id);
    if idx < state.items.len() {
        let removed = state.items.remove(idx);
        // D-14 `#11-file-api` Phase 5 — keep `file_entries` and the
        // `[SameObject]` files_wrapper in sync if the removed entry
        // was a File.  Drop the FIRST matching `file_id` only — the
        // same File may legitimately appear multiple times (HTML §6.2
        // `items.add(f); items.add(f)` is a valid no-dedup append), so
        // a `retain(|id| id != file_id)` would wrongly evict every
        // sibling copy when only one items entry was removed.
        if let DataTransferEntry::File { file_id, .. } = removed {
            if let Some(pos) = state.file_entries.iter().position(|&id| id == file_id) {
                state.file_entries.remove(pos);
            }
            state.files_wrapper = None;
        }
        // Invalidate downstream identity-cache entries for indices
        // ≥ idx because the index→entry mapping shifts.
        invalidate_item_wrapper_cache_from(ctx.vm, parent_dt_id, idx as u32);
    }
    Ok(JsValue::Undefined)
}

fn native_dt_item_list_clear(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parent_dt_id = match this {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::DataTransferItemList { parent_dt_id } => parent_dt_id,
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'clear' on 'DataTransferItemList': \
                     Illegal invocation.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'clear' on 'DataTransferItemList': \
                 Illegal invocation.",
            ));
        }
    };
    let state = dt_state_mut(ctx.vm, parent_dt_id);
    state.items.clear();
    // D-14 `#11-file-api` Phase 5 — clear file_entries + invalidate
    // [SameObject] files_wrapper in lockstep with items.clear.
    state.file_entries.clear();
    state.files_wrapper = None;
    invalidate_item_wrapper_cache_from(ctx.vm, parent_dt_id, 0);
    Ok(JsValue::Undefined)
}

/// Materialise or look up an identity-cached
/// [`ObjectKind::DataTransferItem`] wrapper for `(parent_dt_id, index)`.
fn item_wrapper_for(vm: &mut VmInner, parent_dt_id: ObjectId, index: u32) -> ObjectId {
    vm.intern_wrapper(
        WrapperKey::object_indexed(parent_dt_id, WrapperKind::DataTransferItem, index),
        |vm| {
            let proto = vm
                .data_transfer_item_prototype
                .expect("DataTransferItem.prototype must be registered before item_wrapper_for");
            vm.alloc_object(Object {
                kind: ObjectKind::DataTransferItem {
                    parent_dt_id,
                    index,
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: Some(proto),
                extensible: true,
            })
        },
    )
}

/// Drop cached DataTransferItem wrappers at or above `from_index`
/// for the given parent — used after list mutations that shift
/// indices.  Cache entries below `from_index` survive because
/// their index→entry mapping is unaffected.
///
/// Post-seam this is a whole-`wrapper_store` `retain` rather than a
/// scan of a dedicated DataTransferItem map — the same bulk-op idiom
/// the GC sweep (`gc/collect.rs`) and `Vm::unbind` retains use. The
/// scan is bounded by total interned wrappers, but DataTransfer item
/// lists are tiny and these index-shifting mutations (`remove` /
/// `clearData`) are rare cold-path drag-and-drop operations, so the
/// O(store) pass is an acceptable trade for one unified store.
pub(super) fn invalidate_item_wrapper_cache_from(
    vm: &mut VmInner,
    parent_dt_id: ObjectId,
    from_index: u32,
) {
    if let Some(hd) = vm.host_data.as_deref_mut() {
        hd.wrapper_store.retain(|key, _| {
            !(key.kind == WrapperKind::DataTransferItem
                && key.owner == WrapperOwner::Object(parent_dt_id)
                && matches!(key.subkey, WrapperSubkey::Index(idx) if idx >= from_index))
        });
    }
}
