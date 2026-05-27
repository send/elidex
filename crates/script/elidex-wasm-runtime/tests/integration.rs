//! End-to-end integration tests for `elidex-wasm-runtime`.
//!
//! Exercises the public API surface from a downstream-crate viewpoint:
//! compile → instantiate → call → introspect → trap-map. Tests that
//! need user-defined host-fn imports (`WasmImportValue::Func` built
//! from a Rust closure) are deferred — F1 scope ships engine-indep
//! conversion + dispatch but does not expose a host-fn builder
//! (D-16 builds host fns via the JS engine, not Rust closures; the
//! Rust-closure path is tracked as `#11-wasm-user-import-host-fn-builder`).

use elidex_wasm_runtime::{
    ImportExportKind, ImportObject, WasmErrorKind, WasmExportItem, WasmImportValue, WasmRuntime,
    WasmValue,
};

fn rt() -> WasmRuntime {
    WasmRuntime::new().expect("WasmRuntime::new failed")
}

fn compile(rt: &WasmRuntime, wat: &str) -> elidex_wasm_runtime::WasmModule {
    let bytes = wat::parse_str(wat).unwrap();
    rt.compile(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Minimal end-to-end: compile + instantiate + call
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_i32_const_returns_42() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (func (export "main") (result i32) i32.const 42)
        )"#,
    );
    let instance = rt
        .instantiate(&module, &ImportObject::default())
        .expect("instantiate failed");

    let main = instance.get_func("main").expect("export 'main' not found");

    // Spec coverage: §5.6 Exported Functions invocation. Stub session
    // / dom — host fn callbacks aren't invoked by this wasm so the
    // pointers stay unused throughout the call.
    let mut session = elidex_script_session::SessionCore::new();
    let mut dom = elidex_ecs::EcsDom::new();
    let document = dom.create_document_root();
    let bridge = elidex_wasm_runtime::ScriptHostBinding {
        session: &mut session,
        dom: &mut dom,
        document,
    };
    let results = main.call(&[], bridge).unwrap();
    assert_eq!(results.len(), 1);
    match &results[0] {
        WasmValue::I32(42) => {}
        other => panic!("expected I32(42), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. Module introspection
// ---------------------------------------------------------------------------

#[test]
fn module_imports_exports_introspection() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (import "env" "host_log" (func (param i32)))
            (func (export "main") (result i32) i32.const 0)
            (memory (export "mem") 1)
            (global (export "g") i32 (i32.const 7))
        )"#,
    );

    let imports = module.imports();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module, "env");
    assert_eq!(imports[0].name, "host_log");
    assert_eq!(imports[0].kind, ImportExportKind::Function);

    let exports = module.exports();
    // Order is module-declared order; build a set to compare kinds robustly.
    let names: Vec<_> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"mem"));
    assert!(names.contains(&"g"));

    let kind_of = |n: &str| {
        exports
            .iter()
            .find(|e| e.name == n)
            .map(|e| e.kind)
            .unwrap()
    };
    assert_eq!(kind_of("main"), ImportExportKind::Function);
    assert_eq!(kind_of("mem"), ImportExportKind::Memory);
    assert_eq!(kind_of("g"), ImportExportKind::Global);
}

// ---------------------------------------------------------------------------
// 3. Memory grow + buffer_handle_invalidated signal
// ---------------------------------------------------------------------------

#[test]
fn memory_grow_signals_unconditional_invalidation() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (memory (export "mem") 1)
        )"#,
    );
    let instance = rt.instantiate(&module, &ImportObject::default()).unwrap();

    let mut mem = instance.get_memory("mem").expect("export 'mem' not found");

    let initial_size = mem.byte_size();
    assert_eq!(initial_size, 64 * 1024); // 1 page = 64 KiB

    // Per WASM JS API §5.3 grow algorithm, the buffer is
    // unconditionally detached on every successful grow regardless of
    // whether wasmtime relocated the backing store.
    let result = mem.grow(1).expect("grow failed");
    assert_eq!(result.pre_pages, 1);
    let post_size = mem.byte_size();
    assert_eq!(post_size, 2 * 64 * 1024);
    assert!(
        result.buffer_handle_invalidated,
        "spec §5.3 requires unconditional detach on grow"
    );
}

#[test]
fn memory_with_data_reads_bytes() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (memory (export "mem") 1)
            (data (i32.const 0) "hello")
        )"#,
    );
    let instance = rt.instantiate(&module, &ImportObject::default()).unwrap();
    let mem = instance.get_memory("mem").unwrap();
    let first5: Vec<u8> = mem.with_data(|data| data[0..5].to_vec());
    assert_eq!(&first5, b"hello");
}

// ---------------------------------------------------------------------------
// 4. Trap mapping → RuntimeError
// ---------------------------------------------------------------------------

#[test]
fn unreachable_trap_maps_to_runtime_error() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (func (export "boom") unreachable)
        )"#,
    );
    let instance = rt.instantiate(&module, &ImportObject::default()).unwrap();

    let boom = instance.get_func("boom").unwrap();
    let mut session = elidex_script_session::SessionCore::new();
    let mut dom = elidex_ecs::EcsDom::new();
    let document = dom.create_document_root();
    let bridge = elidex_wasm_runtime::ScriptHostBinding {
        session: &mut session,
        dom: &mut dom,
        document,
    };

    let err = boom.call(&[], bridge).unwrap_err();
    // Per WASM JS API §5.2 `initialize an Instance object` step 3:
    // traps become RuntimeError regardless of the call-site's default
    // kind. Error class definitions are in §5.10.
    assert!(matches!(err.kind, WasmErrorKind::Runtime));
    // Source preserved (D-9 + deviation D-iii: Option<wasmtime::Error>
    // is Some for any wasmtime-originated error).
    assert!(err.source_err().is_some());
}

// ---------------------------------------------------------------------------
// 5. Exports iteration via the new engine-indep API
// ---------------------------------------------------------------------------

#[test]
fn instance_exports_yields_engine_indep_items() {
    let rt = rt();
    let module = compile(
        &rt,
        r#"(module
            (func (export "f") (result i32) i32.const 1)
            (memory (export "m") 1)
            (global (export "g") i32 (i32.const 0))
        )"#,
    );
    let instance = rt.instantiate(&module, &ImportObject::default()).unwrap();

    let exports = instance.exports();
    let mut saw_func = false;
    let mut saw_memory = false;
    let mut saw_global = false;
    for (name, item) in &exports {
        match (name.as_str(), item) {
            ("f", WasmExportItem::Func(_)) => saw_func = true,
            ("m", WasmExportItem::Memory(_)) => saw_memory = true,
            ("g", WasmExportItem::Global(_)) => saw_global = true,
            _ => {}
        }
    }
    assert!(saw_func, "expected Func export 'f'");
    assert!(saw_memory, "expected Memory export 'm'");
    assert!(saw_global, "expected Global export 'g'");
}

// ---------------------------------------------------------------------------
// 6. Non-empty ImportObject fails fast with LinkError (D-16 deferral)
// ---------------------------------------------------------------------------

#[test]
fn instantiate_rejects_non_empty_imports_with_link_error() {
    let rt = rt();
    // Donor module: produces a `WasmFunc` we can stash in an
    // `ImportObject`. Its store is private to this instance — passing
    // the func to a *second* `instantiate` is the cross-store case the
    // D-16 deferral guard rejects.
    let donor = compile(
        &rt,
        r#"(module (func (export "f") (result i32) i32.const 0))"#,
    );
    let donor_inst = rt.instantiate(&donor, &ImportObject::default()).unwrap();
    let donor_func = donor_inst.get_func("f").expect("donor 'f' missing");

    let mut imports = ImportObject::new();
    imports.define("env", "f", WasmImportValue::Func(donor_func));

    let target = compile(
        &rt,
        r#"(module
            (import "env" "f" (func (result i32)))
            (func (export "wrap") (result i32) call 0)
        )"#,
    );
    let err = rt.instantiate(&target, &imports).unwrap_err();
    assert!(matches!(err.kind, WasmErrorKind::Link));
    assert!(
        err.message().contains("ImportObject"),
        "expected guard message, got: {}",
        err.message()
    );
}
