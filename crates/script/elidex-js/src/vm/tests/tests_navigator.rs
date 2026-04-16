//! PR4b C4: `navigator` host-global tests.
//!
//! Phase 2 scope — static UA/language/etc. probes that feature
//! detection patterns rely on.  Verifies both the presence of the
//! global and the shape of its fields.

#![cfg(feature = "engine")]

use super::super::Vm;

fn eval_string(source: &str) -> String {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        super::super::value::JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_bool(source: &str) -> bool {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        super::super::value::JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(source: &str) -> f64 {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        super::super::value::JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn navigator_is_object() {
    assert_eq!(eval_string("typeof navigator;"), "object");
}

#[test]
fn navigator_user_agent_is_present() {
    // Identifies elidex but keeps the `Mozilla/5.0 (compatible; ...)`
    // convention so sniffing libraries do not short-circuit.
    assert!(eval_bool("navigator.userAgent.indexOf('Elidex') >= 0;"));
    assert!(eval_bool(
        "navigator.userAgent.indexOf('Mozilla/5.0') === 0;"
    ));
}

#[test]
fn navigator_language_defaults_to_en_us() {
    assert_eq!(eval_string("navigator.language;"), "en-US");
}

#[test]
fn navigator_languages_is_array() {
    assert!(eval_bool("Array.isArray(navigator.languages);"));
    assert_eq!(eval_number("navigator.languages.length;"), 2.0);
    assert_eq!(eval_string("navigator.languages[0];"), "en-US");
    assert_eq!(eval_string("navigator.languages[1];"), "en");
}

#[test]
fn navigator_online_is_true() {
    assert!(eval_bool("navigator.onLine === true;"));
}

#[test]
fn navigator_cookie_enabled_is_false_until_cookies_ship() {
    // Honest reporting — we have no cookie jar yet.
    assert!(eval_bool("navigator.cookieEnabled === false;"));
}

#[test]
fn navigator_java_enabled_is_false() {
    assert!(eval_bool("navigator.javaEnabled === false;"));
}

#[test]
fn navigator_hardware_concurrency_is_positive_integer() {
    let v = eval_number("navigator.hardwareConcurrency;");
    assert!(v >= 1.0 && v.fract() == 0.0, "hardwareConcurrency={v}");
}

#[test]
fn navigator_app_name_is_netscape_per_spec() {
    // WHATWG HTML §8.1.5 fixes `appName` to "Netscape" regardless of UA.
    assert_eq!(eval_string("navigator.appName;"), "Netscape");
}

#[test]
fn navigator_vendor_is_empty() {
    assert_eq!(eval_string("navigator.vendor;"), "");
}

#[test]
fn navigator_platform_is_non_empty() {
    // We default to `std::env::consts::OS`; on every supported target
    // this is non-empty.
    assert!(eval_bool("typeof navigator.platform === 'string';"));
    assert!(eval_bool("navigator.platform.length > 0;"));
}
