//! PR5d: Window.prototype iframe-related accessor tests
//! (`self` / `parent` / `top` / `frames` / `frameElement` / `opener` /
//! `length` / `closed` / `name`, WHATWG HTML §7.3).
//!
//! All values are single-context stubs today (the VM models one
//! browsing context); these tests pin the surface so that
//! installing `Window.prototype` always resolves these reads
//! deterministically and the same way the legacy boa registration
//! did.  When sub-frame wiring lands (PR6 / Phase 3), the bodies of
//! the accessors will change but the test contract `parent === window`
//! / `frameElement === null` / etc. should still hold for the
//! top-level window.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(src: &str) -> bool {
    let mut vm = Vm::new();
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

#[test]
fn parent_is_window_self_reference() {
    assert!(eval_bool("window.parent === window"));
}

#[test]
fn top_is_window_self_reference() {
    assert!(eval_bool("window.top === window"));
}

#[test]
fn frames_is_window_self_reference() {
    assert!(eval_bool("window.frames === window"));
}

#[test]
fn self_is_window_self_reference() {
    assert!(eval_bool("window.self === window"));
}

#[test]
fn parent_top_frames_self_all_strict_equal() {
    // The four WindowProxy aliases must all coincide for a top-level
    // browsing context — the spec requires them to share identity
    // when there is no parent context.
    assert!(eval_bool(
        "window.parent === window.top &&
         window.top === window.frames &&
         window.frames === window.self &&
         window.self === window"
    ));
}

#[test]
fn frame_element_is_null() {
    assert!(eval_bool("window.frameElement === null"));
}

#[test]
fn opener_is_null() {
    assert!(eval_bool("window.opener === null"));
}

#[test]
fn length_is_zero() {
    assert!(eval_bool("window.length === 0"));
}

#[test]
fn closed_is_false() {
    assert!(eval_bool("window.closed === false"));
}

#[test]
fn iframe_accessors_live_on_window_prototype() {
    // The accessors must live on `Window.prototype`, not as own
    // properties of `globalThis`, so the chain matches WHATWG HTML
    // §7.3.  Verify by reading from `Object.getPrototypeOf(window)`.
    assert!(eval_bool(
        "var p = Object.getPrototypeOf(window);
         var d = Object.getOwnPropertyDescriptor(p, 'parent');
         d !== undefined && typeof d.get === 'function' && d.set === undefined"
    ));
}

#[test]
fn name_default_is_empty_string() {
    let mut vm = Vm::new();
    let v = vm.eval("window.name").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "");
}

#[test]
fn name_setter_stores_string() {
    let mut vm = Vm::new();
    let v = vm.eval("window.name = 'hello'; window.name").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "hello");
}

#[test]
fn name_setter_coerces_via_to_string() {
    // WebIDL DOMString attribute coerces non-string assignments via
    // ToString.  `42` → `"42"`, `null` → `"null"`, `undefined` →
    // `"undefined"`, `true` → `"true"`.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "window.name = 42;
             var a = window.name;
             window.name = null;
             var b = window.name;
             window.name = undefined;
             var c = window.name;
             window.name = true;
             a + ',' + b + ',' + c + ',' + window.name",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "42,null,undefined,true");
}

#[test]
fn name_setter_invokes_user_to_string() {
    // §7.1.12 step 9 → §7.1.1.1: a non-wrapper Object passed through
    // `ToString` runs `OrdinaryToPrimitive(hint='string')`, which calls
    // user-defined `toString()` first and returns the produced primitive.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "window.name = { toString() { return 'from-object'; } };
             window.name",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "from-object");
}

#[test]
fn name_persists_across_evals() {
    let mut vm = Vm::new();
    vm.eval("window.name = 'persistent'").unwrap();
    let v = vm.eval("window.name").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "persistent");
}

#[test]
fn name_accessor_lives_on_window_prototype() {
    assert!(eval_bool(
        "var p = Object.getPrototypeOf(window);
         var d = Object.getOwnPropertyDescriptor(p, 'name');
         d !== undefined &&
         typeof d.get === 'function' &&
         typeof d.set === 'function'"
    ));
}
