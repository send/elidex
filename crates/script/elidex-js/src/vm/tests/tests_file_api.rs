//! `File` interface tests (File API §4, slot `#11-file-api` Phase 1).
//!
//! Covers ctor (bits + name + options), `name` / `lastModified`
//! accessors, `/ → :` sanitisation, `endings: "native"` line-ending
//! normalisation, prototype-chain inheritance (`instanceof File` /
//! `instanceof Blob`), and Blob accessor reuse via brand widening
//! (`require_blob_or_file_this`).
//!
//! FileList + FileReader cover lives in dedicated sibling test
//! modules added by Phases 2 and 4.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

// Minimal `doc > html > body` fixture for tests that need
// `document.createElement(...)` (Phase 3 input.files coverage).
// Mirrors the pattern in `tests_dom_handler_dispatch.rs`.
fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

fn with_doc_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    f(guard.0)
}

fn eval_in_doc_bool(source: &str) -> bool {
    with_doc_vm(|vm| match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    })
}

fn eval_in_doc_number(source: &str) -> f64 {
    with_doc_vm(|vm| match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    })
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Constructor — required args + name handling
// ---------------------------------------------------------------------------

#[test]
fn ctor_requires_two_args_zero_throws() {
    let mut vm = Vm::new();
    let err = vm.eval("new File();").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("File"), "expected File error, got: {msg}");
}

#[test]
fn ctor_requires_two_args_one_throws() {
    let mut vm = Vm::new();
    let err = vm.eval("new File([]);").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("File"), "expected File error, got: {msg}");
}

#[test]
fn ctor_two_args_minimum_succeeds() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "new File([], 'x') instanceof File;"));
}

#[test]
fn ctor_must_be_called_with_new() {
    let mut vm = Vm::new();
    let err = vm.eval("File([], 'a');").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("new"), "expected 'new' error, got: {msg}");
}

// ---------------------------------------------------------------------------
// .name accessor
// ---------------------------------------------------------------------------

#[test]
fn name_accessor_returns_arg() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new File([], 'hello.txt').name;"),
        "hello.txt"
    );
}

#[test]
fn name_empty_string_allowed() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new File([], '').name;"), "");
}

#[test]
fn name_slash_replaced_with_colon() {
    let mut vm = Vm::new();
    // FileAPI §4.1 step 2 — `/` → `:` for filesystem path safety.
    assert_eq!(
        eval_string(&mut vm, "new File([], 'a/b/c.txt').name;"),
        "a:b:c.txt"
    );
}

#[test]
fn name_no_slash_unchanged() {
    let mut vm = Vm::new();
    // No `/` present — should return the same StringId (no allocation
    // on the fast path).  Observably: identity to original.
    assert_eq!(
        eval_string(&mut vm, "new File([], 'normal-name.bin').name;"),
        "normal-name.bin"
    );
}

#[test]
fn name_coerced_via_to_string() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new File([], 42).name;"), "42");
}

#[test]
fn name_object_via_to_string() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new File([], {toString: () => 'fromObj'}).name;"),
        "fromObj"
    );
}

// ---------------------------------------------------------------------------
// .lastModified accessor
// ---------------------------------------------------------------------------

#[test]
fn last_modified_default_finite() {
    let mut vm = Vm::new();
    // Default is `Date.now()` per FileAPI §4.1 step 3 — Unix epoch ms.
    let n = eval_number(&mut vm, "new File([], 'x').lastModified;");
    assert!(n.is_finite(), "expected finite lastModified, got {n}");
    assert!(n >= 0.0, "expected non-negative lastModified, got {n}");
}

#[test]
fn last_modified_default_is_unix_epoch_ms() {
    // Copilot R1 regression: `now_epoch_ms` returned VM-uptime ms
    // (~0) instead of Unix epoch ms (~1.7e12).  This made
    // `new Date(file.lastModified)` render 1970 instead of "now" and
    // broke `Date.now() - file.lastModified < 1000` framework patterns.
    let mut vm = Vm::new();
    let n = eval_number(&mut vm, "new File([], 'x').lastModified;");
    // 2026-01-01T00:00:00Z in ms == 1767225600000.  Any sane wall
    // clock running this test is past that.  Upper bound 2050 keeps
    // the test honest if the clock somehow runs absurdly fast.
    assert!(
        (1_767_225_600_000.0..2_524_608_000_000.0).contains(&n),
        "expected Unix epoch ms in (2026, 2050) range, got {n}"
    );
}

#[test]
fn last_modified_default_is_integer_milliseconds() {
    // Copilot R2 regression: `now_epoch_ms` used `as_secs_f64() *
    // 1000.0` which yielded fractional ms (e.g. 1767234567890.123).
    // `Date.now()` always returns integer ms per WebIDL `long long`
    // semantics; fix uses `Duration::as_millis() as f64` to truncate.
    let mut vm = Vm::new();
    let n = eval_number(&mut vm, "new File([], 'x').lastModified;");
    assert_eq!(n, n.trunc(), "expected integer lastModified, got {n}");
}

#[test]
fn last_modified_explicit_value() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([], 'x', {lastModified: 12345}).lastModified;"
        ),
        12345.0
    );
}

#[test]
fn last_modified_negative_allowed() {
    let mut vm = Vm::new();
    // WebIDL `long long` — negative values are valid.
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([], 'x', {lastModified: -100}).lastModified;"
        ),
        -100.0
    );
}

#[test]
fn last_modified_nan_becomes_zero() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([], 'x', {lastModified: NaN}).lastModified;"
        ),
        0.0
    );
}

#[test]
fn last_modified_infinity_becomes_zero() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([], 'x', {lastModified: Infinity}).lastModified;"
        ),
        0.0
    );
}

#[test]
fn last_modified_non_integer_truncated() {
    let mut vm = Vm::new();
    // WebIDL `long long` truncates toward zero.
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([], 'x', {lastModified: 3.7}).lastModified;"
        ),
        3.0
    );
    let mut vm2 = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm2,
            "new File([], 'x', {lastModified: -3.7}).lastModified;"
        ),
        -3.0
    );
}

// ---------------------------------------------------------------------------
// Inheritance: instanceof + Blob accessor brand widening
// ---------------------------------------------------------------------------

#[test]
fn instance_of_file() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "new File([], 'x') instanceof File;"));
}

#[test]
fn instance_of_blob_via_prototype_chain() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "new File([], 'x') instanceof Blob;"));
}

#[test]
fn blob_not_instance_of_file() {
    let mut vm = Vm::new();
    assert!(!eval_bool(&mut vm, "new Blob([]) instanceof File;"));
}

#[test]
fn inherited_size_accessor_works() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new File(['hello'], 'x').size;"), 5.0);
}

#[test]
fn inherited_type_accessor_works() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new File(['x'], 'a', {type: 'TEXT/Plain'}).type;"),
        "text/plain"
    );
}

#[test]
fn inherited_type_default_empty() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new File([], 'x').type;"), "");
}

#[test]
fn inherited_slice_returns_blob_not_file() {
    let mut vm = Vm::new();
    // Spec: `Blob.prototype.slice` returns a Blob, not a File, even
    // when called on a File receiver.
    assert!(eval_bool(
        &mut vm,
        "let f = new File(['hello world'], 'x'); \
         let s = f.slice(0, 5); \
         s instanceof Blob && !(s instanceof File);"
    ));
}

// ---------------------------------------------------------------------------
// Bits coercion — same path as Blob ctor
// ---------------------------------------------------------------------------

#[test]
fn bits_concatenate_strings() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new File(['hi', ' ', 'world'], 'g').size;"),
        8.0
    );
}

#[test]
fn bits_empty_array() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new File([], 'g').size;"), 0.0);
}

#[test]
fn bits_includes_blob() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new File([new Blob(['abc'])], 'g').size;"),
        3.0
    );
}

#[test]
fn bits_includes_file_as_blob_part() {
    let mut vm = Vm::new();
    // File is a Blob per spec — should be acceptable as a BlobPart.
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([new File(['abc'], 'inner.txt')], 'outer.txt').size;"
        ),
        3.0
    );
}

#[test]
fn bits_includes_arraybuffer() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File([new Uint8Array([1, 2, 3, 4]).buffer], 'g').size;"
        ),
        4.0
    );
}

#[test]
fn bits_includes_typed_array_view() {
    let mut vm = Vm::new();
    // 4-byte buffer, view starts at offset 1 with length 2.
    assert_eq!(
        eval_number(
            &mut vm,
            "let buf = new Uint8Array([10, 20, 30, 40]).buffer; \
             let view = new Uint8Array(buf, 1, 2); \
             new File([view], 'g').size;"
        ),
        2.0
    );
}

// ---------------------------------------------------------------------------
// options.endings line-ending normalisation
// ---------------------------------------------------------------------------

#[test]
fn endings_transparent_default_no_normalize() {
    let mut vm = Vm::new();
    // No `\r` → `\n` collapse with default endings.  Spec §4.1 step 1
    // says "transparent" leaves USVString bytes untouched.
    // `\r\n` is 2 bytes in UTF-8 ASCII.
    assert_eq!(
        eval_number(&mut vm, "new File(['a\\r\\nb'], 'g').size;"),
        4.0 // 'a' + '\r' + '\n' + 'b' = 4 bytes
    );
}

#[test]
fn endings_native_normalizes_crlf_to_lf() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File(['a\\r\\nb'], 'g', {endings: 'native'}).size;"
        ),
        3.0 // 'a' + '\n' + 'b' = 3 bytes ('\r\n' → '\n')
    );
}

#[test]
fn endings_native_normalizes_lone_cr_to_lf() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File(['a\\rb'], 'g', {endings: 'native'}).size;"
        ),
        3.0 // 'a' + '\n' + 'b' = 3 bytes (lone '\r' → '\n')
    );
}

#[test]
fn endings_native_leaves_lf_unchanged() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new File(['a\\nb'], 'g', {endings: 'native'}).size;"
        ),
        3.0
    );
}

#[test]
fn endings_native_does_not_affect_buffer_source() {
    let mut vm = Vm::new();
    // BufferSource (ArrayBuffer) bytes pass through verbatim regardless
    // of endings — only USVString entries are normalised per spec.
    assert_eq!(
        eval_number(
            &mut vm,
            "let crlf = new Uint8Array([97, 13, 10, 98]).buffer; \
             new File([crlf], 'g', {endings: 'native'}).size;"
        ),
        4.0 // raw 'a' + '\r' + '\n' + 'b' preserved
    );
}

#[test]
fn endings_invalid_throws_type_error() {
    let mut vm = Vm::new();
    let err = vm
        .eval("new File([], 'g', {endings: 'invalid'});")
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("EndingType") || msg.contains("invalid"),
        "expected TypeError mentioning enum, got: {msg}"
    );
}

#[test]
fn endings_transparent_explicit_accepted() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new File([], 'g', {endings: 'transparent'}) instanceof File;"
    ));
}

// ---------------------------------------------------------------------------
// Brand check: Blob.prototype methods called on Files
// ---------------------------------------------------------------------------

#[test]
fn blob_proto_get_size_via_call() {
    let mut vm = Vm::new();
    // Explicit call via prototype proves the brand widening works.
    assert_eq!(
        eval_number(
            &mut vm,
            "let f = new File(['abc'], 'g'); \
             Object.getOwnPropertyDescriptor(\
               Blob.prototype, 'size').get.call(f);"
        ),
        3.0
    );
}

#[test]
fn blob_proto_get_size_rejects_plain_object() {
    let mut vm = Vm::new();
    let err = vm
        .eval(
            "Object.getOwnPropertyDescriptor(Blob.prototype, 'size')\
              .get.call({});",
        )
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("non-Blob"),
        "expected non-Blob error, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// FileList — interface presence (Phase 2)
//
// Instance behaviour (real `length` / `item(i)` round-trips) covered
// once Phase 3 (`<input type=file>.files`) or Phase 5 (DataTransfer
// wiring) wires production-path FileList allocators.
// ---------------------------------------------------------------------------

#[test]
fn file_list_global_exists() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "typeof FileList;"), "function");
}

#[test]
fn file_list_new_throws_illegal_constructor() {
    let mut vm = Vm::new();
    let err = vm.eval("new FileList();").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("Illegal"),
        "expected Illegal constructor, got: {msg}"
    );
}

#[test]
fn file_list_prototype_has_length_accessor() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "typeof Object.getOwnPropertyDescriptor(FileList.prototype, 'length').get;"
        ),
        "function"
    );
}

#[test]
fn file_list_prototype_has_item_method() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "typeof FileList.prototype.item;"),
        "function"
    );
}

#[test]
fn file_list_prototype_constructor_identity() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "FileList.prototype.constructor === FileList;"
    ));
}

// ---------------------------------------------------------------------------
// HTMLInputElement.files (Phase 3) — empty FileList wrapper, SameObject
//
// The wrapper is always empty until shell-side file picker staging
// lands (defer slot `#11-input-file-shell-staging`).  These tests
// verify the wrapper shape + SameObject identity, NOT actual file
// staging behaviour.
// ---------------------------------------------------------------------------

#[test]
fn input_files_returns_file_list_not_null() {
    assert!(eval_in_doc_bool(
        "let i = document.createElement('input'); i.type = 'file'; \
         i.files instanceof FileList;"
    ));
}

#[test]
fn input_files_default_empty_length_zero() {
    assert_eq!(
        eval_in_doc_number(
            "let i = document.createElement('input'); i.type = 'file'; \
             i.files.length;"
        ),
        0.0
    );
}

#[test]
fn file_list_item_nan_returns_first_entry() {
    // Copilot R1 regression: `item(NaN)` was returning null because
    // the bespoke `if !n.is_finite()` guard early-exited.  Per WebIDL
    // `unsigned long` ToUint32 §3.10.10, NaN → 0 — so on a non-empty
    // list `item(NaN)` returns index 0, matching Chrome / Firefox.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let f = new File(['x'], 'a.txt'); \
         dt.items.add(f); \
         dt.files.item(NaN) === f;"
    ));
}

#[test]
fn file_list_item_negative_one_wraps_uint32() {
    // ToUint32(-1) === 0xFFFFFFFF; on a non-empty list with length
    // 1, that's out-of-range → null (correctly, after the wrap).
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         dt.items.add(new File(['x'], 'a.txt')); \
         dt.files.item(-1) === null;"
    ));
}

#[test]
fn input_files_item_out_of_range_null() {
    assert!(eval_in_doc_bool(
        "let i = document.createElement('input'); i.type = 'file'; \
         i.files.item(0) === null;"
    ));
}

#[test]
fn input_files_same_object_identity() {
    // Spec [SameObject] — `input.files === input.files` per read.
    assert!(eval_in_doc_bool(
        "let i = document.createElement('input'); i.type = 'file'; \
         i.files === i.files;"
    ));
}

#[test]
fn input_files_distinct_per_input_instance() {
    // Two distinct `<input>` elements get distinct FileList wrappers
    // (per-instance SameObject — NOT shared across inputs).
    assert!(eval_in_doc_bool(
        "let a = document.createElement('input'); a.type = 'file'; \
         let b = document.createElement('input'); b.type = 'file'; \
         a.files !== b.files;"
    ));
}

#[test]
fn input_files_same_object_identity_survives_gc() {
    // `[SameObject]` requires `input.files === input.files` even when
    // GC runs between the two reads with no script-side reference
    // holding the intermediate FileList alive.  Pre-fix the cached
    // wrapper would be swept (its only "root" was the cache entry,
    // which the trace walker did not fan out to from the input
    // wrapper) and the second read would allocate a fresh wrapper —
    // breaking identity.
    with_doc_vm(|vm| {
        vm.eval(
            "globalThis.input = document.createElement('input'); \
             input.type = 'file'; \
             globalThis.first = input.files;",
        )
        .unwrap();
        vm.inner.collect_garbage();
        assert!(eval_bool(
            vm,
            "globalThis.first === globalThis.input.files;"
        ));
    });
}

#[test]
fn input_files_available_on_non_file_type_input() {
    // HTML spec: `input.files` returns FileList only when type=file,
    // null otherwise (per §4.10.5.3.10).  Phase 3 implementation
    // ALWAYS returns FileList regardless of type — matches the
    // simpler implementation choice; type=file gating is a small
    // future refinement noted at the accessor.  Verify current
    // behaviour to lock in the design (so a future spec-tightening
    // tightens this test too).
    assert!(eval_in_doc_bool(
        "let i = document.createElement('input'); i.type = 'text'; \
         i.files instanceof FileList;"
    ));
}

// ---------------------------------------------------------------------------
// FileReader — interface, ctor, readyState, accessors (Phase 4)
// ---------------------------------------------------------------------------

#[test]
fn file_reader_ctor_returns_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new FileReader() instanceof FileReader;"
    ));
}

#[test]
fn file_reader_initial_ready_state_empty() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new FileReader().readyState;"), 0.0);
}

#[test]
fn file_reader_initial_result_null() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "new FileReader().result === null;"));
}

#[test]
fn file_reader_initial_error_null() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "new FileReader().error === null;"));
}

#[test]
fn file_reader_constants_on_ctor() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "FileReader.EMPTY;"), 0.0);
    let mut vm2 = Vm::new();
    assert_eq!(eval_number(&mut vm2, "FileReader.LOADING;"), 1.0);
    let mut vm3 = Vm::new();
    assert_eq!(eval_number(&mut vm3, "FileReader.DONE;"), 2.0);
}

#[test]
fn file_reader_constants_on_prototype() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new FileReader().DONE;"), 2.0);
}

#[test]
fn file_reader_must_be_called_with_new() {
    let mut vm = Vm::new();
    let err = vm.eval("FileReader();").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("new"), "expected 'new' error, got: {msg}");
}

// ---------------------------------------------------------------------------
// FileReader — event handler attributes
// ---------------------------------------------------------------------------

#[test]
fn on_handler_default_null() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let r = new FileReader(); \
         r.onload === null && r.onerror === null && r.onabort === null && \
         r.onloadstart === null && r.onloadend === null && r.onprogress === null;"
    ));
}

#[test]
fn on_handler_set_and_get_callable() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let r = new FileReader(); \
         let fn = function() {}; \
         r.onload = fn; \
         r.onload === fn;"
    ));
}

#[test]
fn on_handler_set_non_callable_becomes_null() {
    let mut vm = Vm::new();
    // Spec: only callable values are retained; others null the slot.
    assert!(eval_bool(
        &mut vm,
        "let r = new FileReader(); \
         r.onload = 42; r.onload === null;"
    ));
}

// ---------------------------------------------------------------------------
// FileReader — readAs* + async drain (eval boundary fires task)
// ---------------------------------------------------------------------------

#[test]
fn read_as_text_basic() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._resolved = null; \
         r.onload = function() { globalThis._resolved = r.result; }; \
         r.readAsText(new Blob(['hello']));",
    )
    .unwrap();
    let result = match vm.eval("_resolved;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(result, "hello");
}

#[test]
fn read_as_text_state_transitions_to_done() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._states = []; \
         globalThis._states.push(r.readyState); \
         r.onloadstart = function() { globalThis._states.push(r.readyState); }; \
         r.onload = function() { globalThis._states.push(r.readyState); }; \
         r.readAsText(new Blob(['x']));",
    )
    .unwrap();
    let len = match vm.eval("_states.length;").unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    };
    assert_eq!(len, 3.0);
    // Initial 0 (EMPTY), loadstart fires at 1 (LOADING), load fires at 2 (DONE).
    assert_eq!(
        match vm.eval("_states[0];").unwrap() {
            JsValue::Number(n) => n,
            _ => -1.0,
        },
        0.0
    );
    assert_eq!(
        match vm.eval("_states[1];").unwrap() {
            JsValue::Number(n) => n,
            _ => -1.0,
        },
        1.0
    );
    assert_eq!(
        match vm.eval("_states[2];").unwrap() {
            JsValue::Number(n) => n,
            _ => -1.0,
        },
        2.0
    );
}

#[test]
fn read_as_text_with_encoding_label() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._got = null; \
         r.onload = function() { globalThis._got = r.result; }; \
         let buf = new Uint8Array([0x82, 0xA0]).buffer; \
         r.readAsText(new Blob([buf]), 'shift_jis');",
    )
    .unwrap();
    let got = match vm.eval("_got;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    // Shift_JIS 0x82A0 → U+3042 (HIRAGANA LETTER A).
    assert_eq!(got, "\u{3042}");
}

#[test]
fn read_as_text_utf8_bom_sniff() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._got = null; \
         r.onload = function() { globalThis._got = r.result; }; \
         let buf = new Uint8Array([0xEF, 0xBB, 0xBF, 0x68, 0x69]).buffer; \
         r.readAsText(new Blob([buf]));",
    )
    .unwrap();
    let got = match vm.eval("_got;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    // BOM is consumed by encoding_rs; "hi" remains.
    assert_eq!(got, "hi");
}

#[test]
fn read_as_text_invalid_encoding_label_falls_back_to_utf8() {
    // FileAPI §6.3 step 1: if the user-provided label is not a valid
    // encoding name, fall through to subsequent steps (Blob.type
    // charset → BOM → UTF-8 default) — do NOT throw.
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._got = null; \
         r.onload = function() { globalThis._got = r.result; }; \
         r.readAsText(new Blob(['hello']), 'not-a-real-encoding');",
    )
    .unwrap();
    let got = match vm.eval("_got;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(got, "hello");
}

#[test]
fn read_as_text_blob_type_charset() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._got = null; \
         r.onload = function() { globalThis._got = r.result; }; \
         let buf = new Uint8Array([0x82, 0xA0]).buffer; \
         r.readAsText(new Blob([buf], {type: 'text/plain;charset=shift_jis'}));",
    )
    .unwrap();
    let got = match vm.eval("_got;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(got, "\u{3042}");
}

#[test]
fn read_as_array_buffer_basic() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._len = -1; \
         r.onload = function() { globalThis._len = r.result.byteLength; }; \
         r.readAsArrayBuffer(new Blob(['hello']));",
    )
    .unwrap();
    let len = match vm.eval("_len;").unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    };
    assert_eq!(len, 5.0);
}

#[test]
fn read_as_data_url_basic() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._url = null; \
         r.onload = function() { globalThis._url = r.result; }; \
         r.readAsDataURL(new Blob(['hi'], {type: 'text/plain'}));",
    )
    .unwrap();
    let url = match vm.eval("_url;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    // "hi" → base64 "aGk="
    assert_eq!(url, "data:text/plain;base64,aGk=");
}

#[test]
fn read_as_data_url_empty_type_keeps_semicolon() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._url = null; \
         r.onload = function() { globalThis._url = r.result; }; \
         r.readAsDataURL(new Blob(['hi']));",
    )
    .unwrap();
    let url = match vm.eval("_url;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    // Empty type → `data:;base64,...`
    assert_eq!(url, "data:;base64,aGk=");
}

#[test]
fn read_as_binary_string_byte_to_codepoint() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._s = null; \
         r.onload = function() { globalThis._s = r.result; }; \
         let buf = new Uint8Array([0x00, 0x80, 0xFF]).buffer; \
         r.readAsBinaryString(new Blob([buf]));",
    )
    .unwrap();
    let len = match vm.eval("_s.length;").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    assert_eq!(len, 3.0);
    let code0 = match vm.eval("_s.charCodeAt(0);").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    assert_eq!(code0, 0.0);
    let code2 = match vm.eval("_s.charCodeAt(2);").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    assert_eq!(code2, 255.0);
}

#[test]
fn read_as_text_rejects_non_blob() {
    let mut vm = Vm::new();
    let err = vm
        .eval("new FileReader().readAsText('not a blob');")
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("Blob"), "expected Blob error, got: {msg}");
}

#[test]
fn read_while_loading_throws_invalid_state() {
    let mut vm = Vm::new();
    // Two synchronous readAs* before drain — second should throw.
    let err = vm
        .eval(
            "let r = new FileReader(); \
             r.readAsText(new Blob(['a'])); \
             r.readAsText(new Blob(['b']));",
        )
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("InvalidState") || msg.contains("busy"),
        "expected InvalidStateError, got: {msg}"
    );
}

#[test]
fn abort_during_loading_fires_abort_and_loadend() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._fires = []; \
         r.onabort = function() { globalThis._fires.push('abort'); }; \
         r.onloadend = function() { globalThis._fires.push('loadend'); }; \
         r.onload = function() { globalThis._fires.push('load'); }; \
         r.readAsText(new Blob(['x'])); \
         r.abort();",
    )
    .unwrap();
    let len = match vm.eval("_fires.length;").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    // abort + loadend fired (NOT load — abort_seq invalidates the
    // pending FileRead task).
    assert_eq!(len, 2.0);
    let first = match vm.eval("_fires[0];").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        _ => String::new(),
    };
    let second = match vm.eval("_fires[1];").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        _ => String::new(),
    };
    assert_eq!(first, "abort");
    assert_eq!(second, "loadend");
}

#[test]
fn abort_handler_can_start_new_read_with_fresh_state() {
    // Re-read race: `onabort` calling `readAsText(blob2)` is legal per
    // FileAPI §6.4 — the new read sets state=LOADING with a fresh
    // abort_seq; the stale task from the original (aborted) read
    // silent-discards on drain via abort_seq snapshot mismatch.  After
    // both tasks settle, only the new read's result is observable.
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._loads = []; \
         r.onabort = function() { r.readAsText(new Blob(['second'])); }; \
         r.onload = function() { globalThis._loads.push(r.result); }; \
         r.readAsText(new Blob(['first'])); \
         r.abort();",
    )
    .unwrap();
    // Only ONE onload fires (for the second blob); the first read's
    // task was discarded by the abort_seq guard.
    let count = match vm.eval("_loads.length;").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    assert_eq!(count, 1.0, "expected exactly one onload fire");
    let val = match vm.eval("_loads[0];").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(val, "second");
}

#[test]
fn abort_progress_event_carries_blob_size_as_loaded_total() {
    // Copilot R2 regression: `abort()` cleared target_blob to None
    // BEFORE firing abort + loadend, so `fire_progress_event` saw a
    // missing blob and emitted loaded = total = 0.  Fix captures
    // blob_size BEFORE the clear and threads it through as an
    // override.  Observable: a 5-byte Blob aborted mid-read should
    // report loaded = total = 5 on the abort event.
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._loaded = -1; globalThis._total = -1; \
         r.onabort = function(e) { globalThis._loaded = e.loaded; globalThis._total = e.total; }; \
         r.readAsText(new Blob(['hello'])); \
         r.abort();",
    )
    .unwrap();
    assert_eq!(
        eval_number(&mut vm, "_loaded;"),
        5.0,
        "abort onabort.loaded should be blob byte length"
    );
    assert_eq!(
        eval_number(&mut vm, "_total;"),
        5.0,
        "abort onabort.total should be blob byte length"
    );
}

#[test]
fn abort_when_empty_is_noop() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._abort_fired = false; \
         r.onabort = function() { globalThis._abort_fired = true; }; \
         r.abort();",
    )
    .unwrap();
    let fired = match vm.eval("_abort_fired;").unwrap() {
        JsValue::Boolean(b) => b,
        _ => true,
    };
    assert!(!fired, "abort() on EMPTY state should be no-op");
}

#[test]
fn abort_after_done_is_noop() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._abort_count = 0; \
         r.onabort = function() { globalThis._abort_count++; }; \
         r.onload = function() { r.abort(); }; \
         r.readAsText(new Blob(['x']));",
    )
    .unwrap();
    let count = match vm.eval("_abort_count;").unwrap() {
        JsValue::Number(n) => n,
        _ => -1.0,
    };
    assert_eq!(count, 0.0);
}

#[test]
fn progress_event_carries_loaded_total() {
    let mut vm = Vm::new();
    vm.eval(
        "let r = new FileReader(); \
         globalThis._loaded; globalThis._total; globalThis._lc; \
         r.onprogress = function(e) { \
            globalThis._loaded = e.loaded; \
            globalThis._total = e.total; \
            globalThis._lc = e.lengthComputable; \
         }; \
         r.readAsText(new Blob(['hello world']));",
    )
    .unwrap();
    assert_eq!(
        match vm.eval("_loaded;").unwrap() {
            JsValue::Number(n) => n,
            _ => -1.0,
        },
        11.0
    );
    assert_eq!(
        match vm.eval("_total;").unwrap() {
            JsValue::Number(n) => n,
            _ => -1.0,
        },
        11.0
    );
    assert!(matches!(vm.eval("_lc;").unwrap(), JsValue::Boolean(true)));
}

// ---------------------------------------------------------------------------
// DataTransfer 3-site wiring (Phase 5) — closes
// `#11-data-transfer-file-paired` defer slot
// ---------------------------------------------------------------------------

#[test]
fn data_transfer_files_returns_file_list_not_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); dt.files instanceof FileList;"
    ));
}

#[test]
fn data_transfer_files_initial_empty() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new DataTransfer().files.length;"),
        0.0
    );
}

#[test]
fn data_transfer_files_same_object_until_mutation() {
    let mut vm = Vm::new();
    // Two reads before any mutation return the same wrapper.
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); dt.files === dt.files;"
    ));
}

#[test]
fn data_transfer_add_file_appends_to_files() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "let dt = new DataTransfer(); \
             dt.items.add(new File(['x'], 'a.txt')); \
             dt.files.length;"
        ),
        1.0
    );
}

#[test]
fn data_transfer_add_file_then_get_as_file_round_trip() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let f = new File(['x'], 'a.txt'); \
         dt.items.add(f); \
         dt.items[0] !== undefined ? dt.items.length === 1 : true;"
    ));
    // dt.items[0] is undefined per pending indexed-exotic, but
    // `.item(0)` via DataTransferItemList works in principle —
    // however DataTransferItemList doesn't expose `.item(i)` either
    // per its sparse IDL.  The cleanest round-trip path is via
    // files.item(0).
    let mut vm2 = Vm::new();
    assert!(eval_bool(
        &mut vm2,
        "let dt = new DataTransfer(); \
         let f = new File(['x'], 'a.txt'); \
         dt.items.add(f); \
         dt.files.item(0) === f;"
    ));
}

#[test]
fn data_transfer_add_string_does_not_count_as_file() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "let dt = new DataTransfer(); \
             dt.items.add('hello', 'text/plain'); \
             dt.files.length;"
        ),
        0.0
    );
}

#[test]
fn data_transfer_add_blob_falls_through_to_string_overload() {
    let mut vm = Vm::new();
    // Blob is NOT a File at IDL overload-resolution level here —
    // should fall through to (DOMString, DOMString) overload which
    // requires 2 args.  With only 1 arg → TypeError.
    let err = vm
        .eval(
            "let dt = new DataTransfer(); \
             dt.items.add(new Blob(['x']));",
        )
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("2 arguments"),
        "expected 2-arg error, got: {msg}"
    );
}

#[test]
fn data_transfer_files_wrapper_invalidates_on_add() {
    let mut vm = Vm::new();
    // .files identity should NOT survive a mutation that touches
    // file entries (Chrome behaviour).
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let pre = dt.files; \
         dt.items.add(new File(['x'], 'a.txt')); \
         let post = dt.files; \
         pre !== post && post.length === 1;"
    ));
}

#[test]
fn data_transfer_remove_duplicate_file_keeps_sibling_copies() {
    // Regression: adding the same File twice then removing index 0
    // should leave items.length == 1 AND files.length == 1.  A
    // `retain(|id| id != file_id)` on file_entries would wrongly
    // evict both copies, breaking the items↔files invariant.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let f = new File(['x'], 'a.txt'); \
         dt.items.add(f); dt.items.add(f); \
         dt.items.remove(0); \
         dt.items.length === 1 && dt.files.length === 1 && dt.files.item(0) === f;"
    ));
}

#[test]
fn data_transfer_files_wrapper_invalidates_on_clear() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         dt.items.add(new File(['x'], 'a.txt')); \
         let pre = dt.files; \
         dt.items.clear(); \
         let post = dt.files; \
         pre !== post && post.length === 0;"
    ));
}

#[test]
fn data_transfer_item_get_as_file_for_string_kind_null() {
    // String-kind entries: getAsFile() returns null.  Round-trip
    // via DataTransferItem requires reaching items[i] — since
    // indexed-exotic is deferred, use the `add` return value
    // (it's the newly-added DataTransferItem wrapper).
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let item = dt.items.add('hi', 'text/plain'); \
         item.getAsFile() === null;"
    ));
}

#[test]
fn data_transfer_item_get_as_file_for_file_kind_returns_file() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         let f = new File(['x'], 'a.txt'); \
         let item = dt.items.add(f); \
         item.getAsFile() === f;"
    ));
}

#[test]
fn data_transfer_types_includes_files_when_file_added() {
    let mut vm = Vm::new();
    // HTML §6.2: when any File entry is present, `types` includes
    // the literal "Files".
    assert!(eval_bool(
        &mut vm,
        "let dt = new DataTransfer(); \
         dt.items.add(new File(['x'], 'a.txt')); \
         dt.types.indexOf('Files') >= 0;"
    ));
}

#[test]
fn file_proto_name_rejects_plain_blob() {
    let mut vm = Vm::new();
    // File-specific accessor should reject Blob receiver (Blob is
    // NOT a File).
    let err = vm
        .eval(
            "Object.getOwnPropertyDescriptor(File.prototype, 'name')\
              .get.call(new Blob([]));",
        )
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("non-File"),
        "expected non-File error, got: {msg}"
    );
}
