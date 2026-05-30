//! Exports namespace builder + exported function call adapter —
//! slot `#11-wasm-vm` / D-16, plan-memo §5 Stage 3.2.
//!
//! Per WASM JS API §5.6 Exported Functions, each `instance.exports.f`
//! is a Function exotic whose call dispatches the wasm function
//! through `WasmFunc::call(args, ScriptHostBinding)`.  The
//! `[[FunctionAddress]]` is interpreted relative to the surrounding
//! agent's associated store (§4.1) — by routing the call through
//! the F1 `WasmFunc` (which carries a clone of the `WasmStoreHandle`
//! owned by the parent `WasmInstance`), cross-store mismatch is
//! structurally impossible.
//!
//! The exports namespace itself is built per WASM JS API §5 "create
//! an exports object":
//!
//! 1. Let `exportsObject` be `OrdinaryObjectCreate(null)`.
//! 2. For each export of `module.exports`, set the matching property
//!    on `exportsObject`.
//! 3. Perform `SetIntegrityLevel(exportsObject, "frozen")`.
//!
//! All 4 [`WasmExportItem`] variants are wrapped:
//! - `Func` → fresh `ObjectKind::WasmExportedFunction` with cached
//!   params (per F1 F8 lesson — avoids per-call `func_type()` walk).
//! - `Memory` → fresh `ObjectKind::WasmMemory` payload with
//!   `buffer_id: None` (lazily-set on first `.buffer` accessor fire
//!   in Stage 4) and `view: None`.
//! - `Table` → fresh `ObjectKind::WasmTable` with cached
//!   `element_kind` from F2 `WasmTable::element_kind()`.
//! - `Global` → fresh `ObjectKind::WasmGlobal`.

use elidex_wasm_runtime::{
    HeapType, ScriptHostBinding, WasmExportItem, WasmFunc, WasmRef, WasmValue, WasmValueType,
};

use super::super::super::coerce::f64_to_int32;
use super::super::super::error::VmError;
use super::super::super::natives_bigint::to_bigint64;
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue,
};
use super::super::super::wasm_payload::{
    WasmExportedFuncPayload, WasmGlobalPayload, WasmMemoryPayload, WasmTablePayload,
};
use super::errors::wasm_error_to_vm_error;

/// PropertyAttrs for an own data property on the frozen exports
/// namespace per WASM JS API §5 "create an exports object" step 2.
///
/// SetIntegrityLevel "frozen" runs in step 3 → writable=false +
/// configurable=false on every property.  Enumerability is preserved
/// from step 2's `CreateDataProperty` (writable + enumerable +
/// configurable) — so the post-freeze state is {¬W, E, ¬C}, matching
/// the [`PropertyAttrs::WEBIDL_RO_PERMANENT`] shape.
const FROZEN_EXPORT_ATTR: PropertyAttrs = PropertyAttrs::WEBIDL_RO_PERMANENT;

/// Walk `WasmInstance::exports()` and build the frozen exports
/// namespace JS object per WASM JS API §5 `create an exports
/// object` (called via §5.2 ctor step 6 → `initialize an instance
/// object` step 3).  Returns the allocated namespace's `ObjectId`.
///
/// Each exported item is wrapped into a fresh JS wrapper with the
/// matching `ObjectKind` brand + inserted into the per-VM side-store
/// (see file-level docstring for the per-variant payload shapes).
///
/// The namespace itself is allocated with `prototype: None` per spec
/// step 1 (`OrdinaryObjectCreate(null)`) + `extensible: true` so each
/// per-export `define_shaped_property` succeeds; `extensible` is then
/// flipped to `false` after population per the `SetIntegrityLevel(_,
/// "frozen")` final step.  Each property is installed with
/// [`FROZEN_EXPORT_ATTR`] so the post-freeze {¬W, E, ¬C} state is
/// observable from JS via `Object.getOwnPropertyDescriptor`.
pub(super) fn build_exports_namespace(
    ctx: &mut NativeContext<'_>,
    instance_id: ObjectId,
) -> Result<ObjectId, VmError> {
    // Pull the engine-bridge `WasmInstance` clone.  We need a fresh
    // clone (not a borrowed reference) because `exports()` mutably
    // borrows the underlying store + we'll need to allocate JS
    // wrappers (which also borrows VmInner) during the walk.
    let instance = ctx
        .vm
        .wasm_instance_storage
        .get(&instance_id)
        .expect("instance brand-check guarantees storage entry exists")
        .instance
        .clone();

    let exports = instance.exports();
    let namespace_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        // WASM JS API §5 step 1 — `OrdinaryObjectCreate(null)`.
        prototype: None,
        extensible: true,
    });

    for (name, item) in exports {
        let export_id = match item {
            WasmExportItem::Func(func) => wrap_exported_func(ctx, instance_id, func)?,
            WasmExportItem::Memory(memory) => wrap_exported_memory(ctx, memory),
            WasmExportItem::Table(table) => wrap_exported_table(ctx, table)?,
            WasmExportItem::Global(global) => wrap_exported_global(ctx, global),
        };
        let name_sid = ctx.vm.strings.intern(&name);
        ctx.vm.define_shaped_property(
            namespace_id,
            PropertyKey::String(name_sid),
            PropertyValue::Data(JsValue::Object(export_id)),
            FROZEN_EXPORT_ATTR,
        );
    }

    // §5 step 3 — `SetIntegrityLevel(exportsObject, "frozen")` final
    // step.  Property descriptors are already non-writable +
    // non-configurable (via `FROZEN_EXPORT_ATTR`); flip extensible
    // to seal the shape.  `Object.isFrozen(i.exports) === true`
    // holds post-this.
    ctx.vm.get_object_mut(namespace_id).extensible = false;
    Ok(namespace_id)
}

/// Wrap a [`WasmFunc`] into a Function exotic.  Caches the function's
/// param type list at wrap time per F1 F8 lesson (avoids per-call
/// `func_type()` walk + moves any future-proposal HeapType
/// conversion error from per-call to module-load time).
///
/// The returned JS object's `ObjectKind::WasmExportedFunction`
/// brand causes the standard call path (see
/// `vm/interpreter.rs::call_dispatch`) to route through
/// [`call_wasm_exported_function`] — no NativeFunction trampoline
/// indirection.  Wasm exports cannot be `new`-called (per WASM JS
/// API §5.6 + Chrome/Firefox behaviour); `do_new` on a
/// `WasmExportedFunction` reaches the catch-all "not a constructor"
/// arm in `ops.rs::do_new` so the spec contract is structurally
/// enforced.
fn wrap_exported_func(
    ctx: &mut NativeContext<'_>,
    instance_id: ObjectId,
    func: WasmFunc,
) -> Result<ObjectId, VmError> {
    // Cache params now — propagates any future-proposal HeapType
    // conversion error as `LinkError` at module-load time per F1 F8.
    let func_type = func
        .func_type()
        .map_err(|e| wasm_error_to_vm_error(ctx, &e))?;
    let params = func_type.params.clone();
    let func_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::WasmExportedFunction,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        // Function exotics chain to Function.prototype per WebIDL
        // §3.10.16; reuse the standard `function_prototype` if
        // populated so `typeof f === 'function'` + Function-prototype
        // methods (`bind` / `call` / `apply`) resolve correctly.
        prototype: ctx.vm.function_prototype,
        extensible: true,
    });
    ctx.vm.wasm_exported_func_storage.insert(
        func_id,
        WasmExportedFuncPayload {
            func,
            params,
            instance_id,
        },
    );
    Ok(func_id)
}

/// Wrap a [`WasmMemory`](elidex_wasm_runtime::WasmMemory) into a JS
/// `WebAssembly.Memory` instance.  `buffer_id` + `view` remain
/// `None` until the first `.buffer` accessor fire (Stage 4 wiring).
fn wrap_exported_memory(
    ctx: &mut NativeContext<'_>,
    memory: elidex_wasm_runtime::WasmMemory,
) -> ObjectId {
    let proto = ctx
        .vm
        .wasm_memory_prototype
        .expect("wasm_memory_prototype populated in register_wasm_namespace");
    let mem_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::WasmMemory,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.vm.wasm_memory_storage.insert(
        mem_id,
        WasmMemoryPayload {
            memory,
            buffer_id: None,
            view: None,
        },
    );
    mem_id
}

/// Wrap a [`WasmTable`](elidex_wasm_runtime::WasmTable) into a JS
/// `WebAssembly.Table` instance.  Caches `element_kind` from F2
/// `WasmTable::element_kind()` at wrap time (IMMUTABLE post-build —
/// wasm validation fixes the table element type so no re-sync
/// needed).  Returns `LinkError` if `element_kind` surfaces a
/// future-proposal HeapType variant (e.g. concrete func types).
fn wrap_exported_table(
    ctx: &mut NativeContext<'_>,
    table: elidex_wasm_runtime::WasmTable,
) -> Result<ObjectId, VmError> {
    let element_kind = table
        .element_kind()
        .map_err(|e| wasm_error_to_vm_error(ctx, &e))?;
    let proto = ctx
        .vm
        .wasm_table_prototype
        .expect("wasm_table_prototype populated in register_wasm_namespace");
    let table_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::WasmTable,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.vm.wasm_table_storage.insert(
        table_id,
        WasmTablePayload {
            table,
            element_kind,
        },
    );
    Ok(table_id)
}

/// Wrap a [`WasmGlobal`](elidex_wasm_runtime::WasmGlobal) into a JS
/// `WebAssembly.Global` instance.  `value_type` / `mutable` read on
/// demand via handle accessors (sentinel discipline per plan-memo
/// §2.2 — no duplicate metadata fields).
fn wrap_exported_global(
    ctx: &mut NativeContext<'_>,
    global: elidex_wasm_runtime::WasmGlobal,
) -> ObjectId {
    let proto = ctx
        .vm
        .wasm_global_prototype
        .expect("wasm_global_prototype populated in register_wasm_namespace");
    let global_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::WasmGlobal,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.vm
        .wasm_global_storage
        .insert(global_id, WasmGlobalPayload { global });
    global_id
}

/// Call adapter for exported wasm functions per WASM JS API §5.6.
/// Invoked by `vm/interpreter.rs::call_dispatch`'s
/// [`ObjectKind::WasmExportedFunction`] arm.
///
/// Resolves the `WasmFunc` + cached params via the brand-checked
/// `func_obj_id`, coerces each JS argument to a `WasmValue` per the
/// declared param type, dispatches through F1 `WasmFunc::call(args,
/// ScriptHostBinding)`, then reverse-coerces results.
///
/// Multi-value results (>1) return a JS `Array` per the WebAssembly
/// multi-value proposal; 1-result returns the JS scalar; 0-result
/// returns `undefined`.
pub(in crate::vm) fn call_wasm_exported_function(
    ctx: &mut NativeContext<'_>,
    func_obj_id: ObjectId,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (func, params) = {
        let payload = ctx
            .vm
            .wasm_exported_func_storage
            .get(&func_obj_id)
            .ok_or_else(|| {
                VmError::type_error("WebAssembly exported function called on non-exported-function")
            })?;
        (payload.func.clone(), payload.params.clone())
    };

    // Coerce JS args → WasmValues per declared param types.
    let mut wasm_args: Vec<WasmValue> = Vec::with_capacity(params.len());
    for (i, ty) in params.iter().enumerate() {
        let arg = args.get(i).copied().unwrap_or(JsValue::Undefined);
        wasm_args.push(js_value_to_wasm_value(ctx, arg, *ty)?);
    }

    // Resolve the document entity BEFORE borrowing `host_data` for
    // the with_session_and_dom call (document() takes &self;
    // with_session_and_dom takes &mut self).
    let document = match ctx.host_if_bound() {
        Some(hd) => hd.document(),
        None => {
            return Err(VmError::type_error(
                "WebAssembly exported function called outside a bound VM session",
            ));
        }
    };

    let host_data = ctx
        .host_if_bound()
        .expect("bound check above guarantees Some here");
    let result = host_data.with_session_and_dom(|session, dom| {
        func.call(
            &wasm_args,
            ScriptHostBinding {
                session,
                dom,
                document,
            },
        )
    });

    match result {
        Ok(values) => Ok(wasm_values_to_js_result(ctx, &values)),
        Err(e) => Err(wasm_error_to_vm_error(ctx, &e)),
    }
}

/// Coerce a JS value to a [`WasmValue`] per the declared parameter
/// type.  Matches the WebAssembly JS API §5.6 ToWebAssemblyValue
/// algorithm.
///
/// - `I32`: `ToInt32` per JS coerce rules (Number input, mod-2^32).
/// - `I64`: `ToBigInt64` per JS coerce rules; accepts BigInt only
///   (low-64 two's-complement slice).  Number / undefined / null /
///   Symbol throw TypeError per strict ToBigInt semantics.
/// - `F32` / `F64`: `ToNumber`.
/// - `V128`: synchronous TypeError per F1 F13 (no JS surface to
///   construct a v128 value).
/// - `Ref(funcref)` / `Ref(externref)`: nullable variant accepts
///   `null`/`undefined` → `WasmRef::Null(heap)`; non-null only
///   supported for funcref via a wrapped exported function (Stage 3
///   limits this; non-nullable + non-Func/Extern surfaces as
///   TypeError per F1 F13).
fn js_value_to_wasm_value(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    expected: WasmValueType,
) -> Result<WasmValue, VmError> {
    match expected {
        WasmValueType::I32 => {
            let n = ctx.to_number(val)?;
            // ECMA-262 §7.1.7 ToInt32 — mod-2^32 wrap via shared helper.
            // Direct `n as i64 as i32` would saturate per Rust RFC 2484
            // (Infinity → -1, NaN-handling diverges) instead of spec
            // mod-2^32 truncation.
            Ok(WasmValue::I32(f64_to_int32(n)))
        }
        WasmValueType::I64 => {
            // ECMA-262 §7.1.16 ToBigInt64 via the shared helper —
            // accepts BigInt (low-64 two's-complement slice), throws
            // TypeError on Number / undefined / null / Symbol per
            // strict ToBigInt semantics.
            Ok(WasmValue::I64(to_bigint64(ctx, val)?))
        }
        WasmValueType::F32 => {
            let n = ctx.to_number(val)?;
            #[allow(clippy::cast_possible_truncation)]
            let f = n as f32;
            Ok(WasmValue::F32(f))
        }
        WasmValueType::F64 => {
            let n = ctx.to_number(val)?;
            Ok(WasmValue::F64(n))
        }
        WasmValueType::V128 => Err(VmError::type_error(
            "WebAssembly v128 values cannot be passed from JS",
        )),
        WasmValueType::Ref(ref_ty) => {
            // Nullable null → typed-null.
            if ref_ty.nullable && matches!(val, JsValue::Null | JsValue::Undefined) {
                return Ok(WasmValue::Ref(WasmRef::Null(ref_ty.heap)));
            }
            match ref_ty.heap {
                HeapType::Func => {
                    // Non-null funcref: must be an exported wasm
                    // function wrapper.  Stage 3 reads the side-store
                    // entry to recover the underlying `WasmFunc`.
                    let JsValue::Object(id) = val else {
                        return Err(VmError::type_error(
                            "WebAssembly funcref parameter must be a wasm exported function or null",
                        ));
                    };
                    let payload = ctx.vm.wasm_exported_func_storage.get(&id).ok_or_else(|| {
                        VmError::type_error(
                            "WebAssembly funcref parameter must be a wasm exported function",
                        )
                    })?;
                    Ok(WasmValue::Ref(WasmRef::Func(payload.func.clone())))
                }
                HeapType::Extern => {
                    // Non-null externref: Stage 3 doesn't yet
                    // support host-owned externref payloads (defer
                    // to `#11-wasm-externref-host-payload` slot
                    // pending host_extern_ref design).  Reject with
                    // TypeError per F1 F13.
                    Err(VmError::type_error(
                        "WebAssembly non-null externref values are not yet supported from JS",
                    ))
                }
                // `HeapType` is `#[non_exhaustive]` — future proposals
                // (Exception Handling Exn / NoExn, GC variants) reject
                // until host machinery lands.
                _ => Err(VmError::type_error(
                    "WebAssembly future-proposal reference types are not yet supported from JS",
                )),
            }
        }
    }
}

/// Reverse coerce result `WasmValue`s into a JS result per WASM JS
/// API §5.6 multi-value proposal:
/// - 0 results → `undefined`
/// - 1 result → the converted scalar
/// - 2+ results → a JS `Array` of converted scalars
fn wasm_values_to_js_result(ctx: &mut NativeContext<'_>, values: &[WasmValue]) -> JsValue {
    match values.len() {
        0 => JsValue::Undefined,
        1 => wasm_value_to_js(ctx, &values[0]),
        _ => {
            let elements: Vec<JsValue> = values.iter().map(|v| wasm_value_to_js(ctx, v)).collect();
            JsValue::Object(ctx.vm.create_array_object(elements))
        }
    }
}

/// Convert a single [`WasmValue`] to a JS value per WASM JS API §5.6
/// ToJSValue algorithm.  SoT for the conversion — both
/// `exported_func` (multi-result) and `global` / `table` accessors
/// route through this single helper per `feedback_one-issue-one-way`.
///
/// - `I64` → JS `BigInt` per ECMA-262 / WASM JS API §5.6.
/// - `Ref::Func` → reverse-lookup against `wasm_exported_func_storage`
///   (linear scan, O(N) where N = exported funcs of all live
///   instances — typically ≤ 64 per instance, acceptable until
///   identity cache lands as a follow-up).
/// - `Ref::Extern` → host-owned externref payload not yet wired
///   (returns `undefined`; defer per `#11-wasm-externref-host-payload`).
pub(super) fn wasm_value_to_js(ctx: &mut NativeContext<'_>, val: &WasmValue) -> JsValue {
    match val {
        WasmValue::I32(i) => JsValue::Number(f64::from(*i)),
        WasmValue::I64(i) => {
            // ECMA-262 ToJSValue(i64) → BigInt per WASM JS API §5.6.
            let bi = num_bigint::BigInt::from(*i);
            JsValue::BigInt(ctx.vm.bigints.alloc(bi))
        }
        WasmValue::F32(f) => JsValue::Number(f64::from(*f)),
        WasmValue::F64(f) => JsValue::Number(*f),
        WasmValue::Ref(WasmRef::Null(_)) => JsValue::Null,
        WasmValue::Ref(WasmRef::Func(wf)) => {
            // Reverse-lookup against existing exported-function
            // wrappers so `f === f` identity holds when the engine
            // round-trips the same `WasmFunc`.  Linear scan over the
            // side-store; if no match (e.g. a funcref minted by an
            // instruction that elidex hasn't observed at JS surface
            // yet), returns null until host-fn-builder slot lands.
            funcref_reverse_lookup(ctx, wf).unwrap_or(JsValue::Null)
        }
        WasmValue::V128(_) | WasmValue::Ref(WasmRef::Extern(_)) => {
            // V128 / externref not representable at JS boundary in
            // Stage 3 surface.  Same disposition as below catch-all.
            JsValue::Undefined
        }
    }
}

/// Reverse-lookup an exported-function wrapper for the given
/// `WasmFunc`.  Returns `Some(JsValue::Object(id))` if the side-store
/// already holds a wrapper for the underlying function, else `None`
/// (caller decides the fallback — currently `null` per Stage 3).
///
/// **Identity by `WasmFunc::is_same_func`**: F1 handle defines the
/// same-function predicate as identity of the underlying `wasmtime`
/// `Func` (i.e. same store + same export index).  Linear scan over
/// `wasm_exported_func_storage` is O(N) but N is bounded by the
/// total exported-function wrappers across all live instances; for
/// typical SPA workloads this stays in the 10s-100s range.
fn funcref_reverse_lookup(ctx: &NativeContext<'_>, wf: &WasmFunc) -> Option<JsValue> {
    for (id, payload) in &ctx.vm.wasm_exported_func_storage {
        if payload.func.is_same_func(wf) {
            return Some(JsValue::Object(*id));
        }
    }
    None
}
