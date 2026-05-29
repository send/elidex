//! `WebAssembly.Global` constructor + `.value` getter/setter +
//! `.valueOf` — slot `#11-wasm-vm` / D-16, plan-memo §5 Stage 4.3.
//!
//! Per WASM JS API §5.5, `new WebAssembly.Global({value, mutable},
//! v?)`:
//!
//! - `value`: WebIDL `ValueType` enum (`"i32" | "i64" | "f32" | "f64"
//!   | "anyfunc" | "externref" | "v128"`) coerced to engine-indep
//!   [`WasmValueType`].  Setter step 4 rejects v128/exnref with
//!   TypeError per F1 F13.
//! - `mutable`: bool flag — setter step 5 rejects writes to
//!   immutable globals with TypeError.
//! - `v`: initial value (defaults per spec — i32/i64 → 0, f32/f64 →
//!   +0, ref → typed-null).
//!
//! `.value` accessor pair reads/writes via [`WasmGlobal::get`] /
//! [`WasmGlobal::set`]; `.valueOf()` is an alias of `.value` getter
//! per IDL `[[ToPrimitive]]` impl convention.

use elidex_wasm_runtime::{
    HeapType, RefType, WasmGlobalDescriptor, WasmRef, WasmRuntime, WasmValue, WasmValueType,
};

use super::super::super::error::VmError;
use super::super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind};
use super::super::super::wasm_payload::WasmGlobalPayload;
use super::errors::wasm_error_to_vm_error;

/// `new WebAssembly.Global({value, mutable}, v?)` — WASM JS API §5.5.
pub(super) fn native_wasm_global_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let descriptor_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let init_value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (value_type, mutable) = coerce_global_descriptor(ctx, descriptor_arg)?;

    // §5.5 setter step 4 — reject v128 at JS boundary.  exnref handled
    // by the typed-null surface (HeapType::Exn not yet supported).
    if matches!(value_type, WasmValueType::V128) {
        return Err(VmError::type_error(
            "Failed to construct 'Global' on 'WebAssembly': v128 globals cannot be created from JS",
        ));
    }

    let init = js_value_to_wasm_value(
        ctx,
        init_value_arg,
        value_type,
        "Failed to construct 'Global' on 'WebAssembly'",
    )?;
    let descriptor = WasmGlobalDescriptor {
        value_type,
        mutable,
    };
    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    let global = match WasmRuntime::new_global(&runtime, descriptor, init) {
        Ok(g) => g,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };

    let proto = ctx
        .vm
        .wasm_global_prototype
        .expect("wasm_global_prototype populated in register_wasm_namespace");
    let receiver = ctx.vm.ensure_instance_or_alloc(this, Some(proto), ctx.mode);
    let JsValue::Object(id) = receiver else {
        unreachable!("ensure_instance_or_alloc returns an Object");
    };
    ctx.vm.get_object_mut(id).kind = ObjectKind::WasmGlobal;
    ctx.vm
        .wasm_global_storage
        .insert(id, WasmGlobalPayload { global });
    Ok(receiver)
}

/// `Global.prototype.value` getter — WASM JS API §5.5 IDL.
pub(super) fn native_wasm_global_value_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_global_this(ctx, this, "value")?;
    let global = ctx
        .vm
        .wasm_global_storage
        .get(&id)
        .expect("brand-check guarantees storage entry exists")
        .global
        .clone();
    let v = global.get();
    Ok(wasm_value_to_js(&v))
}

/// `Global.prototype.value` setter — WASM JS API §5.5 IDL.
/// - Step 4: v128/exnref → TypeError.
/// - Step 5: immutable → TypeError (surfaced via `WasmGlobal::set`
///   returning `WasmError::Runtime`, marshalled here).
pub(super) fn native_wasm_global_value_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_global_this(ctx, this, "value")?;
    let value_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let (mut global, value_type, mutable) = {
        let payload = ctx
            .vm
            .wasm_global_storage
            .get(&id)
            .expect("brand-check guarantees storage entry exists");
        let g = payload.global.clone();
        let vt = match g.value_type() {
            Ok(t) => t,
            Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
        };
        let m = g.mutable();
        (g, vt, m)
    };
    // §5.5 setter step 5 — immutable globals reject writes with
    // TypeError per spec; surface eagerly with the JS shape rather
    // than waiting for the engine-bridge `Runtime` error to round-trip.
    if !mutable {
        return Err(VmError::type_error(
            "Cannot assign to value of immutable WebAssembly.Global",
        ));
    }
    let v = js_value_to_wasm_value(
        ctx,
        value_arg,
        value_type,
        "WebAssembly.Global.prototype.value setter",
    )?;
    if let Err(e) = global.set(v) {
        return Err(wasm_error_to_vm_error(ctx, &e));
    }
    Ok(JsValue::Undefined)
}

/// `Global.prototype.valueOf()` — WASM JS API §5.5 IDL alias of the
/// `.value` getter (per `[[ToPrimitive]]` impl convention so
/// `Number(global)` / `global + 1` work for i32 / i64 / f32 / f64
/// globals).
pub(super) fn native_wasm_global_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    native_wasm_global_value_get(ctx, this, args)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_global_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "WebAssembly.Global.prototype.{method} called on non-Global"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmGlobal) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "WebAssembly.Global.prototype.{method} called on non-Global"
        )))
    }
}

/// Coerce the `GlobalDescriptor` JS dict into engine-indep parts per
/// WASM JS API §5.5 IDL.  Returns the value type and mutable flag.
fn coerce_global_descriptor(
    ctx: &mut NativeContext<'_>,
    descriptor_arg: JsValue,
) -> Result<(WasmValueType, bool), VmError> {
    use super::super::super::value::PropertyKey;
    let JsValue::Object(dict) = descriptor_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'Global' on 'WebAssembly': parameter 1 must be a GlobalDescriptor",
        ));
    };
    let value_sid = ctx.vm.strings.intern("value");
    let mutable_sid = ctx.vm.strings.intern("mutable");
    let value_val = ctx.get_property_value(dict, PropertyKey::String(value_sid))?;
    let value_str_sid = ctx.to_string_val(value_val)?;
    let value_str = ctx.vm.strings.get_utf8(value_str_sid);
    let value_type = match value_str.as_str() {
        "i32" => WasmValueType::I32,
        "i64" => WasmValueType::I64,
        "f32" => WasmValueType::F32,
        "f64" => WasmValueType::F64,
        "v128" => WasmValueType::V128,
        "anyfunc" => WasmValueType::Ref(RefType {
            nullable: true,
            heap: HeapType::Func,
        }),
        "externref" => WasmValueType::Ref(RefType {
            nullable: true,
            heap: HeapType::Extern,
        }),
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'Global' on 'WebAssembly': value must be a valid WebAssembly value type",
            ));
        }
    };
    let mutable_val = ctx.get_property_value(dict, PropertyKey::String(mutable_sid))?;
    let mutable = ctx.to_boolean(mutable_val);
    Ok((value_type, mutable))
}

/// Coerce a JS value to a [`WasmValue`] per the declared value type.
/// Mirrors the exported-function arg coerce in
/// [`super::exported_func`].  `undefined` at construction time
/// defaults per WASM JS API §5.5 (i32/i64 → 0, f32/f64 → +0,
/// ref → typed-null).
fn js_value_to_wasm_value(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    expected: WasmValueType,
    error_prefix: &'static str,
) -> Result<WasmValue, VmError> {
    // §5.5 step 6 — default initializer per value type when `v` is
    // omitted (undefined).
    if matches!(val, JsValue::Undefined) {
        return Ok(default_value_for(expected));
    }
    match expected {
        WasmValueType::I32 => {
            let n = ctx.to_number(val)?;
            #[allow(clippy::cast_possible_truncation)]
            let i = n as i64 as i32;
            Ok(WasmValue::I32(i))
        }
        WasmValueType::I64 => {
            #[allow(clippy::cast_possible_truncation)]
            let n = ctx.to_number(val)? as i64;
            Ok(WasmValue::I64(n))
        }
        WasmValueType::F32 => {
            let n = ctx.to_number(val)?;
            #[allow(clippy::cast_possible_truncation)]
            let f = n as f32;
            Ok(WasmValue::F32(f))
        }
        WasmValueType::F64 => Ok(WasmValue::F64(ctx.to_number(val)?)),
        WasmValueType::V128 => Err(VmError::type_error(format!(
            "{error_prefix}: v128 values cannot be passed from JS"
        ))),
        WasmValueType::Ref(ref_ty) => {
            if ref_ty.nullable && matches!(val, JsValue::Null | JsValue::Undefined) {
                return Ok(WasmValue::Ref(WasmRef::Null(ref_ty.heap)));
            }
            match ref_ty.heap {
                HeapType::Func => {
                    let JsValue::Object(id) = val else {
                        return Err(VmError::type_error(format!(
                            "{error_prefix}: funcref value must be a wasm exported function or null"
                        )));
                    };
                    let payload = ctx.vm.wasm_exported_func_storage.get(&id).ok_or_else(|| {
                        VmError::type_error(format!(
                            "{error_prefix}: funcref value must be a wasm exported function"
                        ))
                    })?;
                    Ok(WasmValue::Ref(WasmRef::Func(payload.func.clone())))
                }
                HeapType::Extern => Err(VmError::type_error(format!(
                    "{error_prefix}: non-null externref values are not yet supported from JS"
                ))),
                _ => Err(VmError::type_error(format!(
                    "{error_prefix}: future-proposal reference types are not yet supported from JS"
                ))),
            }
        }
    }
}

/// Default initializer per [`WasmValueType`] for the v=undefined case.
fn default_value_for(ty: WasmValueType) -> WasmValue {
    match ty {
        WasmValueType::I32 => WasmValue::I32(0),
        WasmValueType::I64 => WasmValue::I64(0),
        WasmValueType::F32 => WasmValue::F32(0.0),
        WasmValueType::F64 => WasmValue::F64(0.0),
        WasmValueType::V128 => WasmValue::V128([0; 16]),
        WasmValueType::Ref(ref_ty) => WasmValue::Ref(WasmRef::Null(ref_ty.heap)),
    }
}

/// Reverse coerce a [`WasmValue`] back to a JS value.  Numeric
/// variants are direct; ref variants surface as `null` per the Stage
/// 3 simplification (exported-function reverse-lookup lands in
/// Stage 5).  i64 → f64 lossy precision beyond 2^53; Stage 5 will
/// tighten to JS BigInt.
fn wasm_value_to_js(v: &WasmValue) -> JsValue {
    match v {
        WasmValue::I32(i) => JsValue::Number(f64::from(*i)),
        WasmValue::I64(i) => {
            #[allow(clippy::cast_precision_loss)]
            let n = *i as f64;
            JsValue::Number(n)
        }
        WasmValue::F32(f) => JsValue::Number(f64::from(*f)),
        WasmValue::F64(f) => JsValue::Number(*f),
        WasmValue::V128(_) | WasmValue::Ref(WasmRef::Extern(_) | WasmRef::Func(_)) => {
            JsValue::Undefined
        }
        WasmValue::Ref(WasmRef::Null(_)) => JsValue::Null,
    }
}
