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

use super::super::value::JsValue;
use super::super::Vm;

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
    // Default is `Date.now()`-equivalent (start_instant.elapsed() in
    // our VM).  Just check that it's a finite number (not NaN).
    let n = eval_number(&mut vm, "new File([], 'x').lastModified;");
    assert!(n.is_finite(), "expected finite lastModified, got {n}");
    assert!(n >= 0.0, "expected non-negative lastModified, got {n}");
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
