//! `WasmRuntime` — compiles and instantiates Wasm modules with DOM host functions.

use std::sync::Arc;

use elidex_dom_api::registry::{
    create_cssom_registry, create_dom_registry, CssomHandlerRegistry, DomHandlerRegistry,
};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store, Val, ValType};

use crate::error::{classify_wasmtime_error, WasmError, WasmErrorKind};
use crate::host_funcs::register_host_functions;
use crate::host_state::HostState;

/// A compiled Wasm module (engine-independent, reusable).
#[derive(Clone)]
pub struct WasmModule {
    module: Module,
}

/// Default fuel budget per `call_export()` invocation.
///
/// 1 billion instructions is roughly equivalent to a few seconds of execution
/// on modern hardware. Prevents infinite loops / runaway Wasm modules.
const DEFAULT_FUEL: u64 = 1_000_000_000;

/// A Wasm runtime that compiles and instantiates modules with DOM host functions.
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostState>,
    dom_registry: Arc<DomHandlerRegistry>,
    cssom_registry: Arc<CssomHandlerRegistry>,
}

impl WasmRuntime {
    /// Create a new runtime with default DOM/CSSOM registries.
    pub fn new() -> Result<Self, WasmError> {
        let dom_registry = Arc::new(create_dom_registry());
        let cssom_registry = Arc::new(create_cssom_registry());
        Self::with_registries(dom_registry, cssom_registry)
    }

    /// Create a new runtime with custom registries.
    pub fn with_registries(
        dom_registry: Arc<DomHandlerRegistry>,
        cssom_registry: Arc<CssomHandlerRegistry>,
    ) -> Result<Self, WasmError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)
            .map_err(|e| WasmError::new(WasmErrorKind::Compile, e.to_string()))?;
        let mut linker = Linker::new(&engine);
        register_host_functions(&mut linker)
            .map_err(|e| WasmError::new(WasmErrorKind::Link, e.to_string()))?;
        Ok(Self {
            engine,
            linker,
            dom_registry,
            cssom_registry,
        })
    }

    /// Compile a Wasm module from bytes.
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<WasmModule, WasmError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| classify_wasmtime_error(&e, WasmErrorKind::Compile))?;
        Ok(WasmModule { module })
    }

    /// Validate a Wasm module without compiling it.
    ///
    /// Per WebAssembly JS API §4.5.2 `WebAssembly.validate(bufferSource)`.
    pub fn validate(&self, wasm_bytes: &[u8]) -> bool {
        Module::validate(&self.engine, wasm_bytes).is_ok()
    }

    /// Instantiate a compiled module, linking host functions.
    pub fn instantiate(&self, module: &WasmModule) -> Result<WasmInstance, WasmError> {
        let host_state = HostState::new(self.dom_registry.clone(), self.cssom_registry.clone());
        let mut store = Store::new(&self.engine, host_state);
        let instance = self
            .linker
            .instantiate(&mut store, &module.module)
            .map_err(|e| classify_wasmtime_error(&e, WasmErrorKind::Link))?;
        Ok(WasmInstance { store, instance })
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("failed to create default WasmRuntime")
    }
}

/// A live Wasm module instance with its own store.
pub struct WasmInstance {
    store: Store<HostState>,
    instance: Instance,
}

/// Drop guard that calls `HostState::unbind()` on drop.
struct UnbindGuard<'a>(&'a mut Store<HostState>);
impl Drop for UnbindGuard<'_> {
    fn drop(&mut self) {
        self.0.data_mut().unbind();
    }
}

impl WasmInstance {
    /// Collect the names of exported items.
    pub fn export_names(&mut self) -> Vec<String> {
        self.instance
            .exports(&mut self.store)
            .map(|export| export.name().to_owned())
            .collect()
    }

    /// Look up an exported function by name (without binding).
    pub fn get_func(&mut self, name: &str) -> Option<wasmtime::Func> {
        self.instance.get_func(&mut self.store, name)
    }

    /// Get the parameter types of an exported function.
    pub fn export_param_types(&mut self, name: &str) -> Option<Vec<ValType>> {
        let func = self.instance.get_func(&mut self.store, name)?;
        let ty = func.ty(&self.store);
        Some(ty.params().collect())
    }

    /// Get the byte size of an exported memory.
    pub fn memory_byte_size(&mut self, name: &str) -> Option<usize> {
        let mem = self.instance.get_memory(&mut self.store, name)?;
        Some(mem.data_size(&self.store))
    }

    /// Call an exported function by name.
    ///
    /// The `session` and `dom` are bound for the duration of the call.
    pub fn call_export(
        &mut self,
        name: &str,
        args: &[Val],
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> Result<Vec<Val>, WasmError> {
        self.store.data_mut().bind(session, dom, document);
        // UnbindGuard ensures unbind even on early return or panic.
        let guard = UnbindGuard(&mut self.store);
        guard
            .0
            .set_fuel(DEFAULT_FUEL)
            .map_err(|e| WasmError::new(WasmErrorKind::Runtime, e.to_string()))?;

        let export_func = self.instance.get_func(&mut *guard.0, name).ok_or_else(|| {
            WasmError::new(WasmErrorKind::Link, format!("export '{name}' not found"))
        })?;

        let ty = export_func.ty(&*guard.0);
        let mut results = vec![Val::I32(0); ty.results().len()];
        export_func
            .call(&mut *guard.0, args, &mut results)
            .map_err(|e| classify_wasmtime_error(&e, WasmErrorKind::Runtime))?;

        drop(guard);
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            self.runtime.instantiate(&module).unwrap()
        }

        fn call(
            &mut self,
            instance: &mut WasmInstance,
            name: &str,
            args: &[Val],
        ) -> Result<Vec<Val>, WasmError> {
            instance.call_export(name, args, &mut self.session, &mut self.dom, self.doc)
        }
    }

    /// WAT fragment: bump allocator starting at offset 1024.
    /// Include inside a `(module ...)` that exports `memory`.
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

    /// Read a packed string result from Wasm linear memory.
    fn read_wasm_string(instance: &mut WasmInstance, packed: i64) -> String {
        let (ptr, len) = unpack_wasm_string(packed);
        let memory = instance
            .instance
            .get_memory(&mut instance.store, "memory")
            .unwrap();
        let data = memory.data(&instance.store);
        String::from_utf8(data[ptr..ptr + len].to_vec()).unwrap()
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
        let mut instance = env.compile_and_instantiate(
            r#"(module
                (import "elidex" "get_document" (func $get_doc (result i64)))
                (func (export "test_get_doc") (result i64)
                    call $get_doc
                )
            )"#,
        );

        let results = env.call(&mut instance, "test_get_doc", &[]).unwrap();

        #[allow(clippy::cast_possible_wrap)]
        let expected = env.doc.to_bits().get() as i64;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unwrap_i64(), expected);
    }

    #[test]
    fn create_element_and_append() {
        let mut env = TestEnv::new();
        let mut instance = env.compile_and_instantiate(
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

        let results = env.call(&mut instance, "test", &[]).unwrap();

        let div_i64 = results[0].unwrap_i64();
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
        let mut instance = env.compile_and_instantiate(&wat);

        let results = env.call(&mut instance, "test", &[]).unwrap();

        let packed = results[0].unwrap_i64();
        assert_ne!(
            packed, 0,
            "get_attribute should return non-zero packed string"
        );

        let returned = read_wasm_string(&mut instance, packed);
        assert_eq!(returned, "myid");
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
        let mut instance = env.compile_and_instantiate(&wat);

        let results = env.call(&mut instance, "test", &[]).unwrap();

        let packed = results[0].unwrap_i64();
        assert_ne!(packed, 0);

        let returned = read_wasm_string(&mut instance, packed);
        assert_eq!(returned, "hello world");
    }

    #[test]
    fn null_entity_returns_zero() {
        let mut env = TestEnv::new();
        let mut instance = env.compile_and_instantiate(
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

        env.call(&mut instance, "test", &[]).unwrap();
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
        let mut instance = env.compile_and_instantiate(&wat);

        let results = env.call(&mut instance, "test", &[]).unwrap();

        let packed = results[0].unwrap_i64();
        assert_ne!(packed, 0);

        let returned = read_wasm_string(&mut instance, packed);
        assert_eq!(returned, "Hello!");
    }
}
