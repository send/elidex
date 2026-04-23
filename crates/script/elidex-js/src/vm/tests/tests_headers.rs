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

#[test]
fn ctor_consumes_user_iterable_as_sequence() {
    let mut vm = Vm::new();
    // WebIDL `HeadersInit` union (Copilot R17.1): any object whose
    // `[Symbol.iterator]` is callable must be consumed as
    // `sequence<sequence<ByteString>>`, not silently dropped to
    // the record branch.  Custom iterable yields two 2-tuples.
    assert_eq!(
        eval_string(
            &mut vm,
            "var src = { [Symbol.iterator]() { \
                 var i = 0; \
                 var pairs = [['X-A', '1'], ['X-B', '2']]; \
                 return { next() { \
                     return i < pairs.length \
                         ? { value: pairs[i++], done: false } \
                         : { value: undefined, done: true }; \
                 } }; \
             } }; \
             var h = new Headers(src); h.get('x-a') + ',' + h.get('x-b');"
        ),
        "1,2"
    );
}

#[test]
fn ctor_iterable_yielding_non_pair_throws() {
    let mut vm = Vm::new();
    // Inner element must be a length-2 array; a 3-tuple must be
    // rejected (WebIDL `sequence<sequence<ByteString>>` inner
    // arity check routed through `validate_pair_entry`).
    assert!(eval_bool(
        &mut vm,
        "var src = { [Symbol.iterator]() { \
             var done = false; \
             return { next() { \
                 if (done) return { value: undefined, done: true }; \
                 done = true; \
                 return { value: ['a', 'b', 'c'], done: false }; \
             } }; \
         } }; \
         var r = false; try { new Headers(src); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn ctor_symbol_iterator_null_falls_through_to_record() {
    let mut vm = Vm::new();
    // `[Symbol.iterator]: null` matches `GetMethod`'s
    // "null/undefined ⇒ no method" rule (ES §7.3.11) and must
    // therefore be interpreted as a plain record.
    assert_eq!(
        eval_string(
            &mut vm,
            "var src = { 'x-a': '1', [Symbol.iterator]: null }; \
             var h = new Headers(src); h.get('x-a');"
        ),
        "1"
    );
}

#[test]
fn ctor_symbol_iterator_non_callable_throws() {
    let mut vm = Vm::new();
    // A non-null, non-callable `@@iterator` is a TypeError — the
    // WebIDL union resolution picks the sequence branch and then
    // fails the IsCallable check.
    assert!(eval_bool(
        &mut vm,
        "var src = { [Symbol.iterator]: 42 }; \
         var r = false; try { new Headers(src); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn ctor_inner_pair_accepts_custom_iterable() {
    let mut vm = Vm::new();
    // WebIDL `sequence<sequence<ByteString>>`: the inner pair may
    // be any iterable yielding exactly two items — not only a
    // literal two-element Array (R22.1).  Here the outer is an
    // Array, the inner is a user `[Symbol.iterator]` object.
    assert_eq!(
        eval_string(
            &mut vm,
            "var inner = { [Symbol.iterator]() { \
                 var step = 0; \
                 return { next() { \
                     step++; \
                     if (step === 1) return { value: 'X-A', done: false }; \
                     if (step === 2) return { value: '1', done: false }; \
                     return { value: undefined, done: true }; \
                 } }; \
             } }; \
             var h = new Headers([inner]); h.get('x-a');"
        ),
        "1"
    );
}

#[test]
fn ctor_inner_pair_arity_three_throws() {
    let mut vm = Vm::new();
    // Inner iterable yielding three items is a spec TypeError
    // ("must contain iterables of length 2").  Closes inner
    // iterator before propagating (R22.1 early-exit path).
    assert!(eval_bool(
        &mut vm,
        "globalThis.returnCalled = false; \
         var inner = { [Symbol.iterator]() { \
             var step = 0; \
             return { \
                 next() { \
                     step++; \
                     if (step === 1) return { value: 'a', done: false }; \
                     if (step === 2) return { value: 'b', done: false }; \
                     return { value: 'c', done: false }; \
                 }, \
                 return() { \
                     globalThis.returnCalled = true; \
                     return { value: undefined, done: true }; \
                 } \
             }; \
         } }; \
         var threw = false; \
         try { new Headers([inner]); } \
         catch (e) { threw = e instanceof TypeError; } \
         threw && globalThis.returnCalled;"
    ));
}

#[test]
fn ctor_inner_pair_arity_one_throws() {
    let mut vm = Vm::new();
    // Inner iterable yielding only one item (then done) is also a
    // TypeError.  No IteratorClose here — `done` is normal
    // completion per §7.4.6.
    assert!(eval_bool(
        &mut vm,
        "var inner = { [Symbol.iterator]() { \
             var step = 0; \
             return { next() { \
                 step++; \
                 if (step === 1) return { value: 'only', done: false }; \
                 return { value: undefined, done: true }; \
             } }; \
         } }; \
         var r = false; try { new Headers([inner]); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn ctor_iterable_abrupt_completion_calls_return() {
    let mut vm = Vm::new();
    // ES §7.4.6 / WebIDL sequence conversion: if a yielded pair
    // fails validation (abrupt completion of the for-of-like body),
    // `IteratorClose` must invoke the iterator's `.return()`.  This
    // regression test hands the ctor an iterator whose second value
    // is an invalid 3-tuple; `validate_pair_entry` rejects it, which
    // triggers the abrupt-completion path.  `.return()` records its
    // invocation by setting `globalThis.returnCalled = true`; the
    // outer expression asserts both that the ctor threw TypeError
    // **and** that `.return()` was called (R18.1).
    assert!(eval_bool(
        &mut vm,
        "globalThis.returnCalled = false; \
         var src = { [Symbol.iterator]() { \
             var step = 0; \
             return { \
                 next() { \
                     step++; \
                     if (step === 1) return { value: ['a', '1'], done: false }; \
                     return { value: ['b', 'c', 'd'], done: false }; \
                 }, \
                 return() { \
                     globalThis.returnCalled = true; \
                     return { value: undefined, done: true }; \
                 } \
             }; \
         } }; \
         var threw = false; \
         try { new Headers(src); } \
         catch (e) { threw = e instanceof TypeError; } \
         threw && globalThis.returnCalled;"
    ));
}
