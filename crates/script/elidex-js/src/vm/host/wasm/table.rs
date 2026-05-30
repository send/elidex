//! `WebAssembly.Table` constructor + `.length` / `.get` / `.set` /
//! `.grow` accessors — slot `#11-wasm-vm` / D-16, plan-memo §5 Stage
//! 4.2.
//!
//! Per WASM JS API §5.4, `new WebAssembly.Table({element, initial,
//! maximum?}, value?)`:
//!
//! - `element`: WebIDL `TableKind` enum (`"anyfunc" | "externref"`)
//!   coerced to engine-indep [`WasmValueType::Ref`].
//! - `initial`: u32 entry count (WebIDL `[EnforceRange]`).
//! - `maximum`: optional u32 cap.
//! - `value`: initial entry value (defaults to typed-null per spec).
//!
//! `.length`, `.get(idx)`, `.set(idx, val)`, `.grow(delta, val?)` all
//! consume the engine-indep [`WasmTable`] surface from F1 +
//! [`WasmTable::element_kind`] from F2.  Element coerce per
//! [`WasmTablePayload::element_kind`] cached at ctor / wrap time so
//! per-call func-type walks are avoided (F1 F8 lesson).

use elidex_wasm_runtime::{
    HeapType, RefType, WasmRef, WasmRuntime, WasmTableDescriptor, WasmValue, WasmValueType,
};

use super::super::super::error::VmError;
use super::super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind};
use super::super::super::wasm_payload::WasmTablePayload;
use super::errors::wasm_error_to_vm_error;

/// `new WebAssembly.Table({element, initial, maximum?}, value?)` —
/// WASM JS API §5.4.
pub(super) fn native_wasm_table_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Table' on 'WebAssembly': Please use the 'new' operator",
        ));
    }
    let descriptor_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let initial_value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (element_kind, initial, maximum) = coerce_table_descriptor(ctx, descriptor_arg)?;

    let init_ref = js_value_to_wasm_ref(ctx, initial_value_arg, element_kind)?;
    let descriptor = WasmTableDescriptor {
        element: match element_kind {
            WasmValueType::Ref(r) => r,
            _ => {
                return Err(VmError::type_error(
                    "Failed to construct 'Table' on 'WebAssembly': element must be a reference type",
                ));
            }
        },
        initial,
        maximum,
    };
    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    let table = match WasmRuntime::new_table(&runtime, descriptor, init_ref) {
        Ok(t) => t,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };

    let proto = ctx
        .vm
        .wasm_table_prototype
        .expect("wasm_table_prototype populated in register_wasm_namespace");
    let receiver = ctx.vm.ensure_instance_or_alloc(this, Some(proto), ctx.mode);
    let JsValue::Object(id) = receiver else {
        unreachable!("ensure_instance_or_alloc returns an Object");
    };
    ctx.vm.get_object_mut(id).kind = ObjectKind::WasmTable;
    ctx.vm.wasm_table_storage.insert(
        id,
        WasmTablePayload {
            table,
            element_kind,
        },
    );
    Ok(receiver)
}

/// `Table.prototype.length` — WASM JS API §5.4 IDL.
pub(super) fn native_wasm_table_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_table_this(ctx, this, "length")?;
    let table = ctx
        .vm
        .wasm_table_storage
        .get(&id)
        .expect("brand-check guarantees storage entry exists")
        .table
        .clone();
    let len = match table.length() {
        Ok(l) => l,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    Ok(JsValue::Number(f64::from(len)))
}

/// `Table.prototype.get(idx)` — WASM JS API §5.4 IDL.
/// OOB returns `RangeError` per spec (`If index ≥ tableSize, throw
/// RangeError`).
pub(super) fn native_wasm_table_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_table_this(ctx, this, "get")?;
    let idx_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let idx = coerce_uint32(ctx, idx_arg)?;
    let table = ctx
        .vm
        .wasm_table_storage
        .get(&id)
        .expect("brand-check guarantees storage entry exists")
        .table
        .clone();
    let r = table.get(idx).ok_or_else(|| {
        VmError::range_error("WebAssembly.Table.prototype.get: index out of bounds")
    })?;
    Ok(wasm_ref_to_js(ctx, &r))
}

/// `Table.prototype.set(idx, value)` — WASM JS API §5.4 IDL.
pub(super) fn native_wasm_table_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_table_this(ctx, this, "set")?;
    let idx_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let idx = coerce_uint32(ctx, idx_arg)?;
    let (mut table, element_kind) = {
        let payload = ctx
            .vm
            .wasm_table_storage
            .get(&id)
            .expect("brand-check guarantees storage entry exists");
        (payload.table.clone(), payload.element_kind)
    };
    let value_ref = js_value_to_wasm_ref(ctx, value_arg, element_kind)?;
    if let Err(e) = table.set(idx, value_ref) {
        return Err(wasm_error_to_vm_error(ctx, &e));
    }
    Ok(JsValue::Undefined)
}

/// `Table.prototype.grow(delta, value?)` — WASM JS API §5.4 IDL.
/// Returns the previous size per spec.
pub(super) fn native_wasm_table_grow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_table_this(ctx, this, "grow")?;
    let delta_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let delta = coerce_uint32(ctx, delta_arg)?;
    let (mut table, element_kind) = {
        let payload = ctx
            .vm
            .wasm_table_storage
            .get(&id)
            .expect("brand-check guarantees storage entry exists");
        (payload.table.clone(), payload.element_kind)
    };
    let init_ref = js_value_to_wasm_ref(ctx, value_arg, element_kind)?;
    let prev = match table.grow(delta, init_ref) {
        Ok(p) => p,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    Ok(JsValue::Number(f64::from(prev)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_table_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "WebAssembly.Table.prototype.{method} called on non-Table"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmTable) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "WebAssembly.Table.prototype.{method} called on non-Table"
        )))
    }
}

/// Coerce the `TableDescriptor` JS dict into engine-indep parts per
/// WASM JS API §5.4 IDL.  Returns the element value type (synthesized
/// nullable Ref per `TableKind`), initial, and optional maximum.
fn coerce_table_descriptor(
    ctx: &mut NativeContext<'_>,
    descriptor_arg: JsValue,
) -> Result<(WasmValueType, u32, Option<u32>), VmError> {
    use super::super::super::value::PropertyKey;
    let JsValue::Object(dict) = descriptor_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'Table' on 'WebAssembly': parameter 1 must be a TableDescriptor",
        ));
    };
    let element_sid = ctx.vm.strings.intern("element");
    let initial_sid = ctx.vm.strings.intern("initial");
    let maximum_sid = ctx.vm.strings.intern("maximum");
    let element_val = ctx.get_property_value(dict, PropertyKey::String(element_sid))?;
    let element_str_sid = ctx.to_string_val(element_val)?;
    let element_str = ctx.vm.strings.get_utf8(element_str_sid);
    let heap = match element_str.as_str() {
        "anyfunc" => HeapType::Func,
        "externref" => HeapType::Extern,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'Table' on 'WebAssembly': element must be 'anyfunc' or 'externref'",
            ));
        }
    };
    let initial_val = ctx.get_property_value(dict, PropertyKey::String(initial_sid))?;
    let initial = coerce_uint32(ctx, initial_val)?;
    let maximum_val = ctx.get_property_value(dict, PropertyKey::String(maximum_sid))?;
    let maximum = if matches!(maximum_val, JsValue::Undefined) {
        None
    } else {
        Some(coerce_uint32(ctx, maximum_val)?)
    };
    Ok((
        WasmValueType::Ref(RefType {
            nullable: true,
            heap,
        }),
        initial,
        maximum,
    ))
}

/// Coerce a JS value to a [`WasmRef`] per the declared table element
/// kind.  Nullable null → typed-null; non-null funcref → resolves
/// from the exported-function wrapper side-store.  Stage 4 doesn't
/// yet support host-owned externref payloads — non-null externref
/// surfaces TypeError per F1 F13 (defer
/// `#11-wasm-externref-host-payload`).
fn js_value_to_wasm_ref(
    ctx: &NativeContext<'_>,
    val: JsValue,
    element_kind: WasmValueType,
) -> Result<WasmRef, VmError> {
    let WasmValueType::Ref(ref_ty) = element_kind else {
        return Err(VmError::type_error(
            "WebAssembly.Table operates only on reference element types",
        ));
    };
    if ref_ty.nullable && matches!(val, JsValue::Null | JsValue::Undefined) {
        return Ok(WasmRef::Null(ref_ty.heap));
    }
    match ref_ty.heap {
        HeapType::Func => {
            let JsValue::Object(id) = val else {
                return Err(VmError::type_error(
                    "WebAssembly.Table funcref value must be a wasm exported function or null",
                ));
            };
            let payload = ctx.vm.wasm_exported_func_storage.get(&id).ok_or_else(|| {
                VmError::type_error(
                    "WebAssembly.Table funcref value must be a wasm exported function",
                )
            })?;
            Ok(WasmRef::Func(payload.func.clone()))
        }
        HeapType::Extern => Err(VmError::type_error(
            "WebAssembly.Table non-null externref values are not yet supported from JS",
        )),
        _ => Err(VmError::type_error(
            "WebAssembly.Table future-proposal reference types are not yet supported from JS",
        )),
    }
}

/// Reverse coerce a [`WasmRef`] back to a JS value.  Same Stage 3
/// Routes through the shared [`super::exported_func::wasm_value_to_js`]
/// SoT (typed-null → JS null, non-null funcref → reverse-lookup
/// against `wasm_exported_func_storage`).
fn wasm_ref_to_js(ctx: &mut NativeContext<'_>, r: &WasmRef) -> JsValue {
    super::exported_func::wasm_value_to_js(ctx, &WasmValue::Ref(r.clone()))
}

/// Coerce a JS value to a u32 per WebIDL `[EnforceRange]` u32 —
/// rejects NaN/Infinity (TypeError) + out-of-range (TypeError per
/// WebIDL §3.2.5 — RangeError is reserved for in-domain bounds,
/// not for the `[EnforceRange]` non-finite / fractional-domain
/// rejection).  Shared with sibling `memory.rs` / `global.rs` via
/// the `pub(super)` visibility.
pub(super) fn coerce_uint32(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<u32, VmError> {
    let n = ctx.to_number(val)?;
    if !n.is_finite() {
        return Err(VmError::type_error(
            "WebAssembly numeric parameter must be a finite number",
        ));
    }
    let truncated = n.trunc();
    if !(0.0..=f64::from(u32::MAX)).contains(&truncated) {
        return Err(VmError::type_error(
            "WebAssembly numeric parameter is out of u32 range",
        ));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(truncated as u32)
}
