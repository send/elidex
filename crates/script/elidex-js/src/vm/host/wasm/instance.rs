//! `WebAssembly.Instance` constructor + `.exports` accessor — slot
//! `#11-wasm-vm` / D-16, plan-memo §5 Stage 3.
//!
//! Per WASM JS API §5.2 Instance ctor algorithm (steps 4-6):
//!
//! 4. Read the imports of `module` against the supplied
//!    `importObject` record-of-records.
//! 5. Instantiate the core of `module` (step 5 inner step 3 surfaces
//!    `LinkError` / `RuntimeError` / "another error type if
//!    appropriate, for example an out-of-memory exception" — RangeError
//!    is inferred from §7.2 per-algorithm OOM mapping for Memory.grow /
//!    Table.grow specifically, not literally enumerated here).
//! 6. Initialize the Instance object from `module` and `instance`.
//!
//! Per F1 D-vi guard, the engine-bridge `WasmRuntime::instantiate`
//! rejects non-empty `ImportObject` with `LinkError` until the
//! `#11-wasm-user-import-host-fn-builder` slot lands the shared-store
//! wiring.  This file's JS-side coerce does NOT add a second guard —
//! per `feedback_one-issue-one-way`, the singular rejection site
//! stays at F1's `runtime.rs:122-130` so the future lift is a single
//! removal.
//!
//! The `.exports` accessor is wrapper-identity-stable (DR-4) —
//! cached in `WasmInstancePayload.exports_id` so
//! `i.exports === i.exports` holds across calls.  IDL has no
//! `[SameObject]` attribute; the choice is motivated by
//! `Object.isFrozen` + ergonomic identity + cycle avoidance.

use elidex_wasm_runtime::ImportObject;

use super::super::super::error::VmError;
use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage,
};
use super::super::super::wasm_payload::WasmInstancePayload;
use super::errors::wasm_error_to_js_value;
use super::exported_func::build_exports_namespace;

/// `new WebAssembly.Instance(module, importObject?)` — WASM JS API
/// §5.2.  Synchronous; throws (not Promise-rejects) on failure since
/// the synchronous form does not wrap in a Promise.
pub(super) fn native_wasm_instance_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Instance' on 'WebAssembly': Please use the 'new' operator",
        ));
    }
    let module_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let import_object_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let module_id = require_module(ctx, module_arg)?;

    // Brand-promote the construct-mode receiver so subclassing via
    // `class X extends WebAssembly.Instance {}` preserves
    // `new.target.prototype` (ECMA-262 §10.2.2 [[Construct]] step
    // 5.b → §10.1.13 OrdinaryCreateFromConstructor).  Mirrors the
    // Module / Memory / Table / Global ctor pattern.
    let proto = ctx
        .vm
        .wasm_instance_prototype
        .expect("wasm_instance_prototype populated in register_wasm_namespace");
    let receiver = ctx.vm.ensure_instance_or_alloc(this, Some(proto), ctx.mode);
    let JsValue::Object(receiver_id) = receiver else {
        unreachable!("ensure_instance_or_alloc returns an Object");
    };

    // Build ImportObject + instantiate, installing the brand + payload
    // onto the receiver in place (no second alloc).  Errors thrown
    // synchronously per §5.2 ctor semantics.
    instantiate_module(ctx, module_id, import_object_arg, Some(receiver_id))
        .map_err(VmError::throw)?;
    Ok(receiver)
}

/// `Instance.prototype.exports` getter — WASM JS API §5.2 / §5
/// `initialize an instance object` step 3 + IDL getter
/// (`exports attribute of Instance returns this.[[Exports]]`).
///
/// Wrapper-identity-stable per DR-4: first access allocates the
/// frozen namespace via [`build_exports_namespace`] + caches in
/// [`WasmInstancePayload::exports_id`]; subsequent reads return the
/// cached `ObjectId`.
pub(super) fn native_wasm_instance_exports_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(this_id) = this else {
        return Err(VmError::type_error(
            "WebAssembly.Instance.prototype.exports called on non-Instance",
        ));
    };
    if !matches!(ctx.vm.get_object(this_id).kind, ObjectKind::WasmInstance) {
        return Err(VmError::type_error(
            "WebAssembly.Instance.prototype.exports called on non-Instance",
        ));
    }

    if let Some(cached) = ctx
        .vm
        .wasm_instance_storage
        .get(&this_id)
        .and_then(|p| p.exports_id)
    {
        return Ok(JsValue::Object(cached));
    }

    let namespace_id = build_exports_namespace(ctx, this_id)?;
    if let Some(payload) = ctx.vm.wasm_instance_storage.get_mut(&this_id) {
        payload.exports_id = Some(namespace_id);
    }
    Ok(JsValue::Object(namespace_id))
}

/// Brand-check that `module_arg` is a `WebAssembly.Module` instance.
fn require_module(ctx: &NativeContext<'_>, module_arg: JsValue) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = module_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'Instance' on 'WebAssembly': parameter 1 is not of type 'Module'",
        ));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmModule) {
        return Err(VmError::type_error(
            "Failed to construct 'Instance' on 'WebAssembly': parameter 1 is not of type 'Module'",
        ));
    }
    Ok(id)
}

/// Shared instantiate path consumed by both the sync `new
/// WebAssembly.Instance(...)` ctor and the async
/// `WebAssembly.instantiate(...)` namespace method.
///
/// Returns `Ok(instance_id)` on success or `Err(JsValue)` with the
/// JS-formatted rejection reason (CompileError / LinkError /
/// RuntimeError / TypeError instance) on failure.  The async
/// namespace caller settles the Promise with the value; the sync
/// ctor caller wraps as `VmError::throw`.
///
/// `import_object_arg` is the user-supplied JS value (typically a
/// record-of-records or `undefined`).  Per F1 D-vi, this Stage 3
/// surface only supports the empty-imports case (undefined / null /
/// `{}` / `Object.create(null)`); any non-empty record produces a
/// LinkError via the engine-bridge guard.  No second JS-side guard
/// is added (singular rejection site per
/// `feedback_one-issue-one-way`).
/// `target` is `Some(receiver_id)` for the sync ctor path (brand-
/// promote that receiver to preserve `new.target.prototype` for
/// subclassing) or `None` for the async `WebAssembly.instantiate`
/// namespace path (fresh-alloc against `wasm_instance_prototype`).
pub(super) fn instantiate_module(
    ctx: &mut NativeContext<'_>,
    module_id: ObjectId,
    import_object_arg: JsValue,
    target: Option<ObjectId>,
) -> Result<ObjectId, JsValue> {
    // Build ImportObject from JS record-of-records.  Stage 3 walks
    // the user dict but rejects non-empty cases via the engine-bridge
    // guard; the JS-side walk catches non-Object `importObject` (per
    // WebIDL `optional object importObject` — TypeError on non-object
    // / non-undefined value).
    let import_object = coerce_import_object(ctx, import_object_arg)
        .map_err(|err| ctx.vm.vm_error_to_thrown(&err))?;

    // Clone the module out of the side-store so we can drop the
    // borrow before calling `runtime.instantiate` (which takes
    // `&WasmModule` — needs the clone to survive the storage borrow).
    let module = ctx
        .vm
        .wasm_module_storage
        .get(&module_id)
        .expect("brand-check guarantees storage entry exists")
        .module
        .clone();

    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(wasm_error_to_js_value(ctx, &e)),
    };
    let instance = match runtime.instantiate(&module, &import_object) {
        Ok(i) => i,
        Err(e) => return Err(wasm_error_to_js_value(ctx, &e)),
    };

    let instance_id = if let Some(receiver_id) = target {
        // Brand-promote the construct-receiver in place — preserves
        // `new.target.prototype` subclass chain.
        ctx.vm.get_object_mut(receiver_id).kind = ObjectKind::WasmInstance;
        receiver_id
    } else {
        let proto = ctx
            .vm
            .wasm_instance_prototype
            .expect("wasm_instance_prototype populated in register_wasm_namespace");
        ctx.vm.alloc_object(Object {
            kind: ObjectKind::WasmInstance,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        })
    };
    ctx.vm.wasm_instance_storage.insert(
        instance_id,
        WasmInstancePayload {
            instance,
            module_id,
            exports_id: None,
        },
    );
    Ok(instance_id)
}

/// Coerce the user-supplied `importObject` JS value into an engine-
/// indep [`ImportObject`].
///
/// Per WebIDL `optional object importObject`:
/// - `undefined` → empty `ImportObject`
/// - non-object (string / number / boolean / etc.) → TypeError
/// - object → walked as a record-of-records.  Stage 3 only handles
///   the empty case — any defined module-level entry yields an
///   empty `ImportObject` here and lets F1 D-vi reject downstream
///   with `LinkError`.  This is intentional per
///   `feedback_one-issue-one-way`: singular rejection site stays at
///   the F1 engine-bridge guard so the future host-fn-builder lift
///   is a single removal.
fn coerce_import_object(
    _ctx: &NativeContext<'_>,
    import_object_arg: JsValue,
) -> Result<ImportObject, VmError> {
    match import_object_arg {
        JsValue::Undefined => Ok(ImportObject::default()),
        JsValue::Object(_) => {
            // Stage 3: build empty ImportObject regardless of content.
            // Future host-fn-builder slot walks the record-of-records
            // here.  See file-level docstring + F1 D-vi reference.
            Ok(ImportObject::default())
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Instance' on 'WebAssembly': parameter 2 is not of type 'object'",
        )),
    }
}
