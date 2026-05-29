//! `WebAssembly.CompileError` / `LinkError` / `RuntimeError` install
//! plus the [`wasm_error_to_js_value`] marshalling helper —
//! slot `#11-wasm-vm` / D-16, plan-memo §5 Stage 2.2.
//!
//! Per WASM JS API §5.10 "Error Objects", the 3 classes are
//! Error-subclass constructors with `name` / `message` / `stack` —
//! `instanceof Error` holds because each prototype chains to
//! `Error.prototype`.  §5.10 only enumerates the class set; the
//! cause→class mapping is:
//!
//! - `CompileError`: §5.1 step 3 ("If module is error, throw a
//!   CompileError exception") + §5.2 validate inner step 3 (when
//!   the prose flags compile failure)
//! - `LinkError`: `instantiate the core of a WebAssembly module`
//!   inner step 3 + setup-time link failure
//! - `RuntimeError`: §7.1 stack overflow (impl-defined per spec; elidex
//!   convention) + §7.2 OOM (impl-defined; `RangeError` is
//!   spec-prescribed only for `Memory.grow` / `Table.grow`
//!   algorithm-internal failures) + trap mapping
//!
//! See plan-memo §3 / §6 for the full per-row spec-prose citations
//! and `feedback_helper-prefer-upstream-machine-readable.md` for the
//! `webref body wasm-js-api-2 <anchor>` verification discipline.

use elidex_wasm_runtime::{WasmError, WasmErrorKind};

use super::super::super::error::VmError;
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue,
};
use super::super::super::VmInner;

impl VmInner {
    /// Install the 3 WebAssembly error classes (`CompileError` /
    /// `LinkError` / `RuntimeError`) as Error-subclass constructors
    /// on the supplied `wasm_namespace` plain object (WASM JS API
    /// §5.10).  Each prototype chains to
    /// [`Self::error_prototype`] so `instanceof Error` holds; the
    /// constructor ctor.prototype links back to the wasm prototype
    /// for `new WebAssembly.CompileError(msg) instanceof
    /// WebAssembly.CompileError`.
    ///
    /// The prototype slots
    /// ([`Self::wasm_compile_error_prototype`] etc.) are populated
    /// so [`wasm_error_to_js_value`] can build instances at runtime
    /// without re-resolving the namespace.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::error_prototype`] is `None` — indicates a
    /// mis-ordered registration pass (must run after
    /// `register_error_constructors`).
    pub(in crate::vm) fn install_wasm_error_classes(&mut self, wasm_namespace: ObjectId) {
        let error_proto = self
            .error_prototype
            .expect("install_wasm_error_classes called before register_error_constructors");

        let compile_proto = install_wasm_error_class(
            self,
            wasm_namespace,
            "CompileError",
            error_proto,
            super::super::super::natives::native_wasm_compile_error_constructor,
        );
        self.wasm_compile_error_prototype = Some(compile_proto);

        let link_proto = install_wasm_error_class(
            self,
            wasm_namespace,
            "LinkError",
            error_proto,
            super::super::super::natives::native_wasm_link_error_constructor,
        );
        self.wasm_link_error_prototype = Some(link_proto);

        let runtime_proto = install_wasm_error_class(
            self,
            wasm_namespace,
            "RuntimeError",
            error_proto,
            super::super::super::natives::native_wasm_runtime_error_constructor,
        );
        self.wasm_runtime_error_prototype = Some(runtime_proto);
    }
}

/// Build a `WebAssembly.<Name>` Error-subclass constructor + prototype
/// pair, install on `wasm_namespace[name]`, and return the prototype
/// `ObjectId`.
///
/// `name` is stored on the prototype as a non-enumerable data property
/// (`{W, ¬E, C}` per §20.5.3.2 + mirrored at WASM JS API §5.10) so
/// `String(new WebAssembly.CompileError("m"))` produces
/// `"CompileError: m"` via the inherited `Error.prototype.toString`.
fn install_wasm_error_class(
    vm: &mut VmInner,
    wasm_namespace: ObjectId,
    name: &'static str,
    error_proto: ObjectId,
    ctor_fn: super::super::super::NativeFn,
) -> ObjectId {
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(error_proto),
        extensible: true,
    });
    let name_sid = vm.strings.intern(name);
    let empty_sid = vm.well_known.empty;
    let name_key = PropertyKey::String(vm.well_known.name);
    vm.define_shaped_property(
        proto_id,
        name_key,
        PropertyValue::Data(JsValue::String(name_sid)),
        PropertyAttrs::METHOD,
    );
    let message_key = PropertyKey::String(vm.well_known.message);
    vm.define_shaped_property(
        proto_id,
        message_key,
        PropertyValue::Data(JsValue::String(empty_sid)),
        PropertyAttrs::METHOD,
    );

    let ctor_id = vm.create_constructable_function(name, ctor_fn);
    let proto_key = PropertyKey::String(vm.well_known.prototype);
    vm.define_shaped_property(
        ctor_id,
        proto_key,
        PropertyValue::Data(JsValue::Object(proto_id)),
        PropertyAttrs::BUILTIN,
    );
    let ctor_key = PropertyKey::String(vm.well_known.constructor);
    vm.define_shaped_property(
        proto_id,
        ctor_key,
        PropertyValue::Data(JsValue::Object(ctor_id)),
        PropertyAttrs::BUILTIN,
    );

    vm.define_shaped_property(
        wasm_namespace,
        PropertyKey::String(name_sid),
        PropertyValue::Data(JsValue::Object(ctor_id)),
        PropertyAttrs::METHOD,
    );

    proto_id
}

/// Marshal a [`WasmError`] from the engine-bridge layer into a JS
/// `WebAssembly.<class>` instance per WASM JS API §5.10 + the
/// elidex impl-defined trap-class convention (§7.1 stack-overflow /
/// §7.2 runtime OOM both map to `RuntimeError`).
///
/// Returns the constructed Error instance as a `JsValue::Object`.
/// The instance carries an own non-enumerable `message` data property
/// matching [`WasmError::message`]; `name` is inherited from the
/// matching prototype (`WebAssembly.CompileError.prototype.name` etc.).
///
/// # Spec-prose anchors
///
/// - `Compile`: WASM JS API §5.1 step 3 (`If module is error, throw
///   a CompileError exception`, anchor `#dom-module-module`)
/// - `Link`: WASM JS API §5.2 `instantiate the core of a WebAssembly
///   module` inner step 3 (anchor
///   `#instantiate-the-core-of-a-webassembly-module`)
/// - `Runtime`: WASM JS API §7.1 stack overflow (`#stack-overflow`,
///   impl-defined exception class) + §7.2 OOM (`#out-of-memory`) +
///   trap mapping (`§5.6 Exported Functions` trap conversion)
///
/// `WasmErrorKind` is `#[non_exhaustive]` per F1 R11 F12 — the
/// catch-all arm covers future-proposal kinds (e.g. Exception Handling
/// proposal `WasmException`) by mapping to `RuntimeError` until the
/// matching JS-side surface is added (defer slot
/// `#11-wasm-exception-handling`).
pub(crate) fn wasm_error_to_js_value(ctx: &mut NativeContext<'_>, err: &WasmError) -> JsValue {
    // `WasmErrorKind` is `#[non_exhaustive]` per F1 R11 F12 — future
    // proposal kinds (Exception Handling `WasmException` etc.) fall
    // through to `RuntimeError` until the matching JS-side surface
    // lands per `#11-wasm-exception-handling` defer slot.  Boa parity
    // at `elidex-js-boa/src/globals/wasm.rs::wasm_error_to_js`.
    let proto = match err.kind() {
        WasmErrorKind::Compile => ctx.vm.wasm_compile_error_prototype,
        WasmErrorKind::Link => ctx.vm.wasm_link_error_prototype,
        WasmErrorKind::Runtime | _ => ctx.vm.wasm_runtime_error_prototype,
    };
    let proto = proto.expect(
        "wasm error prototypes populated by install_wasm_error_classes during register_globals",
    );
    let instance = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    let message_sid = ctx.intern(err.message());
    let message_key = PropertyKey::String(ctx.vm.well_known.message);
    ctx.vm.define_shaped_property(
        instance,
        message_key,
        PropertyValue::Data(JsValue::String(message_sid)),
        PropertyAttrs::METHOD,
    );
    JsValue::Object(instance)
}

/// Convenience: build a [`VmError::throw`] from a [`WasmError`] —
/// hands the caller a `Result<_, VmError>` that propagates the
/// constructed instance up the native call stack.
pub(crate) fn wasm_error_to_vm_error(ctx: &mut NativeContext<'_>, err: &WasmError) -> VmError {
    VmError::throw(wasm_error_to_js_value(ctx, err))
}
