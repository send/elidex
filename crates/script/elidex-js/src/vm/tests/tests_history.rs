//! PR4b C7: `history` host-global tests.

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

/// Construct a VM whose navigation base is `http://localhost/`
/// (not the default `about:blank`).  PR5a C6 switched URL handling
/// to the WHATWG `url` crate, which refuses to resolve a relative
/// URL like `"/a"` against a non-hierarchical base like
/// `about:blank` — matching real browsers, where
/// `history.pushState(null, '', '/a')` on an about:blank page
/// throws.  Tests that exercise history / path plumbing therefore
/// need a concrete hierarchical base; this helper installs it
/// up-front so each test body stays focused on the behaviour
/// under test.
fn new_vm_with_base() -> Vm {
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://localhost/';").unwrap();
    vm
}

#[test]
fn history_is_object() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "typeof history;"), "object");
}

#[test]
fn history_initial_length_is_one() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "history.length;"), 1.0);
}

#[test]
fn history_initial_state_is_null() {
    let mut vm = Vm::new();
    // `history.state` at a fresh load is null.
    match vm.eval("history.state;").unwrap() {
        JsValue::Null => {}
        other => panic!("expected null, got {other:?}"),
    }
}

#[test]
fn history_scroll_restoration_is_auto() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "history.scrollRestoration;"), "auto");
}

#[test]
fn history_push_state_extends_stack() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 1}, '', '/a');").unwrap();
    vm.eval("history.pushState({step: 2}, '', '/b');").unwrap();
    // `history.length` starts at 2 because `new_vm_with_base`
    // pushed the `http://localhost/` entry via the href setter.
    assert_eq!(eval_number(&mut vm, "history.length;"), 4.0);
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/b");
}

#[test]
fn history_replace_state_keeps_length() {
    let mut vm = new_vm_with_base();
    let baseline = eval_number(&mut vm, "history.length;");
    vm.eval("history.replaceState({step: 1}, '', '/a');")
        .unwrap();
    vm.eval("history.replaceState({step: 2}, '', '/b');")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), baseline);
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/b");
}

#[test]
fn history_state_round_trips_through_push_state() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 42}, '', '/x');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 42.0);
}

#[test]
fn history_back_and_forward_move_index() {
    let mut vm = new_vm_with_base();
    vm.eval(
        "history.pushState(null, '', '/a');
         history.pushState(null, '', '/b');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/b");
    vm.eval("history.back();").unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a");
    vm.eval("history.forward();").unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/b");
}

#[test]
fn history_back_at_start_is_noop() {
    let mut vm = Vm::new();
    vm.eval("history.back();").unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 1.0);
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}

#[test]
fn history_go_accepts_delta() {
    let mut vm = new_vm_with_base();
    vm.eval(
        "history.pushState(null, '', '/a');
         history.pushState(null, '', '/b');
         history.pushState(null, '', '/c');",
    )
    .unwrap();
    vm.eval("history.go(-2);").unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a");
    vm.eval("history.go(2);").unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/c");
}

#[test]
fn history_state_survives_gc() {
    // Regression: `NavigationState.history_entries[*].state` is a GC
    // root.  Pushing an object + forcing a GC + reading it back must
    // preserve the value.  Without the root, the pushed object would
    // be collected and `history.state.step` would read as `undefined`.
    let mut vm = new_vm_with_base();
    vm.eval(
        "history.pushState({step: 7, nested: {v: 99}}, '', '/x');
         // Many allocations to raise GC pressure — if the state's
         // nested object were unrooted, GC would have claimed it.
         var filler = [];
         for (var i = 0; i < 5000; i++) { filler.push({k: i}); }
         filler = null;",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 7.0);
    assert_eq!(eval_number(&mut vm, "history.state.nested.v;"), 99.0);
}

#[test]
fn history_push_state_truncates_forward_history() {
    let mut vm = new_vm_with_base();
    let baseline = eval_number(&mut vm, "history.length;");
    vm.eval(
        "history.pushState(null, '', '/a');
         history.pushState(null, '', '/b');
         history.back();
         // Now at /a with forward=/b. A push should drop /b.
         history.pushState(null, '', '/c');",
    )
    .unwrap();
    // baseline + 2 push-after-back ops = baseline + 2.  (Only two
    // net pushes land because the forward-branch push between
    // history.back and pushState('/c') drops the '/b' entry.)
    assert_eq!(eval_number(&mut vm, "history.length;"), baseline + 2.0);
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/c");
}
