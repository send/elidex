//! `window.screen` / `Screen` interface tests (CSSOM-View §4.3) — S5-2
//! minor-window-parity.
//!
//! Screen reports **monitor** dimensions (a dedicated `ViewportState` device
//! fact pushed by `set_screen_dimensions`), DISTINCT from the layout viewport
//! (`window.innerWidth`). `screen` is a dedicated `ObjectKind::Screen` brand
//! (T4: `structuredClone(screen)` throws DataCloneError) installed as a
//! no-setter RO accessor returning a cached `[SameObject]` singleton (T3:
//! `screen = null` leaves the singleton intact).

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

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
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
fn screen_is_screen_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "screen instanceof Screen"));
}

#[test]
fn screen_constructor_is_illegal() {
    // WebIDL: no constructor → `new Screen()` / `Screen()` throw.
    super::assert_illegal_constructor("Screen");
}

#[test]
fn screen_window_is_same_object() {
    // `[SameObject]` (CSSOM-View §4): `window.screen === window.screen`.
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "window.screen === window.screen"));
    assert!(eval_bool(&mut vm, "screen === window.screen"));
}

#[test]
fn screen_dimensions_default_monitor() {
    // Default monitor = 1920×1080 (a realistic desktop default, DISTINCT from
    // the 1024×768 viewport default).
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "screen.width") - 1920.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 1080.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availWidth") - 1920.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availHeight") - 1080.0).abs() < f64::EPSILON);
}

#[test]
fn screen_differs_from_inner_width_on_nonsquare_viewport() {
    // T1: `screen.width` is the MONITOR size, NOT the viewport (`innerWidth`).
    // Default monitor 1920 vs default viewport 1024 — distinct out of the box.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "screen.width !== window.innerWidth && screen.height !== window.innerHeight"
    ));
}

#[test]
fn screen_color_depth_is_24() {
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "screen.colorDepth") - 24.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.pixelDepth") - 24.0).abs() < f64::EPSILON);
}

#[test]
fn set_screen_dimensions_moves_screen_attrs() {
    // T1 + M1: the dedicated `set_screen_dimensions` endpoint drives
    // `screen.width` / `.height` / `.availWidth` / `.availHeight`.
    let mut vm = Vm::new();
    vm.set_screen_dimensions(2560.0, 1440.0, 2560.0, 1400.0);
    assert!((eval_number(&mut vm, "screen.width") - 2560.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 1440.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availWidth") - 2560.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.availHeight") - 1400.0).abs() < f64::EPSILON);
}

#[test]
fn screen_is_not_aliased_to_media_environment() {
    // M1 coupling: monitor dims are NOT a `MediaEnvironment` input, so a
    // `set_media_environment` (viewport) push must NOT move `screen.*`.
    let mut vm = Vm::new();
    vm.set_screen_dimensions(1920.0, 1080.0, 1920.0, 1080.0);
    vm.set_media_environment(
        800.0,
        600.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    // `screen.*` stays at the monitor dims; `innerWidth` tracks the viewport.
    assert!((eval_number(&mut vm, "screen.width") - 1920.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "window.innerWidth") - 800.0).abs() < f64::EPSILON);
}

#[test]
fn screen_width_is_integer_long() {
    // WebIDL `long`: a fractional transported width is truncated.
    let mut vm = Vm::new();
    vm.set_screen_dimensions(1920.6, 1080.4, 1920.6, 1080.4);
    assert!((eval_number(&mut vm, "screen.width") - 1920.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "screen.height") - 1080.0).abs() < f64::EPSILON);
}

#[test]
fn screen_assignment_leaves_singleton_intact() {
    // T3: `window.screen` is a no-setter RO accessor (the `[Replaceable]`
    // Window-attr family form). elidex-js core is strict-mode-only, so assigning
    // `screen = null` hits the inherited-no-setter branch and THROWS a TypeError
    // (rather than the sloppy-mode silent no-op) — and crucially must NOT replace
    // the cached singleton: a subsequent read still returns the same `Screen`.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var before = window.screen; var threw = false; \
         try { screen = null; } catch (e) { threw = e instanceof TypeError; } \
         threw && window.screen === before && (window.screen instanceof Screen)"
    ));
}

#[test]
fn screen_brand_check_on_alien_receiver() {
    // WebIDL attribute getter on an alien receiver → TypeError.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var d = Object.getOwnPropertyDescriptor(Screen.prototype, 'width'); \
         var threw = false; try { d.get.call({}); } catch (e) { threw = e instanceof TypeError; } \
         threw"
    ));
}

#[test]
fn structured_clone_screen_throws_data_clone_error() {
    // T4: `Screen` is not [Serializable] — `structuredClone(screen)` throws
    // DataCloneError (it does NOT silently clone the accessor-only object to {}).
    let mut vm = Vm::new();
    let name = eval_string(
        &mut vm,
        "var caught = null; \
         try { structuredClone(screen); } catch (e) { caught = e.name; } caught;",
    );
    assert_eq!(name, "DataCloneError");
}
