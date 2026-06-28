//! `window.screen` / `Screen` interface tests (CSSOM-View §4.3) — S5-2
//! minor-window-parity.

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};

use super::super::value::JsValue;
use super::super::Vm;

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
fn screen_is_an_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "typeof screen === 'object' && screen !== null"
    ));
}

#[test]
fn screen_window_is_same_object() {
    // `[SameObject]` (CSSOM-View §4): `window.screen === window.screen`.
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "window.screen === window.screen"));
    assert!(eval_bool(&mut vm, "screen === window.screen"));
}

#[test]
fn screen_dimensions_default_viewport() {
    // Default viewport = 1024×768 (presence-first: screen reports the viewport).
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "screen.width") - 1024.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 768.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availWidth") - 1024.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availHeight") - 768.0).abs() < f64::EPSILON);
}

#[test]
fn screen_color_depth_is_24() {
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "screen.colorDepth") - 24.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.pixelDepth") - 24.0).abs() < f64::EPSILON);
}

#[test]
fn screen_dimensions_track_transported_viewport() {
    // The presence-first source is the live `ViewportState`, so a
    // `set_media_environment` push moves `screen.width` / `.height`.
    let mut vm = Vm::new();
    vm.set_media_environment(
        1440.0,
        900.0,
        2.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    assert!((eval_number(&mut vm, "screen.width") - 1440.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 900.0).abs() < f64::EPSILON);
}

#[test]
fn screen_width_is_integer_long() {
    // WebIDL `long`: a fractional transported width is truncated.
    let mut vm = Vm::new();
    vm.set_media_environment(
        800.6,
        600.4,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    assert!((eval_number(&mut vm, "screen.width") - 800.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 600.0).abs() < f64::EPSILON);
}
