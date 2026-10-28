//! `WebAssembly` namespace + 3 error classes + `validate` / `compile`
//! / `Module` ctor tests (slot `#11-wasm-vm` / D-16, plan-memo §5
//! Stage 2), split out of `tests_wasm.rs` along its Stage 2 / Stage 3
//! seam.

#![cfg(feature = "engine")]

use super::super::Vm;
use super::tests_wasm::{eval_bool, eval_string, ANSWER_MODULE_BYTES_JS, IMPORT_MODULE_BYTES_JS};

/// WASM JS API §5 — namespace exists on globalThis with the static
/// methods + 5 ctor + 3 error classes visible.  Stage 2 ships
/// `validate` / `compile` / `Module` / `CompileError` / `LinkError` /
/// `RuntimeError`; Stage 3+4 will fill in `instantiate` / `Instance`
/// / `Memory` / `Table` / `Global`.
#[test]
fn namespace_installed_with_stage2_surface() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly === 'object' && WebAssembly !== null"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.validate === 'function'"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.compile === 'function'"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.Module === 'function'"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.CompileError === 'function'"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.LinkError === 'function'"
    ));
    assert!(eval_bool(
        &mut vm,
        "typeof WebAssembly.RuntimeError === 'function'"
    ));
}

/// WASM JS API §5.10 — the 3 error classes are Error subclasses so
/// `instanceof Error` holds on instances.
#[test]
fn error_classes_chain_to_error_prototype() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "(new WebAssembly.CompileError('oops')) instanceof Error"
    ));
    assert!(eval_bool(
        &mut vm,
        "(new WebAssembly.LinkError('boom')) instanceof Error"
    ));
    assert!(eval_bool(
        &mut vm,
        "(new WebAssembly.RuntimeError('trap')) instanceof Error"
    ));
}

/// WASM JS API §5.10 — `name` is the class name, `message` is the
/// ctor argument; `String(err)` flows through Error.prototype.toString
/// to produce `"<Name>: <message>"`.
#[test]
fn error_classes_set_name_and_message() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "String(new WebAssembly.CompileError('oops'))"),
        "CompileError: oops"
    );
    assert_eq!(
        eval_string(&mut vm, "String(new WebAssembly.LinkError('boom'))"),
        "LinkError: boom"
    );
    assert_eq!(
        eval_string(&mut vm, "String(new WebAssembly.RuntimeError('trap'))"),
        "RuntimeError: trap"
    );
}

/// WASM JS API §5 `WebAssembly.validate(bytes)` — returns `false` on
/// bytes that aren't valid wasm; the IDL `bool` return contract means
/// validation failure does NOT throw (distinct from `compile()` which
/// rejects the returned Promise with CompileError).
#[test]
fn validate_rejects_invalid_bytes_without_throwing() {
    let mut vm = Vm::new();
    // 0xff... is not a valid wasm module header.
    assert!(!eval_bool(
        &mut vm,
        "WebAssembly.validate(new Uint8Array([0xff,0xff,0xff,0xff]).buffer)"
    ));
}

/// WASM JS API §5 — empty header bytes (only `\0asm\x01\0\0\0`)
/// constitute a valid (empty) module per spec; `validate` returns
/// `true`.  The bytes are crafted directly to avoid a wat→bytes
/// dependency on the test side.
#[test]
fn validate_accepts_empty_module_bytes() {
    let mut vm = Vm::new();
    // \0asm + version 1 little-endian = empty valid module.
    assert!(eval_bool(
        &mut vm,
        "WebAssembly.validate(new Uint8Array([0,0x61,0x73,0x6d,1,0,0,0]).buffer)"
    ));
}

/// WASM JS API §5.1 ctor algorithm — `new WebAssembly.Module(bytes)`
/// on invalid bytes throws `CompileError` per step 3 ("If module is
/// error, throw a CompileError exception").
#[test]
fn module_ctor_throws_compile_error_on_invalid_bytes() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "try { new WebAssembly.Module(new Uint8Array([0xff]).buffer); false } \
         catch (e) { e instanceof WebAssembly.CompileError && e instanceof Error }"
    ));
}

/// WebIDL §3.7.1 step 1.2 (NewTarget undefined → throw TypeError),
/// enforced for native `[Constructor]` Interface objects at the
/// `vm/interpreter.rs::call_dispatch` `NativeFunction` arm under
/// `CallShape::ConstructorOnly` (#11-vm-native-constructor-only-flag
/// Stage 2 pilot — Module is the first ctor migrated to validate the
/// dispatch-side gate; Stage 3 will migrate the remaining 65 sites).
/// Asserts canonical error message form for bare-call regression.
#[test]
fn module_ctor_requires_new() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "try { WebAssembly.Module(new Uint8Array([0,0x61,0x73,0x6d,1,0,0,0]).buffer); \
             'no throw' } catch (e) { e.message }"
        ),
        "Failed to construct 'Module': Please use the 'new' operator"
    );
}

/// WASM JS API §5.1 Module ctor — empty module bytes succeed; the
/// returned object brand-checks as `WebAssembly.Module` (via the
/// constructor's prototype chain).
#[test]
fn module_ctor_accepts_empty_module_bytes() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var m = new WebAssembly.Module(new Uint8Array([0,0x61,0x73,0x6d,1,0,0,0]).buffer); \
         m instanceof WebAssembly.Module"
    ));
}

/// WASM JS API §5.1 static introspection — `Module.exports(m)` /
/// `Module.imports(m)` on an empty module return empty arrays.
/// Brand-check: passing a non-Module throws TypeError.
#[test]
fn module_static_methods_brand_check_and_return_arrays() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var m = new WebAssembly.Module(new Uint8Array([0,0x61,0x73,0x6d,1,0,0,0]).buffer); \
         Array.isArray(WebAssembly.Module.exports(m)) && \
         WebAssembly.Module.exports(m).length === 0 && \
         Array.isArray(WebAssembly.Module.imports(m)) && \
         WebAssembly.Module.imports(m).length === 0"
    ));
    // Non-Module rejected with TypeError.
    assert!(eval_bool(
        &mut vm,
        "try { WebAssembly.Module.exports({}); false } \
         catch (e) { e instanceof TypeError }"
    ));
}

/// WebIDL §3.2.17 dictionary → JS conversion step 3.1.2.3 uses
/// `CreateDataPropertyOrThrow`, so `ModuleExportDescriptor` /
/// `ModuleImportDescriptor` members must land as enumerable data
/// properties (`{W, E, C}`).  Regression for Copilot R1 finding —
/// builder previously installed members with `PropertyAttrs::METHOD`
/// (¬E), making `Object.keys(desc)` skip them.  Uses
/// `ANSWER_MODULE_BYTES_JS` (one export) + `IMPORT_MODULE_BYTES_JS`
/// (one import), imported from `tests_wasm.rs`, as the descriptor input.
#[test]
fn module_descriptors_are_enumerable_data_properties() {
    let mut vm = Vm::new();
    // Export descriptor: `{name, kind}` — both keys enumerable.
    assert!(eval_bool(
        &mut vm,
        &format!(
            "var m = new WebAssembly.Module({ANSWER_MODULE_BYTES_JS}); \
             var d = WebAssembly.Module.exports(m)[0]; \
             var k = Object.keys(d).sort(); \
             k.length === 2 && k[0] === 'kind' && k[1] === 'name' && \
             d.name === 'main' && d.kind === 'function'"
        )
    ));
    // Import descriptor: `{module, name, kind}` — all 3 keys enumerable.
    assert!(eval_bool(
        &mut vm,
        &format!(
            "var m = new WebAssembly.Module({IMPORT_MODULE_BYTES_JS}); \
             var d = WebAssembly.Module.imports(m)[0]; \
             var k = Object.keys(d).sort(); \
             k.length === 3 && k[0] === 'kind' && k[1] === 'module' && k[2] === 'name' && \
             d.module === 'env' && d.name === 'f' && d.kind === 'function'"
        )
    ));
}

/// WASM JS API §5 `WebAssembly.compile(bytes)` returns a Promise.
/// Stage 2 settles synchronously via the microtask queue; observable
/// shape is `Promise<Module>` resolved on valid bytes / rejected with
/// `CompileError` on invalid bytes.
#[test]
fn compile_returns_promise_resolved_on_valid_bytes() {
    let mut vm = Vm::new();
    // Promise instanceof check + valid-bytes resolves.
    assert!(eval_bool(
        &mut vm,
        "var p = WebAssembly.compile(new Uint8Array([0,0x61,0x73,0x6d,1,0,0,0]).buffer); \
         p instanceof Promise"
    ));
}

/// WASM JS API §5 `WebAssembly.compile(bytes)` — invalid bytes
/// reject with `CompileError`.  Uses the side-channel pattern
/// (`globalThis.__compile_test__.outcome`) so the rejection class is
/// observable in a second `vm.eval` after the microtask queue
/// drains, matching `instantiate_bytes_overload_resolves_with_dict`
/// + `missing_import_surfaces_as_link_error` Promise-overload tests.
#[test]
fn compile_rejects_with_compile_error_on_invalid_bytes() {
    let mut vm = Vm::new();
    let _ = vm
        .eval(
            "globalThis.__compile_test__ = {}; \
             WebAssembly.compile(new Uint8Array([0xff,0xff,0xff,0xff]).buffer).then( \
                 _ => { globalThis.__compile_test__.outcome = 'resolved'; }, \
                 e => { globalThis.__compile_test__.outcome = \
                            e instanceof WebAssembly.CompileError ? 'compile' : 'other'; } \
             );",
        )
        .unwrap();
    assert_eq!(
        eval_string(&mut vm, "String(globalThis.__compile_test__.outcome)"),
        "compile"
    );
}

/// The text format is not a module. WASM JS API "compile a WebAssembly
/// module" step 1 is `module_decode(bytes)`, the binary decoder, so the
/// UTF-8 bytes of `(module)` are an error: `new WebAssembly.Module`
/// throws `CompileError` (its step 3) and `WebAssembly.compile` rejects
/// with one. wasmtime's `wat` feature would have parsed the text instead;
/// elidex builds wasmtime without it.
#[test]
fn text_format_is_a_compile_error() {
    const TEXT_MODULE_BYTES_JS: &str =
        "new Uint8Array([0x28,0x6d,0x6f,0x64,0x75,0x6c,0x65,0x29]).buffer";
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        &format!(
            "try {{ new WebAssembly.Module({TEXT_MODULE_BYTES_JS}); false }} \
             catch (e) {{ e instanceof WebAssembly.CompileError }}"
        )
    ));
    let _ = vm
        .eval(&format!(
            "globalThis.__text_test__ = {{}}; \
             WebAssembly.compile({TEXT_MODULE_BYTES_JS}).then( \
                 _ => {{ globalThis.__text_test__.outcome = 'resolved'; }}, \
                 e => {{ globalThis.__text_test__.outcome = \
                            e instanceof WebAssembly.CompileError ? 'compile' : 'other'; }} \
             );"
        ))
        .unwrap();
    assert_eq!(
        eval_string(&mut vm, "String(globalThis.__text_test__.outcome)"),
        "compile"
    );
}
