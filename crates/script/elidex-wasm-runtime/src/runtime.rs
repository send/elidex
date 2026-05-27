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

use crate::engine_conv::{import_value_to_extern, wasm_error_from_wasmtime};
use crate::error::{WasmError, WasmErrorKind};
use crate::handle::WasmStoreHandle;
use crate::host::funcs::register_host_functions;
use crate::host::state::HostState;
use crate::imports::ImportObject;
use crate::instance::WasmInstance;
use crate::module::WasmModule;

/// Default fuel budget — 1 billion instructions is roughly a few
/// seconds on modern hardware, guarding against runaway wasm modules
/// during host dispatch. The store is seeded with this budget at
/// `instantiate` time, and `WasmInstance::call_func` resets the budget
/// to this value at the start of every call so a single long-running
/// call cannot deplete the budget for subsequent calls on the same
/// instance. Exposed `pub(crate)` so `instance.rs` can perform the
/// per-call reset.
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
    /// "Read the imports". `imports` is the engine-indep flat
    /// `(module, name) → value` map; the host (D-16) flattens the JS
    /// record-of-records before calling. Single canonical form —
    /// callers pass `ImportObject::default()` for the empty-imports
    /// case rather than calling a dual API.
    pub fn instantiate(
        &self,
        module: &WasmModule,
        imports: &ImportObject,
    ) -> Result<WasmInstance, WasmError> {
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
}
