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
//! `WebAssembly` ships via the `globals` bag, which gives it the
//! writable + configurable + enumerable shape that bag uses for all
//! native namespaces (`Math` / `JSON` share the same enumerable diff
//! vs the §5 IDL — `{enumerable: false}` requires a separate global-
//! object property descriptor walk and is deferred to a future polish
//! pass; see the in-fn comment near `globals.insert` for context).

use super::super::super::error::VmError;
use super::super::super::natives_promise::{create_promise, settle_promise};
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
};
use super::super::super::VmInner;
use super::super::text_encoding::extract_buffer_source_bytes;
use super::errors::wasm_error_to_js_value;
use super::instance::{native_wasm_instance_constructor, native_wasm_instance_exports_get};
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
    /// Per §5 IDL the namespace itself is writable + configurable +
    /// `{enumerable: false}`; we install via the `globals` bag (same
    /// path `Math` / `JSON` take in this VM) which is enumerable.  The
    /// enumerable diff is accepted in v1 — see the in-fn comment by
    /// `globals.insert` for the polish-pass deferral.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` / `error_prototype` is `None`
    /// (mis-ordered registration pass).
    #[allow(clippy::too_many_lines)] // one-shot registration: every prototype slot landed in this single pass
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
        let instantiate_sid = self.strings.intern("instantiate");
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
        self.install_native_method(
            namespace_id,
            instantiate_sid,
            native_wasm_instantiate,
            PropertyAttrs::METHOD,
        );

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

        // §5.2 Instance constructor + prototype.  Holds the
        // `exports` accessor (lazily-allocated wrapper-identity-stable
        // namespace per DR-4).  The accessor lives on the prototype
        // rather than as an own data property so `delete i.exports`
        // cannot break the `[[Exports]]` slot reading per WebIDL
        // `Instance` interface IDL.
        let instance_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.wasm_instance_prototype = Some(instance_proto);
        let exports_accessor_sid = self.strings.intern("exports");
        self.install_accessor_pair(
            instance_proto,
            exports_accessor_sid,
            native_wasm_instance_exports_get,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let instance_ctor =
            self.create_constructable_function("Instance", native_wasm_instance_constructor);
        self.define_shaped_property(
            instance_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(instance_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            instance_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(instance_ctor)),
            PropertyAttrs::BUILTIN,
        );
        let instance_name_sid = self.strings.intern("Instance");
        self.define_shaped_property(
            namespace_id,
            PropertyKey::String(instance_name_sid),
            PropertyValue::Data(JsValue::Object(instance_ctor)),
            PropertyAttrs::METHOD,
        );

        // §5.3 / §5.4 / §5.5 — Memory / Table / Global prototype
        // shells.  Stage 4 will install the ctors (`new
        // WebAssembly.Memory({initial})` etc.) on the namespace and
        // populate accessors (`.buffer` / `.grow` / `.length` / `.value`
        // / etc.).  Stage 3 needs the prototypes installed already
        // because the exports-namespace walker wraps each
        // `WasmExportItem::{Memory,Table,Global}` export with the
        // matching `*_prototype` ObjectId — having `None` here would
        // produce JS objects with no prototype chain, breaking even
        // basic property reads on exported instances.
        let memory_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.wasm_memory_prototype = Some(memory_proto);
        let table_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.wasm_table_prototype = Some(table_proto);
        let global_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.wasm_global_prototype = Some(global_proto);

        // §5.3 Memory accessors + ctor.
        let buffer_sid = self.strings.intern("buffer");
        self.install_accessor_pair(
            memory_proto,
            buffer_sid,
            super::memory::native_wasm_memory_buffer_get,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let grow_sid = self.strings.intern("grow");
        self.install_native_method(
            memory_proto,
            grow_sid,
            super::memory::native_wasm_memory_grow,
            PropertyAttrs::METHOD,
        );
        let memory_ctor = self
            .create_constructable_function("Memory", super::memory::native_wasm_memory_constructor);
        self.define_shaped_property(
            memory_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(memory_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            memory_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(memory_ctor)),
            PropertyAttrs::BUILTIN,
        );
        let memory_name_sid = self.strings.intern("Memory");
        self.define_shaped_property(
            namespace_id,
            PropertyKey::String(memory_name_sid),
            PropertyValue::Data(JsValue::Object(memory_ctor)),
            PropertyAttrs::METHOD,
        );

        // §5.4 Table accessors + ctor.
        let length_sid = self.strings.intern("length");
        self.install_accessor_pair(
            table_proto,
            length_sid,
            super::table::native_wasm_table_length_get,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let get_sid = self.strings.intern("get");
        let set_sid = self.strings.intern("set");
        let table_grow_sid = self.strings.intern("grow");
        self.install_native_method(
            table_proto,
            get_sid,
            super::table::native_wasm_table_get,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            table_proto,
            set_sid,
            super::table::native_wasm_table_set,
            PropertyAttrs::METHOD,
        );
        self.install_native_method(
            table_proto,
            table_grow_sid,
            super::table::native_wasm_table_grow,
            PropertyAttrs::METHOD,
        );
        let table_ctor = self
            .create_constructable_function("Table", super::table::native_wasm_table_constructor);
        self.define_shaped_property(
            table_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(table_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            table_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(table_ctor)),
            PropertyAttrs::BUILTIN,
        );
        let table_name_sid = self.strings.intern("Table");
        self.define_shaped_property(
            namespace_id,
            PropertyKey::String(table_name_sid),
            PropertyValue::Data(JsValue::Object(table_ctor)),
            PropertyAttrs::METHOD,
        );

        // §5.5 Global accessors + valueOf + ctor.
        let value_sid = self.strings.intern("value");
        self.install_accessor_pair(
            global_proto,
            value_sid,
            super::global::native_wasm_global_value_get,
            Some(super::global::native_wasm_global_value_set),
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let value_of_sid = self.strings.intern("valueOf");
        self.install_native_method(
            global_proto,
            value_of_sid,
            super::global::native_wasm_global_value_of,
            PropertyAttrs::METHOD,
        );
        let global_ctor = self
            .create_constructable_function("Global", super::global::native_wasm_global_constructor);
        self.define_shaped_property(
            global_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(global_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            global_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(global_ctor)),
            PropertyAttrs::BUILTIN,
        );
        let global_name_sid = self.strings.intern("Global");
        self.define_shaped_property(
            namespace_id,
            PropertyKey::String(global_name_sid),
            PropertyValue::Data(JsValue::Object(global_ctor)),
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
            // per WebIDL §3.2.26 Buffer source types — `compile`
            // rejects with the
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

/// `WebAssembly.instantiate(...)` — WASM JS API §5 namespace, two
/// overloads:
///
/// 1. `instantiate(bytes, importObject?)` → Promise<{module,
///    instance}>: compile-then-instantiate combined.  Resolves with a
///    `WebAssemblyInstantiatedSource` dict (`{module: WebAssembly.Module,
///    instance: WebAssembly.Instance}`).  Rejects with `CompileError`
///    on parse/compile failure, `LinkError` on import resolution
///    failure, `RuntimeError` on initialisation trap.
/// 2. `instantiate(Module, importObject?)` → Promise<Instance>:
///    instantiate-only.  Resolves with a `WebAssembly.Instance`.
///
/// The overload split is by `args[0]` brand check:
/// `ObjectKind::WasmModule` → overload 2, anything else → overload 1
/// (where overload 1's bytes-coerce surfaces TypeError for non-
/// BufferSource arguments per WebIDL §3.2.26 Buffer source types).
///
/// `_options` parameter is currently ignored — the WebIDL surface
/// defines no observable behaviour for it pre-`builtins` proposal,
/// per `body wasm-js-api-2 dom-webassembly-instantiate` step 2 +
/// the IDL `optional dictionary options`.  Spec-correct as-is; a
/// future `String Builtins` proposal would ship as a separate
/// `wasm-js-api-2-fork-builtins` spec fork and gets its own slot
/// at proposal-stabilization time (no D-16 pre-emptive slot).
pub(super) fn native_wasm_instantiate(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let promise = create_promise(ctx.vm);
    let first_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let import_object_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    // Overload split — Module instance vs bytes.
    let module_id_for_overload2 = if let JsValue::Object(id) = first_arg {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmModule) {
            Some(id)
        } else {
            None
        }
    } else {
        None
    };

    let module_id = if let Some(id) = module_id_for_overload2 {
        id
    } else {
        // Overload 1 — bytes path.  Coerce BufferSource, compile, then
        // proceed to instantiate.
        let bytes = match extract_buffer_source_bytes(
            ctx,
            first_arg,
            "Failed to execute 'instantiate' on 'WebAssembly'",
            1,
            false,
        ) {
            Ok(b) => b,
            Err(e) => {
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
        let module = match runtime.compile(&bytes) {
            Ok(m) => m,
            Err(e) => {
                let reason = wasm_error_to_js_value(ctx, &e);
                let _ = settle_promise(ctx.vm, promise, true, reason);
                return Ok(JsValue::Object(promise));
            }
        };
        let proto = ctx
            .vm
            .wasm_module_prototype
            .expect("wasm_module_prototype populated in register_wasm_namespace");
        let id = ctx.vm.alloc_object(Object {
            kind: ObjectKind::WasmModule,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });
        ctx.vm.wasm_module_storage.insert(
            id,
            super::super::super::wasm_payload::WasmModulePayload { module },
        );
        id
    };

    // Instantiate.  Shared between both overloads.
    let instance_id =
        match super::instance::instantiate_module(ctx, module_id, import_object_arg, None) {
            Ok(id) => id,
            Err(reason) => {
                let _ = settle_promise(ctx.vm, promise, true, reason);
                return Ok(JsValue::Object(promise));
            }
        };

    // Overload 1 resolves with `{module, instance}` dict; overload 2
    // resolves with `Instance` directly.
    let resolution_value = if module_id_for_overload2.is_some() {
        JsValue::Object(instance_id)
    } else {
        let dict = ctx.vm.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: ctx.vm.object_prototype,
            extensible: true,
        });
        let module_key_sid = ctx.vm.strings.intern("module");
        let instance_key_sid = ctx.vm.strings.intern("instance");
        ctx.vm.define_shaped_property(
            dict,
            PropertyKey::String(module_key_sid),
            PropertyValue::Data(JsValue::Object(module_id)),
            PropertyAttrs::DATA,
        );
        ctx.vm.define_shaped_property(
            dict,
            PropertyKey::String(instance_key_sid),
            PropertyValue::Data(JsValue::Object(instance_id)),
            PropertyAttrs::DATA,
        );
        JsValue::Object(dict)
    };
    let _ = settle_promise(ctx.vm, promise, false, resolution_value);
    Ok(JsValue::Object(promise))
}
