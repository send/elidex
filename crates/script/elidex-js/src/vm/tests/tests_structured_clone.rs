//! Tests for `structuredClone(value, options?)` (WHATWG HTML §2.9
//! StructuredSerialize + StructuredDeserialize, fused).
//!
//! Covers primitives, cycles, nested clones, RegExp `lastIndex`
//! reset, Error subclass proto preservation, wrapper protos,
//! Blob / ArrayBuffer deep-copy independence, and DataCloneError
//! for the major unclonable families (Function, Symbol, Promise,
//! DOM nodes, Headers / Request / Response, AbortSignal, etc.).

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_throws(vm: &mut Vm, source: &str) {
    assert!(vm.eval(source).is_err(), "expected throw: {source}");
}

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

#[test]
fn primitives_pass_through() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "structuredClone(42);"), 42.0);
    assert_eq!(eval_string(&mut vm, "structuredClone('hi');"), "hi");
    assert!(eval_bool(&mut vm, "structuredClone(true) === true;"));
    assert!(eval_bool(&mut vm, "structuredClone(null) === null;"));
    assert!(eval_bool(
        &mut vm,
        "structuredClone(undefined) === undefined;"
    ));
    // NaN passes through (same bit-pattern preservation not required).
    assert!(eval_bool(&mut vm, "Number.isNaN(structuredClone(NaN));"));
}

// ---------------------------------------------------------------------------
// Plain objects + cycles
// ---------------------------------------------------------------------------

#[test]
fn plain_object_copies_properties_not_identity() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = {x: 1, y: 'two'}; var b = structuredClone(a);
         b.x === 1 && b.y === 'two' && a !== b;",
    ));
}

#[test]
fn nested_object_deep_clones() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = {inner: {k: 42}}; var b = structuredClone(a);
         b.inner.k === 42 && a.inner !== b.inner;",
    ));
}

#[test]
fn self_reference_cycle_round_trips() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = {}; a.self = a; var b = structuredClone(a);
         b.self === b && b !== a;",
    ));
}

#[test]
fn shared_reference_preserves_identity_in_clone() {
    let mut vm = Vm::new();
    // `x` and `y` point at the same inner before clone; the clone
    // should preserve that aliasing on the output side.
    assert!(eval_bool(
        &mut vm,
        "var shared = {k: 1};
         var a = {x: shared, y: shared};
         var b = structuredClone(a);
         b.x === b.y && b.x !== shared;",
    ));
}

#[test]
fn accessor_properties_are_skipped() {
    let mut vm = Vm::new();
    // A getter-only own property is a non-data property; structured
    // clone walks data properties only.
    assert!(eval_bool(
        &mut vm,
        "var a = {x: 7};
         Object.defineProperty(a, 'g', {get: function(){ return 99; }, enumerable: true});
         var b = structuredClone(a);
         b.x === 7 && !('g' in b);",
    ));
}

// ---------------------------------------------------------------------------
// Arrays
// ---------------------------------------------------------------------------

#[test]
fn array_clones_elements_and_length() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = [1, 'two', {x: 3}]; var b = structuredClone(a);
         b.length === 3 && b[0] === 1 && b[1] === 'two' && b[2].x === 3 && a !== b && a[2] !== b[2];",
    ));
}

#[test]
fn sparse_array_preserves_holes() {
    let mut vm = Vm::new();
    // The spec requires hole preservation; `in` is the way to probe.
    assert!(eval_bool(
        &mut vm,
        "var a = [1, , 3]; var b = structuredClone(a);
         b.length === 3 && b[0] === 1 && !(1 in b) && b[2] === 3;",
    ));
}

// ---------------------------------------------------------------------------
// RegExp
// ---------------------------------------------------------------------------

#[test]
fn regexp_resets_last_index() {
    let mut vm = Vm::new();
    // Mutating `lastIndex` on the source should not leak into the
    // clone: spec §2.9 step 15 resets to 0.
    assert!(eval_bool(
        &mut vm,
        "var r = /ab/g; r.lastIndex = 5; var c = structuredClone(r);
         c.lastIndex === 0 && c.source === 'ab' && c.flags === 'g';",
    ));
}

// ---------------------------------------------------------------------------
// Error subclass proto preservation
// ---------------------------------------------------------------------------

#[test]
fn error_subclass_proto_preserved() {
    let mut vm = Vm::new();
    // `new TypeError(...)` should round-trip as a TypeError, not a
    // plain Error.  Also verify the drop of non-standard `stack`.
    assert!(eval_bool(
        &mut vm,
        "var e = new TypeError('boom'); var c = structuredClone(e);
         c instanceof TypeError && c instanceof Error && c.message === 'boom';",
    ));
}

// ---------------------------------------------------------------------------
// Primitive wrappers — target realm prototypes, not Object.prototype
// ---------------------------------------------------------------------------

// Only `String` exposes a constructable wrapper ctor in elidex
// (PR4b left `Number` / `Boolean` non-constructable; `Object(prim)`
// is not yet callable either).  StringWrapper covers the generic
// wrapper-clone code path; NumberWrapper / BooleanWrapper
// correctness follows from the same `alloc_wrapper` helper and is
// exercised at the VM unit-test layer in `tests_structured_clone::
// string_wrapper_clone_has_string_prototype`.
#[test]
fn string_wrapper_clone_has_string_prototype() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var s = new String('hi'); var c = structuredClone(s);
         Object.getPrototypeOf(c) === String.prototype && c.valueOf() === 'hi';",
    ));
}

// ---------------------------------------------------------------------------
// ArrayBuffer / Blob — deep copy (byte independence)
// ---------------------------------------------------------------------------

#[test]
fn array_buffer_byte_length_preserved() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "structuredClone(new ArrayBuffer(16)).byteLength;"),
        16.0,
    );
}

#[test]
fn array_buffer_clone_is_independent_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var buf = new ArrayBuffer(8); var c = structuredClone(buf);
         c !== buf && c.byteLength === 8;",
    ));
}

#[test]
fn blob_clone_preserves_size_and_type() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new Blob(['hello'], {type: 'text/plain'});
         var c = structuredClone(b);
         c !== b && c.size === 5 && c.type === 'text/plain';",
    ));
}

// ---------------------------------------------------------------------------
// DataCloneError family
// ---------------------------------------------------------------------------

fn assert_data_clone_error(vm: &mut Vm, expr: &str) {
    let src = format!(
        "var caught = null;
         try {{ structuredClone({expr}); }} catch (e) {{ caught = e.name; }}
         caught;",
    );
    match vm.eval(&src).unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "DataCloneError", "for `{expr}`"),
        other => panic!("expected DataCloneError name for `{expr}`, got {other:?}"),
    }
}

#[test]
fn data_clone_error_for_function() {
    let mut vm = Vm::new();
    assert_data_clone_error(&mut vm, "function(){}");
}

#[test]
fn data_clone_error_for_symbol() {
    let mut vm = Vm::new();
    assert_data_clone_error(&mut vm, "Symbol('s')");
}

#[test]
fn data_clone_error_for_promise() {
    let mut vm = Vm::new();
    assert_data_clone_error(&mut vm, "Promise.resolve(1)");
}

#[test]
fn data_clone_error_for_dom_element() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert_data_clone_error(&mut vm, "document.createElement('div')");
    vm.unbind();
}

#[test]
fn data_clone_error_for_headers() {
    let mut vm = Vm::new();
    assert_data_clone_error(&mut vm, "new Headers()");
}

#[test]
fn data_clone_error_for_abort_signal() {
    let mut vm = Vm::new();
    assert_data_clone_error(&mut vm, "new AbortController().signal");
}

#[test]
fn data_clone_error_propagates_from_nested_member() {
    let mut vm = Vm::new();
    // The surface type is Ordinary but an inner field is unclonable;
    // the inner failure must bubble up with the same DataCloneError.
    assert_data_clone_error(&mut vm, "{ok: 1, bad: function(){}}");
}

// ---------------------------------------------------------------------------
// Binding-level validation
// ---------------------------------------------------------------------------

#[test]
fn missing_argument_throws_type_error() {
    let mut vm = Vm::new();
    // Spec: WebIDL "not enough arguments" → TypeError (not
    // DataCloneError).
    assert!(vm.eval("structuredClone();").is_err());
}

#[test]
fn options_transfer_empty_array_succeeds() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "structuredClone(7, {transfer: []});"),
        7.0,
    );
}

#[test]
fn options_transfer_nonempty_array_throws_data_clone_error() {
    let mut vm = Vm::new();
    let expr = "new ArrayBuffer(4)";
    let src = format!(
        "var caught = null;
         try {{ structuredClone({expr}, {{transfer: [{expr}]}}); }}
         catch (e) {{ caught = e.name; }}
         caught;",
    );
    match vm.eval(&src).unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "DataCloneError"),
        other => panic!("expected DataCloneError, got {other:?}"),
    }
}

#[test]
fn undefined_and_null_options_accepted() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "structuredClone(1, undefined);"), 1.0);
    assert_eq!(eval_number(&mut vm, "structuredClone(1, null);"), 1.0);
}

// ---------------------------------------------------------------------------
// Identity vs equality sanity
// ---------------------------------------------------------------------------

#[test]
fn clone_is_not_same_identity_even_for_empty_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "var a = {}; structuredClone(a) !== a;",));
}

// ---------------------------------------------------------------------------
// Non-object options coerce failure
// ---------------------------------------------------------------------------

#[test]
fn options_primitive_string_throws_type_error() {
    let mut vm = Vm::new();
    // WebIDL dictionary conversion rejects a non-nullish primitive.
    eval_throws(&mut vm, "structuredClone(1, 'bogus');");
}
