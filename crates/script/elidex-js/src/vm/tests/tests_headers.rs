//! `Headers` interface tests (WHATWG Fetch §5.2).
//!
//! Covers construction forms, mutation methods, iteration (sort +
//! combine with `set-cookie` exception), name/value validation,
//! and `[Symbol.iterator]` identity.

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

#[test]
fn ctor_empty_produces_empty_list() {
    let mut vm = Vm::new();
    // `entries().next().done === true` for an empty Headers.
    assert!(eval_bool(
        &mut vm,
        "var h = new Headers(); h.entries().next().done;"
    ));
}

#[test]
fn ctor_from_record_lowercases_name() {
    let mut vm = Vm::new();
    // Record<string, string> init: `"Content-Type": "text/plain"`
    // → stored as `content-type`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers({'Content-Type': 'text/plain'}); h.get('content-type');"
        ),
        "text/plain"
    );
}

#[test]
fn ctor_from_array_of_pairs() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers([['X-A', '1'], ['X-B', '2']]); h.get('x-a') + ',' + h.get('x-b');"
        ),
        "1,2"
    );
}

#[test]
fn ctor_from_other_headers_copies_entries() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Headers({'x-a': '1'}); var b = new Headers(a); b.get('x-a');"
        ),
        "1"
    );
}

#[test]
fn ctor_rejects_non_object_primitive() {
    let mut vm = Vm::new();
    // Passing a string primitive → TypeError (WebIDL record coercion).
    assert!(eval_bool(
        &mut vm,
        "var r = false; try { new Headers('str'); } catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn append_multiple_values_joins_with_comma_space() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers(); h.append('x-a', '1'); h.append('x-a', '2'); h.get('x-a');"
        ),
        "1, 2"
    );
}

#[test]
fn set_replaces_existing_entries() {
    let mut vm = Vm::new();
    // After two appends + one set, `get` returns only the set value.
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers(); \
             h.append('x-a', '1'); \
             h.append('x-a', '2'); \
             h.set('x-a', '3'); \
             h.get('x-a');"
        ),
        "3"
    );
}

#[test]
fn delete_removes_every_occurrence() {
    let mut vm = Vm::new();
    // `get` returns null after delete (WHATWG §5.2 "get a header").
    // Compare with `== null` since JsValue::Null serialises to a
    // typeof check below.
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers(); \
             h.append('x-a', '1'); \
             h.append('x-a', '2'); \
             h.delete('x-a'); \
             typeof h.get('x-a') === 'object' && h.get('x-a') === null ? 'null' : 'not-null';"
        ),
        "null"
    );
}

#[test]
fn has_is_case_insensitive() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var h = new Headers({'Content-Type': 'x'}); h.has('CONTENT-TYPE');"
    ));
}

#[test]
fn get_set_cookie_returns_each_value_separately() {
    let mut vm = Vm::new();
    // Two separate `Set-Cookie` entries should surface as two
    // array elements (WHATWG §5.2 "get set-cookie"): unlike `get`
    // they are not joined.
    assert_eq!(
        eval_number(
            &mut vm,
            "var h = new Headers(); \
             h.append('set-cookie', 'a=1'); \
             h.append('set-cookie', 'b=2'); \
             h.getSetCookie().length;"
        ),
        2.0
    );
}

#[test]
fn iteration_sorts_by_lowercase_name() {
    let mut vm = Vm::new();
    // Insertion order x-b, x-a, x-c → iteration order x-a, x-b, x-c
    // (sort-and-combine).
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers(); \
             h.append('x-b', 'B'); \
             h.append('x-a', 'A'); \
             h.append('x-c', 'C'); \
             var names = []; \
             for (var pair of h.entries()) { names.push(pair[0]); } \
             names.join(',');"
        ),
        "x-a,x-b,x-c"
    );
}

#[test]
fn foreach_receives_value_name_headers_args() {
    let mut vm = Vm::new();
    // WHATWG §5.2 `forEach(callback, thisArg)` — callback arity
    // matches Chrome: `(value, name, headers) => ...`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var h = new Headers({'x-a': 'v1'}); \
             var out = ''; \
             h.forEach(function(value, name, headers) { \
                 out = name + '=' + value + ',' + (headers === h); \
             }); \
             out;"
        ),
        "x-a=v1,true"
    );
}

#[test]
fn append_rejects_invalid_name() {
    let mut vm = Vm::new();
    // CR/LF inside a name violates RFC 7230 token.
    assert!(eval_bool(
        &mut vm,
        "var h = new Headers(); var r = false; \
         try { h.append('X\\r\\nY', '1'); } catch (e) { r = e instanceof TypeError; } \
         r;"
    ));
}

#[test]
fn append_rejects_invalid_value_with_nul() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var h = new Headers(); var r = false; \
         try { h.append('x-a', 'v\\0'); } catch (e) { r = e instanceof TypeError; } \
         r;"
    ));
}

#[test]
fn symbol_iterator_aliases_entries() {
    let mut vm = Vm::new();
    // Per WHATWG §5.2: `Headers.prototype[@@iterator] ===
    // Headers.prototype.entries`.
    assert!(eval_bool(
        &mut vm,
        "Headers.prototype[Symbol.iterator] === Headers.prototype.entries;"
    ));
}
