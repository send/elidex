//! `WasmRuntime` — engine + linker template + compile / validate /
//! instantiate facade.
//!
//! Holds a single `Engine` and a `linker_template` (populated once with
//! the DOM/CSSOM host functions). Each `instantiate` call clones the
//! linker template (cheap — wasmtime `Linker: Clone` shares the
//! per-host-fn entries via internal `Arc`), adds the per-call user
//! imports, then dispatches `wasmtime::Linker::instantiate`. This
//! eliminates cross-instance import leak by construction: user imports
//! added by instance A never reach instance B because B's `instantiate`
//! starts from a fresh clone (per plan §2 D-6).
//!
//! Spec anchors:
//! - WASM JS API §5.1 Module ctor + §5 `validate(bytes, options)` anchor
//!   `#dom-webassembly-validate`
//! - WASM JS API §5.2 Instance ctor algorithm steps 1-6 + step 4
//!   ("Read the imports")

use std::sync::Arc;

use elidex_dom_api::registry::{
    create_cssom_registry, create_dom_registry, CssomHandlerRegistry, DomHandlerRegistry,
};
use wasmtime::{Config, Engine, Linker, Module, Store};

use crate::engine_conv::{
    import_value_to_extern, wasm_error_from_wasmtime, wasm_ref_to_wasmtime, wasm_value_to_wasmtime,
    wasmtime_val_type_from,
};
use crate::error::{WasmError, WasmErrorKind};
use crate::handle::{WasmGlobal, WasmMemory, WasmStoreHandle, WasmTable};
use crate::host::funcs::register_host_functions;
use crate::host::state::HostState;
use crate::imports::ImportObject;
use crate::instance::WasmInstance;
use crate::module::WasmModule;
use crate::value::{
    WasmGlobalDescriptor, WasmMemoryDescriptor, WasmRef, WasmTableDescriptor, WasmValue,
    WasmValueType,
};

/// Default fuel budget — 1 billion instructions is roughly a few
/// seconds on modern hardware, guarding against runaway wasm modules
/// during host dispatch. The store is seeded with this budget at
/// `instantiate` time, and `WasmFunc::call` resets the budget to this
/// value at the start of every call so a single long-running call
/// cannot deplete the budget for subsequent calls on the same instance.
/// Exposed `pub(crate)` so `handle.rs` can perform the per-call reset.
pub(crate) const DEFAULT_FUEL: u64 = 1_000_000_000;

/// Compiles and instantiates wasm modules with DOM host functions.
///
/// Per plan §2 D-6: holds a `linker_template` that is `Clone`d on each
/// `instantiate` call. User imports are added to the clone, never to
/// the original — instance B cannot inherit instance A's user imports.
pub struct WasmRuntime {
    engine: Engine,
    linker_template: Linker<HostState>,
    dom_registry: Arc<DomHandlerRegistry>,
    cssom_registry: Arc<CssomHandlerRegistry>,
}

impl WasmRuntime {
    /// Create a runtime with default DOM/CSSOM registries.
    pub fn new() -> Result<Self, WasmError> {
        let dom_registry = Arc::new(create_dom_registry());
        let cssom_registry = Arc::new(create_cssom_registry());
        Self::with_registries(dom_registry, cssom_registry)
    }

    /// Create a runtime with custom registries.
    pub fn with_registries(
        dom_registry: Arc<DomHandlerRegistry>,
        cssom_registry: Arc<CssomHandlerRegistry>,
    ) -> Result<Self, WasmError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Compile))?;
        let mut linker_template = Linker::new(&engine);
        register_host_functions(&mut linker_template)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Link))?;
        Ok(Self {
            engine,
            linker_template,
            dom_registry,
            cssom_registry,
        })
    }

    /// Per WASM JS API §5.1 `Module` ctor algorithm — compile wasm
    /// bytes into a reusable `WasmModule`.
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<WasmModule, WasmError> {
        let inner = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Compile))?;
        Ok(WasmModule {
            inner,
            source_bytes: Arc::from(wasm_bytes),
        })
    }

    /// Per WASM JS API §5 `validate(bytes, options)` anchor
    /// `#dom-webassembly-validate` — bytecode validation without
    /// compilation side effects.
    pub fn validate(&self, wasm_bytes: &[u8]) -> bool {
        Module::validate(&self.engine, wasm_bytes).is_ok()
    }

    /// Per WASM JS API §5.2 Instance ctor algorithm steps 1-6 + step 4
    /// "Read the imports". `imports` is the engine-indep
    /// record-of-records (`HashMap<module-name, HashMap<name, value>>`,
    /// see `imports.rs`); callers populate it via `ImportObject::define`
    /// or pass `ImportObject::default()` for the empty-imports case.
    /// Single canonical form — no dual API for empty vs non-empty.
    pub fn instantiate(
        &self,
        module: &WasmModule,
        imports: &ImportObject,
    ) -> Result<WasmInstance, WasmError> {
        // Deferred to `#11-wasm-user-import-host-fn-builder` (D-16
        // surface): every `instantiate` creates a fresh `Store`, and
        // `WasmImportValue::{Func,Memory,Table,Global}` handles are
        // store-tied — passing non-empty imports today guarantees a
        // cross-store mismatch at `Linker::define`, surfacing as a
        // confusing wasmtime error. Fail fast with a clear `LinkError`
        // until the host-fn-builder slot lands the shared-store wiring
        // (per WASM JS API §4.1 "Interaction of the WebAssembly Store
        // with JavaScript" — each agent has an associated store, and
        // cross-store import handles violate the per-agent invariant).
        if !imports.is_empty() {
            return Err(WasmError::new(
                WasmErrorKind::Link,
                "non-empty ImportObject not yet supported \
                 (cross-store import handles deferred to \
                 #11-wasm-user-import-host-fn-builder, D-16 surface)"
                    .to_string(),
            ));
        }

        let host_state = HostState::new(self.dom_registry.clone(), self.cssom_registry.clone());
        let mut store = Store::new(&self.engine, host_state);
        // Initial fuel for the new store. This runs at instantiate
        // time, not during wasm execution — surface any error as
        // `Link` (instance-setup failure) rather than `Runtime`
        // (which is reserved for wasm-level traps).
        store
            .set_fuel(DEFAULT_FUEL)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Link))?;

        // Fresh linker per call — user imports added here cannot leak
        // to other instances. `Linker::clone()` is cheap (shares the
        // host-fn entries via internal Arc per wasmtime API).
        let mut linker = self.linker_template.clone();
        for (module_name, name, value) in imports.iter() {
            let ext = import_value_to_extern(value.clone());
            linker
                .define(&mut store, module_name, name, ext)
                .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Link))?;
        }

        let inst = linker
            .instantiate(&mut store, &module.inner)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Link))?;

        let store_handle = WasmStoreHandle::new(store);
        Ok(WasmInstance::new(inst, store_handle))
    }

    /// Standalone Memory ctor per WASM JS API §5.3 Memory ctor algorithm
    /// — engine-side portion (`#11-wasm-user-import-host-fn-builder`,
    /// D-16 surface, wraps this JS-side). Allocates a fresh per-handle
    /// `Store<HostState>` (mirrors `instantiate` precedent in this file)
    /// so standalone Memory/Table/Global handles are store-isolated from
    /// each other and from any instance. Per WASM JS API §4.1
    /// "Interaction of the WebAssembly Store with JavaScript" the spec
    /// model is one associated store per agent; per-handle store
    /// isolation is an elidex engine-bridge implementation choice (not a
    /// spec-mandated per-item slot) that maps each `WasmStoreHandle`
    /// into its own associated store until the host-fn-builder lands
    /// shared-store wiring.
    pub fn new_memory(&self, desc: WasmMemoryDescriptor) -> Result<WasmMemory, WasmError> {
        let host_state = HostState::new(self.dom_registry.clone(), self.cssom_registry.clone());
        let mut store = Store::new(&self.engine, host_state);
        // `wasmtime::MemoryType::new` panics if `min > max` (or addressable-
        // size overflow) — its doc explicitly says so. Per WASM JS API §5.3
        // Memory(descriptor) ctor step 5 (RangeError when memtype is not
        // valid) / step 7 (RangeError on allocation failure) those
        // conditions must surface as `RangeError`, not abort the process.
        // `MemoryTypeBuilder::build` returns `Result` for the same
        // validations, so we route through the builder + propagate.
        let ty = wasmtime::MemoryTypeBuilder::default()
            .min(u64::from(desc.initial))
            .max(desc.maximum.map(u64::from))
            .build()
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        let inner = wasmtime::Memory::new(&mut store, ty)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        let store_handle = WasmStoreHandle::new(store);
        Ok(WasmMemory::from_parts(inner, store_handle))
    }

    /// Standalone Table ctor per WASM JS API §5.4 Table ctor algorithm —
    /// engine-side portion (`#11-wasm-user-import-host-fn-builder`, D-16
    /// surface, wraps this JS-side). `init` is the
    /// funcref/externref/typed-null used to fill the initial entries
    /// (the D-16 host applies the §5.4 step 8-9 default selection when
    /// the JS-side `value` parameter is absent).
    pub fn new_table(
        &self,
        desc: WasmTableDescriptor,
        init: WasmRef,
    ) -> Result<WasmTable, WasmError> {
        let host_state = HostState::new(self.dom_registry.clone(), self.cssom_registry.clone());
        let mut store = Store::new(&self.engine, host_state);
        let wasmtime::ValType::Ref(element_ty) =
            wasmtime_val_type_from(WasmValueType::Ref(desc.element))
        else {
            unreachable!("WasmValueType::Ref converts to wasmtime::ValType::Ref")
        };
        let ty = wasmtime::TableType::new(element_ty, desc.initial, desc.maximum);
        let init_ref = wasm_ref_to_wasmtime(init, &mut store)?;
        let inner = wasmtime::Table::new(&mut store, ty, init_ref)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        let store_handle = WasmStoreHandle::new(store);
        Ok(WasmTable {
            inner,
            store: store_handle,
        })
    }

    /// Standalone Global ctor per WASM JS API §5.5 Global ctor algorithm
    /// — engine-side portion (`#11-wasm-user-import-host-fn-builder`,
    /// D-16 surface, wraps this JS-side). The D-16 host is responsible
    /// for the §5.5 step 3 `v128` / `exnref` rejection at the JS
    /// boundary; the engine here simply forwards `desc.value_type` to
    /// wasmtime and lets the linker reject any type↔init mismatch.
    pub fn new_global(
        &self,
        desc: WasmGlobalDescriptor,
        init: WasmValue,
    ) -> Result<WasmGlobal, WasmError> {
        let host_state = HostState::new(self.dom_registry.clone(), self.cssom_registry.clone());
        let mut store = Store::new(&self.engine, host_state);
        let val_ty = wasmtime_val_type_from(desc.value_type);
        let mutability = if desc.mutable {
            wasmtime::Mutability::Var
        } else {
            wasmtime::Mutability::Const
        };
        let ty = wasmtime::GlobalType::new(val_ty, mutability);
        let init_val = wasm_value_to_wasmtime(init, &mut store)?;
        let inner = wasmtime::Global::new(&mut store, ty, init_val)
            .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        let store_handle = WasmStoreHandle::new(store);
        Ok(WasmGlobal {
            inner,
            store: store_handle,
        })
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("failed to create default WasmRuntime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::ScriptHostBinding;
    use crate::value::WasmValue;
    use elidex_ecs::{EcsDom, Entity};
    use elidex_script_session::SessionCore;

    /// Test fixture: runtime + session + dom + document root.
    struct TestEnv {
        runtime: WasmRuntime,
        session: SessionCore,
        dom: EcsDom,
        doc: Entity,
    }

    impl TestEnv {
        fn new() -> Self {
            let runtime = WasmRuntime::new().unwrap();
            let mut dom = EcsDom::new();
            let doc = dom.create_document_root();
            Self {
                runtime,
                session: SessionCore::new(),
                dom,
                doc,
            }
        }

        fn compile_and_instantiate(&self, wat_src: &str) -> WasmInstance {
            let wasm = wat::parse_str(wat_src).unwrap();
            let module = self.runtime.compile(&wasm).unwrap();
            self.runtime
                .instantiate(&module, &ImportObject::default())
                .unwrap()
        }

        fn call(
            &mut self,
            instance: &WasmInstance,
            name: &str,
            args: &[WasmValue],
        ) -> Result<Vec<WasmValue>, WasmError> {
            let func = instance.get_func(name).expect("export not found");
            let bridge = ScriptHostBinding {
                session: &mut self.session,
                dom: &mut self.dom,
                document: self.doc,
            };
            func.call(args, bridge)
        }
    }

    /// WAT fragment: bump allocator starting at offset 1024.
    const WAT_BUMP_ALLOC: &str = r#"
                (global $alloc_ptr (mut i32) (i32.const 1024))
                (func (export "__alloc") (param $len i32) (result i32)
                    (local $ptr i32)
                    (local.set $ptr (global.get $alloc_ptr))
                    (global.set $alloc_ptr (i32.add (global.get $alloc_ptr) (local.get $len)))
                    (local.get $ptr)
                )"#;

    /// Unpack a host-returned packed string `(ptr << 32) | len`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn unpack_wasm_string(packed: i64) -> (usize, usize) {
        let ptr = ((packed >> 32) & 0xFFFF_FFFF) as usize;
        let len = (packed & 0xFFFF_FFFF) as usize;
        (ptr, len)
    }

    /// Read a packed string from the instance's exported memory.
    fn read_wasm_string(instance: &WasmInstance, packed: i64) -> String {
        let (ptr, len) = unpack_wasm_string(packed);
        let mem = instance.get_memory("memory").unwrap();
        mem.with_data(|data| String::from_utf8(data[ptr..ptr + len].to_vec()).unwrap())
    }

    fn expect_i64(v: &WasmValue) -> i64 {
        match v {
            WasmValue::I64(x) => *x,
            _ => panic!("expected WasmValue::I64, got {v:?}"),
        }
    }

    #[test]
    fn compile_minimal_module() {
        let env = TestEnv::new();
        let wasm = wat::parse_str("(module)").unwrap();
        let _module = env.runtime.compile(&wasm).unwrap();
    }

    #[test]
    fn instantiate_minimal_module() {
        let env = TestEnv::new();
        let _instance = env.compile_and_instantiate("(module)");
    }

    #[test]
    fn call_get_document() {
        let mut env = TestEnv::new();
        let instance = env.compile_and_instantiate(
            r#"(module
                (import "elidex" "get_document" (func $get_doc (result i64)))
                (func (export "test_get_doc") (result i64)
                    call $get_doc
                )
            )"#,
        );

        let results = env.call(&instance, "test_get_doc", &[]).unwrap();

        #[allow(clippy::cast_possible_wrap)]
        let expected = env.doc.to_bits().get() as i64;
        assert_eq!(results.len(), 1);
        assert_eq!(expect_i64(&results[0]), expected);
    }

    #[test]
    fn create_element_and_append() {
        let mut env = TestEnv::new();
        let instance = env.compile_and_instantiate(
            r#"(module
                (import "elidex" "get_document" (func $get_doc (result i64)))
                (import "elidex" "create_element" (func $create_elem (param i32 i32) (result i64)))
                (import "elidex" "append_child" (func $append (param i64 i64)))

                (memory (export "memory") 1)
                (data (i32.const 0) "div")

                (func (export "test") (result i64)
                    (local $doc i64)
                    (local $div i64)
                    (local.set $doc (call $get_doc))
                    (local.set $div (call $create_elem (i32.const 0) (i32.const 3)))
                    (call $append (local.get $doc) (local.get $div))
                    (local.get $div)
                )
            )"#,
        );

        let results = env.call(&instance, "test", &[]).unwrap();

        let div_i64 = expect_i64(&results[0]);
        assert_ne!(div_i64, 0);

        #[allow(clippy::cast_sign_loss)]
        let div_entity = Entity::from_bits(div_i64 as u64).unwrap();
        assert_eq!(env.dom.get_parent(div_entity), Some(env.doc));
    }

    #[test]
    fn set_and_get_attribute() {
        let mut env = TestEnv::new();
        let wat = format!(
            r#"(module
                (import "elidex" "create_element" (func $create_elem (param i32 i32) (result i64)))
                (import "elidex" "set_attribute" (func $set_attr (param i64 i32 i32 i32 i32)))
                (import "elidex" "get_attribute" (func $get_attr (param i64 i32 i32) (result i64)))

                (memory (export "memory") 1)
                (data (i32.const 0) "div")
                (data (i32.const 3) "id")
                (data (i32.const 5) "myid")
                {WAT_BUMP_ALLOC}

                (func (export "test") (result i64)
                    (local $div i64)
                    (local.set $div (call $create_elem (i32.const 0) (i32.const 3)))
                    (call $set_attr (local.get $div) (i32.const 3) (i32.const 2) (i32.const 5) (i32.const 4))
                    (call $get_attr (local.get $div) (i32.const 3) (i32.const 2))
                )
            )"#
        );
        let instance = env.compile_and_instantiate(&wat);

        let results = env.call(&instance, "test", &[]).unwrap();

        let packed = expect_i64(&results[0]);
        assert_ne!(
            packed, 0,
            "get_attribute should return non-zero packed string"
        );
        assert_eq!(read_wasm_string(&instance, packed), "myid");
    }

    #[test]
    fn set_and_get_text_content() {
        let mut env = TestEnv::new();
        let wat = format!(
            r#"(module
                (import "elidex" "create_element" (func $create_elem (param i32 i32) (result i64)))
                (import "elidex" "set_text_content" (func $set_text (param i64 i32 i32)))
                (import "elidex" "get_text_content" (func $get_text (param i64) (result i64)))

                (memory (export "memory") 1)
                (data (i32.const 0) "div")
                (data (i32.const 3) "hello world")
                {WAT_BUMP_ALLOC}

                (func (export "test") (result i64)
                    (local $div i64)
                    (local.set $div (call $create_elem (i32.const 0) (i32.const 3)))
                    (call $set_text (local.get $div) (i32.const 3) (i32.const 11))
                    (call $get_text (local.get $div))
                )
            )"#
        );
        let instance = env.compile_and_instantiate(&wat);

        let results = env.call(&instance, "test", &[]).unwrap();

        let packed = expect_i64(&results[0]);
        assert_ne!(packed, 0);
        assert_eq!(read_wasm_string(&instance, packed), "hello world");
    }

    #[test]
    fn null_entity_returns_zero() {
        let mut env = TestEnv::new();
        let instance = env.compile_and_instantiate(
            r#"(module
                (import "elidex" "set_attribute" (func $set_attr (param i64 i32 i32 i32 i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "id")
                (data (i32.const 2) "val")

                (func (export "test")
                    (call $set_attr (i64.const 0) (i32.const 0) (i32.const 2) (i32.const 2) (i32.const 3))
                )
            )"#,
        );

        env.call(&instance, "test", &[]).unwrap();
    }

    #[test]
    fn dom_chain_create_append_set_get() {
        let mut env = TestEnv::new();
        let wat = format!(
            r#"(module
                (import "elidex" "get_document" (func $get_doc (result i64)))
                (import "elidex" "create_element" (func $create_elem (param i32 i32) (result i64)))
                (import "elidex" "create_text_node" (func $create_text (param i32 i32) (result i64)))
                (import "elidex" "append_child" (func $append (param i64 i64)))
                (import "elidex" "set_text_content" (func $set_text (param i64 i32 i32)))
                (import "elidex" "get_text_content" (func $get_text (param i64) (result i64)))
                (import "elidex" "set_attribute" (func $set_attr (param i64 i32 i32 i32 i32)))
                (import "elidex" "get_attribute" (func $get_attr (param i64 i32 i32) (result i64)))
                (import "elidex" "query_selector" (func $qs (param i64 i32 i32) (result i64)))

                (memory (export "memory") 1)
                (data (i32.const 0) "div")
                (data (i32.const 3) "span")
                (data (i32.const 7) "class")
                (data (i32.const 12) "greeting")
                (data (i32.const 20) "Hello!")
                (data (i32.const 26) ".greeting")
                {WAT_BUMP_ALLOC}

                (func (export "test") (result i64)
                    (local $doc i64)
                    (local $div i64)
                    (local $span i64)
                    (local $found i64)

                    (local.set $doc (call $get_doc))
                    (local.set $div (call $create_elem (i32.const 0) (i32.const 3)))
                    (call $append (local.get $doc) (local.get $div))
                    (local.set $span (call $create_elem (i32.const 3) (i32.const 4)))
                    (call $set_attr (local.get $span) (i32.const 7) (i32.const 5) (i32.const 12) (i32.const 8))
                    (call $append (local.get $div) (local.get $span))
                    (call $set_text (local.get $span) (i32.const 20) (i32.const 6))
                    (local.set $found (call $qs (local.get $doc) (i32.const 26) (i32.const 9)))
                    (call $get_text (local.get $found))
                )
            )"#
        );
        let instance = env.compile_and_instantiate(&wat);

        let results = env.call(&instance, "test", &[]).unwrap();

        let packed = expect_i64(&results[0]);
        assert_ne!(packed, 0);
        assert_eq!(read_wasm_string(&instance, packed), "Hello!");
    }

    // ---------------------------------------------------------------
    // Standalone ctor coverage (WASM JS API §5.3 / §5.4 / §5.5).
    // ---------------------------------------------------------------

    use crate::value::{
        HeapType, RefType, WasmGlobalDescriptor, WasmMemoryDescriptor, WasmRef,
        WasmTableDescriptor, WasmValueType,
    };

    #[test]
    fn new_memory_round_trip_basic() {
        let runtime = WasmRuntime::new().unwrap();
        let mem = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 1,
                maximum: Some(2),
            })
            .unwrap();
        // 1 page = 64 KiB.
        assert_eq!(mem.byte_size(), 64 * 1024);
    }

    #[test]
    fn new_memory_with_no_maximum() {
        let runtime = WasmRuntime::new().unwrap();
        let mem = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 0,
                maximum: None,
            })
            .unwrap();
        assert_eq!(mem.byte_size(), 0);
    }

    #[test]
    fn new_memory_grow_succeeds() {
        let runtime = WasmRuntime::new().unwrap();
        let mut mem = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 1,
                maximum: Some(4),
            })
            .unwrap();
        let g = mem.grow(2).unwrap();
        assert_eq!(g.pre_pages, 1);
        assert!(g.buffer_handle_invalidated);
        assert_eq!(mem.byte_size(), 3 * 64 * 1024);
    }

    #[test]
    fn new_table_round_trip_funcref_null_init() {
        let runtime = WasmRuntime::new().unwrap();
        let table = runtime
            .new_table(
                WasmTableDescriptor {
                    element: RefType {
                        nullable: true,
                        heap: HeapType::Func,
                    },
                    initial: 3,
                    maximum: Some(5),
                },
                WasmRef::Null(HeapType::Func),
            )
            .unwrap();
        assert_eq!(table.length().unwrap(), 3);
        assert_eq!(
            table.element_kind().unwrap(),
            WasmValueType::Ref(RefType {
                nullable: true,
                heap: HeapType::Func,
            })
        );
    }

    #[test]
    fn new_table_externref_null_init() {
        let runtime = WasmRuntime::new().unwrap();
        let table = runtime
            .new_table(
                WasmTableDescriptor {
                    element: RefType {
                        nullable: true,
                        heap: HeapType::Extern,
                    },
                    initial: 1,
                    maximum: None,
                },
                WasmRef::Null(HeapType::Extern),
            )
            .unwrap();
        assert_eq!(table.length().unwrap(), 1);
        let kind = table.element_kind().unwrap();
        assert_eq!(
            kind,
            WasmValueType::Ref(RefType {
                nullable: true,
                heap: HeapType::Extern,
            })
        );
    }

    #[test]
    fn new_global_immutable_i32() {
        let runtime = WasmRuntime::new().unwrap();
        let g = runtime
            .new_global(
                WasmGlobalDescriptor {
                    value_type: WasmValueType::I32,
                    mutable: false,
                },
                WasmValue::I32(42),
            )
            .unwrap();
        assert!(!g.mutable());
        match g.get() {
            WasmValue::I32(x) => assert_eq!(x, 42),
            other => panic!("expected I32, got {other:?}"),
        }
    }

    #[test]
    fn new_global_mutable_f64_set() {
        let runtime = WasmRuntime::new().unwrap();
        let mut g = runtime
            .new_global(
                WasmGlobalDescriptor {
                    value_type: WasmValueType::F64,
                    mutable: true,
                },
                WasmValue::F64(1.5),
            )
            .unwrap();
        assert!(g.mutable());
        g.set(WasmValue::F64(2.25)).unwrap();
        match g.get() {
            WasmValue::F64(x) => assert!((x - 2.25).abs() < f64::EPSILON),
            other => panic!("expected F64, got {other:?}"),
        }
    }

    #[test]
    fn new_global_immutable_set_errors() {
        let runtime = WasmRuntime::new().unwrap();
        let mut g = runtime
            .new_global(
                WasmGlobalDescriptor {
                    value_type: WasmValueType::I32,
                    mutable: false,
                },
                WasmValue::I32(7),
            )
            .unwrap();
        // WASM JS API §5.5 setter step 5 — engine surfaces as Runtime;
        // D-16 maps to JS TypeError.
        let err = g.set(WasmValue::I32(8)).unwrap_err();
        assert!(matches!(err.kind(), WasmErrorKind::Runtime));
    }

    #[test]
    fn instance_get_memory_cross_lookup_shares_view_flags() {
        // Regression for the cross-handle detach gap (PR F2 /simplify
        // gate, 3-angle convergent CRIT): two `inst.get_memory("m")`
        // lookups returned independent `WasmMemory` wrappers with
        // separate `view_flags`, so a grow via one failed to detach
        // views allocated via the other — violating WASM JS API §5.3
        // "refresh the Memory buffer" step 5.1. The instance-level
        // memory cache routes both lookups (and `exports()`) through
        // the same shared wrapper.
        let env = TestEnv::new();
        let instance = env.compile_and_instantiate(
            r#"(module
                (memory (export "mem") 1 4)
                (func (export "grow_one") (drop (memory.grow (i32.const 1))))
            )"#,
        );
        let mut mem_a = instance.get_memory("mem").unwrap();
        let mem_b = instance.get_memory("mem").unwrap();
        let view_b = mem_b.view();
        assert!(!view_b.is_detached());
        mem_a.grow(1).unwrap();
        assert!(
            view_b.is_detached(),
            "cross-lookup view_b must be detached after mem_a.grow"
        );

        // `exports()` must also route through the same cache so
        // memories returned from iteration share view_flags with
        // direct `get_memory` lookups.
        let view_c = match instance
            .exports()
            .into_iter()
            .find(|(name, _)| name == "mem")
            .expect("export 'mem' missing")
            .1
        {
            crate::WasmExportItem::Memory(m) => m.view(),
            other => panic!("expected Memory, got {other:?}"),
        };
        assert!(!view_c.is_detached());
        mem_a.grow(1).unwrap();
        assert!(
            view_c.is_detached(),
            "exports()-derived view_c must be detached after mem_a.grow"
        );
    }

    #[test]
    fn new_memory_min_greater_than_max_returns_err() {
        // Regression: wasmtime's `MemoryType::new` panics on `min > max`;
        // F2 must surface this as `Err` so D-16 maps it to RangeError per
        // WASM JS API §5.3 Memory(descriptor) ctor step 5 (RangeError
        // when memtype is not valid).
        let runtime = WasmRuntime::new().unwrap();
        let err = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 5,
                maximum: Some(2),
            })
            .unwrap_err();
        assert!(matches!(err.kind(), WasmErrorKind::Runtime));
    }

    #[test]
    fn standalone_handles_independent_stores() {
        // Sanity: two standalone Memories live in different stores so a
        // grow on one cannot perturb the other's byte_size. Per WASM JS
        // API §4.1 "Interaction of the WebAssembly Store with
        // JavaScript" the spec model is one associated store per agent;
        // per-handle store isolation here is an elidex engine-bridge
        // implementation choice (not a spec-mandated per-item slot).
        let runtime = WasmRuntime::new().unwrap();
        let mut a = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 1,
                maximum: Some(8),
            })
            .unwrap();
        let b = runtime
            .new_memory(WasmMemoryDescriptor {
                initial: 1,
                maximum: Some(8),
            })
            .unwrap();
        a.grow(2).unwrap();
        assert_eq!(a.byte_size(), 3 * 64 * 1024);
        assert_eq!(b.byte_size(), 64 * 1024);
    }
}
