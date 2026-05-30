//! `WebAssembly.Module` constructor plus the 3 static methods
//! `exports` / `imports` / `customSections` — slot `#11-wasm-vm` /
//! D-16, plan-memo §5 Stage 2.3.
//!
//! WASM JS API §5.1 Module ctor algorithm:
//!
//! 1. Let `stableBytes` be a copy of `bytes` (`AllowSharedBufferSource`
//!    IDL union — ArrayBuffer / SharedArrayBuffer / TypedArray view).
//! 2. Compile the WebAssembly module `stableBytes` and store the
//!    result as `module`.
//! 3. If `module` is error, throw a `CompileError` exception.
//!
//! WebIDL anchors verified via
//! `.claude/tools/webref body wasm-js-api-2 dom-module-module` per
//! `feedback_helper-prefer-upstream-machine-readable.md`.

use elidex_wasm_runtime::ImportExportKind;

use super::super::super::error::VmError;
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue,
};
use super::super::super::wasm_payload::WasmModulePayload;
use super::super::super::VmInner;
use super::super::array_buffer;
use super::super::text_encoding::extract_buffer_source_bytes;

/// `new WebAssembly.Module(bytes, options?)` — WASM JS API §5.1.
///
/// `bytes` is the WebIDL `AllowSharedBufferSource` union (ArrayBuffer
/// / SharedArrayBuffer / TypedArray view).  Coerced via the shared
/// [`extract_buffer_source_bytes`] helper (slot
/// `#11-array-buffer-detach-state` / F3 — detached-buffer rejection
/// at the WebIDL conversion boundary is structurally enforced).
///
/// On compile failure the helper materialises a JS
/// `WebAssembly.CompileError` instance via
/// [`super::errors::wasm_error_to_vm_error`] and propagates as a
/// VmError throw (§5.1 step 3).
pub(super) fn native_wasm_module_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // The historic `if !ctx.is_construct()` entry guard lives at
    // `vm/interpreter.rs::call_dispatch` NativeFunction arm under the
    // `CallShape::ConstructorOnly` discriminant (installed via
    // `create_constructor_only_function` in `register_wasm_namespace`).
    // Per slot `#11-vm-native-constructor-only-flag` this body trusts
    // the dispatch gate to have rejected bare-call already.
    let bytes_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // §5.1 step 1 stableBytes — `extract_buffer_source_bytes` returns
    // a freshly-owned `Vec<u8>` so the copy is structural.  The
    // `allow_undefined_as_empty=false` arg matches IDL: the union has
    // no `null/undefined` member, so `new Module()` MUST surface
    // TypeError per WebIDL §3.2.25 Union types / §3.2.26 Buffer
    // source types conversion.
    let bytes = extract_buffer_source_bytes(
        ctx,
        bytes_arg,
        "Failed to construct 'Module' on 'WebAssembly'",
        1,
        false,
    )?;

    // §5.1 steps 2-3: compile via the engine-bridge `WasmRuntime`.
    // Compile failure → CompileError (kind-based marshal).  Engine-
    // bridge runtime-singleton ctor failure surfaces with a kind from
    // `WasmRuntime::new`'s set (`Compile` for engine build / unavailable
    // cranelift, `Link` for host-fn registration); `wasm_error_to_vm_error`
    // is kind-based so the JS class follows the underlying failure mode
    // (`Compile → CompileError`, `Link → LinkError`, `Runtime → RuntimeError`).
    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(super::errors::wasm_error_to_vm_error(ctx, &e)),
    };
    let module = match runtime.compile(&bytes) {
        Ok(m) => m,
        Err(e) => return Err(super::errors::wasm_error_to_vm_error(ctx, &e)),
    };

    // §5.1 — allocate Module wrapper with brand `ObjectKind::WasmModule`
    // chained to `WebAssembly.Module.prototype`; insert payload into
    // side-store.
    let proto = ctx.vm.wasm_module_prototype.expect(
        "wasm_module_prototype populated by register_wasm_namespace during register_globals",
    );
    let receiver = ctx.vm.ensure_instance_or_alloc(this, Some(proto), ctx.mode);
    let JsValue::Object(id) = receiver else {
        unreachable!("ensure_instance_or_alloc returns an Object");
    };
    // Promote the receiver from `Ordinary` to `WasmModule` so brand
    // checks at the static methods land correctly.  The shaped
    // storage is preserved (caller may have allocated with
    // `ROOT_SHAPE`); per-property writes below add no enumerable
    // surface (Module has no own data properties — `exports` /
    // `imports` / `customSections` live on the constructor, not the
    // instance per WASM JS API §5.1 IDL).
    ctx.vm.get_object_mut(id).kind = ObjectKind::WasmModule;
    ctx.vm
        .wasm_module_storage
        .insert(id, WasmModulePayload { module });
    Ok(receiver)
}

/// `WebAssembly.Module.exports(moduleObject)` — WASM JS API §5.1.
///
/// Returns a JS `Array` of `{ name, kind }` dictionary objects per
/// `ModuleExportDescriptor` IDL.  Order follows
/// [`elidex_wasm_runtime::WasmModule::exports`] (module binary order).
///
/// Throws `TypeError` if `moduleObject` is not a Module instance
/// (per IDL `Module` interface argument type-check, WebIDL §3.2.15
/// Interface types).
pub(super) fn native_wasm_module_exports(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let module = require_module_argument(ctx, args, "exports")?;
    let descriptors = {
        let payload = ctx
            .vm
            .wasm_module_storage
            .get(&module)
            .expect("brand-check guarantees storage entry exists");
        payload.module.exports()
    };
    let elements: Vec<JsValue> = descriptors
        .into_iter()
        .map(|d| JsValue::Object(build_module_descriptor_object(ctx.vm, &d.name, d.kind)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
}

/// `WebAssembly.Module.imports(moduleObject)` — WASM JS API §5.1.
///
/// Returns a JS `Array` of `{ module, name, kind }` dictionary
/// objects per `ModuleImportDescriptor` IDL.  Order follows
/// [`elidex_wasm_runtime::WasmModule::imports`] (module binary order).
pub(super) fn native_wasm_module_imports(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let module = require_module_argument(ctx, args, "imports")?;
    let descriptors = {
        let payload = ctx
            .vm
            .wasm_module_storage
            .get(&module)
            .expect("brand-check guarantees storage entry exists");
        payload.module.imports()
    };
    let elements: Vec<JsValue> = descriptors
        .into_iter()
        .map(|d| {
            let mod_str = d.module.clone();
            let inner = build_module_descriptor_object(ctx.vm, &d.name, d.kind);
            // Add the `module` key (only present on import descriptors).
            let module_sid = ctx.vm.strings.intern(&mod_str);
            let module_name_sid = ctx.vm.strings.intern("module");
            ctx.vm.define_shaped_property(
                inner,
                PropertyKey::String(module_name_sid),
                PropertyValue::Data(JsValue::String(module_sid)),
                // WebIDL §3.2.17 dictionary→JS: `CreateDataPropertyOrThrow`
                // ({W, E, C}).  Required so `Object.keys(desc)` enumerates
                // the dictionary members.
                PropertyAttrs::DATA,
            );
            JsValue::Object(inner)
        })
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
}

/// `WebAssembly.Module.customSections(moduleObject, sectionName)` —
/// WASM JS API §5.1.
///
/// Iterates per `For each custom section customSection of bytes,
/// interpreted according to the module grammar` (webref
/// `body wasm-js-api-2 dom-module-customsections` step 3).
/// Implementation follows binary order of the `customsec` sequence
/// in the module byte stream.  Each match becomes a fresh
/// `ArrayBuffer` whose backing bytes own the payload copy.
pub(super) fn native_wasm_module_custom_sections(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let module = require_module_argument(ctx, args, "customSections")?;
    let name_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let name_sid = ctx.to_string_val(name_arg)?;
    let name_string = ctx.vm.strings.get_utf8(name_sid);

    let payloads = {
        let payload = ctx
            .vm
            .wasm_module_storage
            .get(&module)
            .expect("brand-check guarantees storage entry exists");
        payload.module.custom_sections(&name_string)
    };
    let elements: Vec<JsValue> = payloads
        .into_iter()
        .map(|bytes| {
            let buf_id = array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
            JsValue::Object(buf_id)
        })
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
}

/// Brand-check helper: pulls the first arg, requires it to be a
/// `WebAssembly.Module` instance, returns its `ObjectId`.  Used by
/// the 3 static methods to guard against non-Module arguments
/// (IDL `Module` interface type-check per WebIDL §3.2.15
/// Interface types).
fn require_module_argument(
    ctx: &NativeContext<'_>,
    args: &[JsValue],
    method: &'static str,
) -> Result<ObjectId, VmError> {
    let Some(JsValue::Object(id)) = args.first().copied() else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Module': parameter 1 is not of type 'Module'"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmModule) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Module': parameter 1 is not of type 'Module'"
        )));
    }
    Ok(id)
}

/// Build a `{ name, kind }` ordinary object for an export/import
/// descriptor per `ModuleExportDescriptor` / `ModuleImportDescriptor`
/// IDL.  Caller adds `module` for the import variant.
fn build_module_descriptor_object(
    vm: &mut VmInner,
    name: &str,
    kind: ImportExportKind,
) -> ObjectId {
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let name_sid = vm.strings.intern(name);
    let kind_str = match kind {
        ImportExportKind::Function => "function",
        ImportExportKind::Table => "table",
        ImportExportKind::Memory => "memory",
        ImportExportKind::Global => "global",
        // ImportExportKind is `#[non_exhaustive]` per engine-bridge
        // module.rs to leave room for the WASM Exception Handling
        // proposal's `tag` variant (defer slot
        // `#11-wasm-exception-handling`).  Until that proposal's host
        // surface lands, any future variant surfacing here is
        // unreachable through `imports()`/`exports()` (the
        // engine-bridge filter drops it); map to `"unknown"` as
        // defensive fallback.
        _ => "unknown",
    };
    let kind_sid = vm.strings.intern(kind_str);
    let name_key_sid = vm.well_known.name;
    let kind_key_sid = vm.strings.intern("kind");
    // WebIDL §3.2.17 dictionary→JS: `CreateDataPropertyOrThrow` ({W, E, C})
    // for `ModuleExportDescriptor` / `ModuleImportDescriptor` members.
    // Required so `Object.keys(desc)` enumerates the dictionary members.
    vm.define_shaped_property(
        obj,
        PropertyKey::String(name_key_sid),
        PropertyValue::Data(JsValue::String(name_sid)),
        PropertyAttrs::DATA,
    );
    vm.define_shaped_property(
        obj,
        PropertyKey::String(kind_key_sid),
        PropertyValue::Data(JsValue::String(kind_sid)),
        PropertyAttrs::DATA,
    );
    obj
}
