//! `URLSearchParams` tests (WHATWG URL §6).
//!
//! Covers ctor (string / array-of-pairs / iterable / record /
//! URLSearchParams clone), `append` / `delete` / `get` / `getAll`
//! / `has` / `set` / `sort` / `toString`, iteration, and the size
//! getter.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

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

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

#[test]
fn ctor_empty() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new URLSearchParams().size;"), 0.0);
}

#[test]
fn ctor_from_string_strips_leading_question_mark() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URLSearchParams('?a=1&b=2').toString();"),
        "a=1&b=2"
    );
}

#[test]
fn ctor_from_string_decodes_percent_and_plus() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URLSearchParams('q=hello+world&n=%E2%9C%93').get('q');"
        ),
        "hello world"
    );
    assert_eq!(
        eval_string(
            &mut vm,
            "new URLSearchParams('q=hello+world&n=%E2%9C%93').get('n');"
        ),
        "✓"
    );
}

#[test]
fn ctor_from_array_of_pairs() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URLSearchParams([['a', '1'], ['b', '2']]).toString();"
        ),
        "a=1&b=2"
    );
}

#[test]
fn ctor_from_record_object() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URLSearchParams({a: '1', b: '2'}).toString();"),
        "a=1&b=2"
    );
}

#[test]
fn ctor_clone_from_url_search_params() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let a = new URLSearchParams('x=1&y=2'); \
             let b = new URLSearchParams(a); \
             a.append('x', '3'); b.toString();"
        ),
        "x=1&y=2"
    );
}

#[test]
fn ctor_pair_arity_error() {
    let mut vm = Vm::new();
    let result = vm.eval("new URLSearchParams([['a']]);");
    assert!(result.is_err());
}

#[test]
fn append_adds_in_insertion_order() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams(); p.append('a', '1'); p.append('a', '2'); \
             p.append('b', '3'); p.toString();"
        ),
        "a=1&a=2&b=3"
    );
}

#[test]
fn delete_removes_all_with_name() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&a=2&b=3'); p.delete('a'); p.toString();"
        ),
        "b=3"
    );
}

#[test]
fn delete_with_value_removes_only_matching() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&a=2&a=3'); p.delete('a', '2'); p.toString();"
        ),
        "a=1&a=3"
    );
}

#[test]
fn get_returns_first_value_or_null() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URLSearchParams('a=1&a=2').get('a');"),
        "1"
    );
    let result = vm.eval("new URLSearchParams('a=1').get('b');").unwrap();
    assert!(matches!(result, JsValue::Null));
}

#[test]
fn get_all_returns_array_of_matches() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "JSON.stringify(new URLSearchParams('a=1&a=2&b=3').getAll('a'));"
        ),
        "[\"1\",\"2\"]"
    );
}

#[test]
fn has_value_filter() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new URLSearchParams('a=1&a=2').has('a', '2');"
    ));
    assert!(!eval_bool(
        &mut vm,
        "new URLSearchParams('a=1&a=2').has('a', '3');"
    ));
}

#[test]
fn set_replaces_first_and_drops_rest() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2&a=3'); p.set('a', '9'); p.toString();"
        ),
        "a=9&b=2"
    );
}

#[test]
fn set_appends_when_absent() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1'); p.set('b', '2'); p.toString();"
        ),
        "a=1&b=2"
    );
}

#[test]
fn sort_is_stable_by_name() {
    let mut vm = Vm::new();
    // Same-name entries preserve insertion order.
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('b=1&a=2&b=3&a=4'); p.sort(); p.toString();"
        ),
        "a=2&a=4&b=1&b=3"
    );
}

#[test]
fn to_string_percent_encodes_special_chars() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams(); p.append('q', 'hello world'); p.toString();"
        ),
        "q=hello+world"
    );
}

#[test]
fn for_each_invokes_callback_in_order() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2'); let acc = ''; \
             p.forEach((value, name) => { acc += name + '=' + value + ';'; }); acc;"
        ),
        "a=1;b=2;"
    );
}

#[test]
fn entries_iterator_yields_pairs() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2'); let acc = ''; \
             for (let [k, v] of p) { acc += k + '=' + v + ';'; } acc;"
        ),
        "a=1;b=2;"
    );
}

#[test]
fn keys_iterator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2'); let acc = ''; \
             for (let k of p.keys()) { acc += k + ';'; } acc;"
        ),
        "a;b;"
    );
}

#[test]
fn values_iterator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2'); let acc = ''; \
             for (let v of p.values()) { acc += v + ';'; } acc;"
        ),
        "1;2;"
    );
}

#[test]
fn iterator_alias_to_entries() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "URLSearchParams.prototype[Symbol.iterator] === URLSearchParams.prototype.entries;"
    ));
}

#[test]
fn size_getter_reflects_entry_count() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "let p = new URLSearchParams('a=1&b=2'); p.append('c', '3'); p.size;"
        ),
        3.0
    );
}

#[test]
fn brand_check_throws_on_alien_receiver() {
    let mut vm = Vm::new();
    let result = vm.eval("URLSearchParams.prototype.toString.call({});");
    assert!(result.is_err());
}

#[test]
fn ctor_requires_new() {
    let mut vm = Vm::new();
    let result = vm.eval("URLSearchParams('a=1');");
    assert!(result.is_err());
}
