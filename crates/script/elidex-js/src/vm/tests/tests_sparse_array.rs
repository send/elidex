//! Tests for sparse array support (JsValue::Empty).

use crate::vm::value::{JsValue, VmError};
use crate::vm::Vm;

fn eval(source: &str) -> Result<JsValue, VmError> {
    let mut vm = Vm::new();
    vm.eval(source)
}

fn eval_number(source: &str) -> f64 {
    match eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_bool(source: &str) -> bool {
    match eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_str(source: &str) -> String {
    let mut vm = Vm::new();
    let result = vm.eval(source).unwrap();
    match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

// ── Elision ─────────────────────────────────────────────────────────────

#[test]
fn elision_length() {
    assert_eq!(eval_number("[1,,3].length;"), 3.0);
}

#[test]
fn elision_hole_reads_as_undefined() {
    assert!(matches!(eval("[1,,3][1];"), Ok(JsValue::Undefined)));
}

#[test]
fn elision_trailing_hole() {
    // [1,2,] has length 2 (trailing comma is not an elision per spec)
    // [1,2,,] has length 3 (trailing elision)
    assert_eq!(eval_number("[1,2,,].length;"), 3.0);
}

// ── Hole vs Undefined ───────────────────────────────────────────────────

#[test]
fn hole_not_in_array() {
    // Hole: `1 in [1,,3]` → false
    assert!(!eval_bool("1 in [1,,3];"));
}

#[test]
fn undefined_in_array() {
    // Explicit undefined: `1 in [1,undefined,3]` → true
    assert!(eval_bool("1 in [1,undefined,3];"));
}

#[test]
fn filled_index_in_array() {
    assert!(eval_bool("0 in [1,,3];"));
    assert!(eval_bool("2 in [1,,3];"));
}

// ── delete ──────────────────────────────────────────────────────────────

#[test]
fn delete_creates_hole() {
    assert!(eval_bool("var a = [1,2,3]; delete a[1]; !(1 in a);"));
}

#[test]
fn delete_preserves_length() {
    assert_eq!(eval_number("var a = [1,2,3]; delete a[1]; a.length;"), 3.0);
}

#[test]
fn delete_reads_undefined() {
    assert!(matches!(
        eval("var a = [1,2,3]; delete a[1]; a[1];"),
        Ok(JsValue::Undefined)
    ));
}

// ── Array(n) constructor ────────────────────────────────────────────────

#[test]
fn array_constructor_length() {
    assert_eq!(eval_number("Array(3).length;"), 3.0);
}

#[test]
fn array_constructor_sparse() {
    // Indices in Array(3) should be holes
    assert!(!eval_bool("0 in Array(3);"));
}

#[test]
fn array_constructor_with_elements() {
    assert_eq!(eval_number("Array(1,2,3).length;"), 3.0);
    assert_eq!(eval_number("Array(1,2,3)[0];"), 1.0);
}

#[test]
fn array_constructor_single_non_number() {
    assert_eq!(eval_number("Array('hello').length;"), 1.0);
    assert_eq!(eval_str("Array('hello')[0];"), "hello");
}

#[test]
fn array_constructor_invalid_length() {
    assert!(eval("Array(-1);").is_err());
    assert!(eval("Array(1.5);").is_err());
}

#[test]
fn array_constructor_is_array() {
    assert!(eval_bool("Array.isArray(Array(3));"));
}

// ── for-in (skips holes) ────────────────────────────────────────────────

#[test]
fn for_in_skips_holes() {
    assert_eq!(
        eval_str("var r = ''; for (var k in [1,,3]) { r += k; } r;"),
        "02",
    );
}

#[test]
fn for_in_skips_deleted_holes() {
    assert_eq!(
        eval_str("var a = [1,2,3]; delete a[1]; var r = ''; for (var k in a) { r += k; } r;"),
        "02",
    );
}

// ── for-of (yields undefined for holes) ─────────────────────────────────

#[test]
fn for_of_yields_undefined_for_hole() {
    // for-of visits all indices; holes become undefined
    assert_eq!(
        eval_number("var s = 0; for (var x of [1,,3]) { s += (x === undefined ? 10 : x); } s;"),
        14.0, // 1 + 10 + 3
    );
}

// ── set_element resize fills with holes ─────────────────────────────────

#[test]
fn resize_creates_holes() {
    // arr[5] = 'x' on empty array creates holes at indices 0-4
    assert!(!eval_bool("var a = []; a[3] = 'x'; 0 in a;"));
    assert!(!eval_bool("var a = []; a[3] = 'x'; 1 in a;"));
    assert!(!eval_bool("var a = []; a[3] = 'x'; 2 in a;"));
    assert!(eval_bool("var a = []; a[3] = 'x'; 3 in a;"));
}

#[test]
fn resize_preserves_length() {
    assert_eq!(eval_number("var a = []; a[5] = 1; a.length;"), 6.0);
}

// ── JSON ────────────────────────────────────────────────────────────────

#[test]
fn json_stringify_holes_to_null() {
    assert_eq!(eval_str("JSON.stringify([1,,3]);"), "[1,null,3]");
}

#[test]
fn json_stringify_deleted_hole() {
    assert_eq!(
        eval_str("var a = [1,2,3]; delete a[1]; JSON.stringify(a);"),
        "[1,null,3]",
    );
}

// ── Abstract equality with holes ────────────────────────────────────────

#[test]
fn hole_abstract_eq_null() {
    // Hole reads as undefined, and undefined == null is true.
    assert!(eval_bool("[1,,3][1] == null;"));
}

#[test]
fn hole_abstract_eq_undefined() {
    assert!(eval_bool("[1,,3][1] == undefined;"));
}

#[test]
fn hole_strict_eq_undefined() {
    assert!(eval_bool("[1,,3][1] === undefined;"));
}
