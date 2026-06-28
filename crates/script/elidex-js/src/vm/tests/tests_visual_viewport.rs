//! `window.visualViewport` / `VisualViewport` interface tests (CSSOM-View
//! §12.1) — S5-2 minor-window-parity.

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};

use super::super::value::JsValue;
use super::super::Vm;

/// A `Vm` with an (unbound) `HostData` installed so the inherited
/// `EventTarget.prototype.addEventListener` has a `listener_store` to write
/// into (the `MediaQueryList` test precedent).
fn new_vm_with_host() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
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

// --- presence + identity ---------------------------------------------------

#[test]
fn visual_viewport_is_an_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "typeof visualViewport === 'object' && visualViewport !== null"
    ));
}

#[test]
fn visual_viewport_is_same_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "window.visualViewport === window.visualViewport"
    ));
    assert!(eval_bool(
        &mut vm,
        "visualViewport === window.visualViewport"
    ));
}

#[test]
fn visual_viewport_is_visual_viewport_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "visualViewport instanceof VisualViewport"
    ));
    // The EventTarget surface is inherited (the VM exposes no `EventTarget`
    // global constructor, so test the inherited method rather than `instanceof
    // EventTarget`): `addEventListener` resolves up the prototype chain.
    assert!(eval_bool(
        &mut vm,
        "typeof Object.getPrototypeOf(VisualViewport.prototype).addEventListener === 'function'"
    ));
}

#[test]
fn visual_viewport_constructor_is_illegal() {
    // WebIDL: no constructor → `new VisualViewport()` / `VisualViewport()` throw.
    super::assert_illegal_constructor("VisualViewport");
}

// --- geometry --------------------------------------------------------------

#[test]
fn geometry_defaults() {
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "visualViewport.width") - 1024.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.height") - 768.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.offsetLeft")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.offsetTop")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.scale") - 1.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageLeft")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageTop")).abs() < f64::EPSILON);
}

#[test]
fn width_height_track_transported_viewport() {
    let mut vm = Vm::new();
    vm.set_media_environment(
        1280.0,
        720.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    assert!((eval_number(&mut vm, "visualViewport.width") - 1280.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.height") - 720.0).abs() < f64::EPSILON);
}

#[test]
fn page_offset_tracks_scroll() {
    // `pageLeft`/`pageTop` = layout-viewport scroll + visual offset(0).
    let mut vm = Vm::new();
    vm.set_scroll_offset(40.0, 90.0);
    assert!((eval_number(&mut vm, "visualViewport.pageLeft") - 40.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageTop") - 90.0).abs() < f64::EPSILON);
}

#[test]
fn attribute_getter_brand_checks_receiver() {
    // WebIDL attribute getter on an alien receiver → TypeError.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var d = Object.getOwnPropertyDescriptor(VisualViewport.prototype, 'width'); \
         var threw = false; try { d.get.call({}); } catch (e) { threw = e instanceof TypeError; } \
         threw"
    ));
}

// --- EventTarget surface ---------------------------------------------------

#[test]
fn event_handler_attributes_present() {
    let mut vm = Vm::new();
    // `onresize` / `onscroll` / `onscrollend` are accessor IDL attributes,
    // default `null` (no handler set).
    assert!(eval_bool(&mut vm, "visualViewport.onresize === null"));
    assert!(eval_bool(&mut vm, "visualViewport.onscroll === null"));
    assert!(eval_bool(&mut vm, "visualViewport.onscrollend === null"));
}

#[test]
fn add_event_listener_is_real_not_stub() {
    // boa exposed a no-op stub; the VM inherits the real EventTarget method.
    let mut vm = new_vm_with_host();
    assert!(eval_bool(
        &mut vm,
        "typeof visualViewport.addEventListener === 'function' \
         && typeof visualViewport.removeEventListener === 'function'"
    ));
    // Registering / removing a listener must not throw.
    assert!(eval_bool(
        &mut vm,
        "var cb = function () {}; visualViewport.addEventListener('resize', cb); \
         visualViewport.removeEventListener('resize', cb); true"
    ));
}

#[test]
fn onresize_handler_roundtrips() {
    let mut vm = new_vm_with_host();
    assert!(eval_bool(
        &mut vm,
        "var f = function () {}; visualViewport.onresize = f; visualViewport.onresize === f"
    ));
}
