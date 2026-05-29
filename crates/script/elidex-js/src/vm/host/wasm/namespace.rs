//! `WebAssembly` namespace install + `validate` / `compile` static
//! methods — slot `#11-wasm-vm` / D-16, plan-memo §5 Stage 2.1 + 2.4.
//!
//! Per WASM JS API §5 the namespace is a plain object (not
//! constructable) installed on `globalThis.WebAssembly` with 3 static
//! methods (`validate` / `compile` / `instantiate`) + 5 class ctors
//! (`Module` / `Instance` / `Memory` / `Table` / `Global`) + 3 error
//! classes (`CompileError` / `LinkError` / `RuntimeError`).
//!
//! Stage 2 ships the namespace shell + `Module` ctor + 3 error
//! classes + `validate` + `compile` (sync wrapper resolving a
//! Promise).  Stage 3 wires `instantiate` + `Instance` ctor + exports
//! exotic.  Stage 4 wires `Memory` / `Table` / `Global` ctors.
//!
//! `WebAssembly` is writable + configurable but NOT enumerable per
//! §5 namespace IDL.

use super::super::super::error::VmError;
use super::super::super::natives_promise::{create_promise, settle_promise};
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue,
};
use super::super::super::VmInner;
use super::super::text_encoding::extract_buffer_source_bytes;
use super::errors::wasm_error_to_js_value;
use super::module::{
    native_wasm_module_constructor, native_wasm_module_custom_sections, native_wasm_module_exports,
    native_wasm_module_imports,
};

impl VmInner {
    /// Install the `WebAssembly` namespace on `globalThis` (slot
    /// `#11-wasm-vm` / D-16, plan-memo §5 Stage 2).
    ///
    /// Allocates the namespace as a plain `ObjectKind::Ordinary`
    /// chained to `Object.prototype`, populates it with the 3 static
    /// methods + `Module` constructor + 3 error class constructors,
    /// then exposes on `globals["WebAssembly"]`.
    ///
    /// Runs during [`Self::register_globals`] after both:
    /// - `register_error_constructors` (for `error_prototype` — wasm
    ///   error subclasses chain to it)
    /// - `register_array_buffer_global` (for `extract_buffer_source_bytes`
    ///   detached-check reachable at Module ctor)
    ///
    /// Per §5 IDL the namespace itself is writable + configurable but
    /// NOT enumerable — matches how `Math` / `JSON` are installed.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` / `error_prototype` is `None`
    /// (mis-ordered registration pass).
    pub(in crate::vm) fn register_wasm_namespace(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_wasm_namespace called before register_prototypes");

        let namespace_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // §5 namespace 3 static methods.
        let validate_sid = self.strings.intern("validate");
        let compile_sid = self.strings.intern("compile");
        self.install_native_method(
            namespace_id,
            validate_sid,
            native_wasm_validate,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            namespace_id,
            compile_sid,
            native_wasm_compile,
            PropertyAttrs::METHOD,
        );
        // `instantiate` is wired in Stage 3 alongside `Instance` ctor.

        // §5.1 Module constructor + prototype.
        let module_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.wasm_module_prototype = Some(module_proto);
        let module_ctor =
            self.create_constructable_function("Module", native_wasm_module_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            module_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(module_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            module_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(module_ctor)),
            PropertyAttrs::BUILTIN,
        );
        // §5.1 Module 3 static methods on the ctor (not the prototype).
        let exports_sid = self.strings.intern("exports");
        let imports_sid = self.strings.intern("imports");
        let custom_sections_sid = self.strings.intern("customSections");
        self.install_native_method(
            module_ctor,
            exports_sid,
            native_wasm_module_exports,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            module_ctor,
            imports_sid,
            native_wasm_module_imports,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            module_ctor,
            custom_sections_sid,
            native_wasm_module_custom_sections,
            PropertyAttrs::METHOD,
        );
        // Expose `WebAssembly.Module` on the namespace.
        let module_name_sid = self.strings.intern("Module");
        self.define_shaped_property(
            namespace_id,
            PropertyKey::String(module_name_sid),
            PropertyValue::Data(JsValue::Object(module_ctor)),
            PropertyAttrs::METHOD,
        );

        // §5.10 — install `CompileError` / `LinkError` / `RuntimeError`
        // on the namespace + populate `wasm_*_error_prototype` slots.
        self.install_wasm_error_classes(namespace_id);

        // §5 IDL — WebAssembly namespace is writable + configurable but
        // NOT enumerable.  `globals` is the writable-configurable
        // enumerable bag; for an enumerable=false attr we'd need to
        // walk the global-object property descriptor.  Most native
        // namespaces (Math / JSON) land via `globals` and accept the
        // enumerable diff in v1 (deferred to a future polish pass —
        // matches boa parity at `register_wasm`).
        let webassembly_sid = self.strings.intern("WebAssembly");
        self.globals
            .insert(webassembly_sid, JsValue::Object(namespace_id));
    }
}

/// `WebAssembly.validate(bytes, options?)` — WASM JS API §5 namespace.
///
/// Synchronous; returns `boolean` (does NOT throw on parse failure
/// per IDL `bool` return type).  Delegates to engine-bridge
/// [`elidex_wasm_runtime::WasmRuntime::validate`].
///
/// Detached-buffer rejection at the WebIDL conversion boundary is
/// handled by [`extract_buffer_source_bytes`] (slot
/// `#11-array-buffer-detach-state` / F3); a detached source is the
/// only path that surfaces a TypeError — actual validation failure
/// is just `false`.
pub(super) fn native_wasm_validate(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bytes_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let bytes = extract_buffer_source_bytes(
        ctx,
        bytes_arg,
        "Failed to execute 'validate' on 'WebAssembly'",
        1,
        false,
    )?;
    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(super::errors::wasm_error_to_vm_error(ctx, &e)),
    };
    Ok(JsValue::Boolean(runtime.validate(&bytes)))
}

/// `WebAssembly.compile(bytes, options?)` — WASM JS API §5 namespace.
///
/// Async — returns `Promise<Module>`.  Resolves with a new
/// `WebAssembly.Module` (per §5.1 ctor algorithm) or rejects with
/// `CompileError` (§5.1 step 3).
///
/// Per the spec the compile is asynchronous; elidex compiles
/// synchronously and settles the Promise immediately (the resolved /
/// rejected reactions still drain through the microtask queue, so
/// observable async semantics are preserved).  Boa parity at
/// `register_wasm::wasm_compile`.
pub(super) fn native_wasm_compile(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let promise = create_promise(ctx.vm);
    let bytes_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let bytes = match extract_buffer_source_bytes(
        ctx,
        bytes_arg,
        "Failed to execute 'compile' on 'WebAssembly'",
        1,
        false,
    ) {
        Ok(b) => b,
        Err(e) => {
            // Pre-spec BufferSource conversion failures are TypeErrors
            // per WebIDL §3.2.21 — `compile` rejects with the
            // converted exception value (does NOT wrap in CompileError).
            let reason = ctx.vm.vm_error_to_thrown(&e);
            let _ = settle_promise(ctx.vm, promise, true, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => {
            let reason = wasm_error_to_js_value(ctx, &e);
            let _ = settle_promise(ctx.vm, promise, true, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    match runtime.compile(&bytes) {
        Ok(module) => {
            let proto = ctx
                .vm
                .wasm_module_prototype
                .expect("wasm_module_prototype populated in register_wasm_namespace");
            let module_id = ctx.vm.alloc_object(Object {
                kind: ObjectKind::WasmModule,
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: Some(proto),
                extensible: true,
            });
            ctx.vm.wasm_module_storage.insert(
                module_id,
                super::super::super::wasm_payload::WasmModulePayload { module },
            );
            let _ = settle_promise(ctx.vm, promise, false, JsValue::Object(module_id));
        }
        Err(e) => {
            let reason = wasm_error_to_js_value(ctx, &e);
            let _ = settle_promise(ctx.vm, promise, true, reason);
        }
    }
    Ok(JsValue::Object(promise))
}

// Suppress unused-import warning until Stage 3 lands `instantiate`.
#[allow(dead_code)]
fn _unused_object_id(_: ObjectId) {}
