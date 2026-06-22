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
fn navigator_cookie_enabled_false_without_jar() {
    // `cookieEnabled` is value-derived (A3): a getter reading the bound
    // `CookieJar` (WHATWG HTML §8.10.1.5). An unbound `Vm::new()` has no
    // jar, so it reports `false`. The `true`-with-jar case is covered in
    // `tests_cookie_referrer` (it owns the bind+jar scaffolding).
    assert!(eval_bool("navigator.cookieEnabled === false;"));
}

#[test]
fn navigator_java_enabled_is_a_method_returning_false() {
    // WHATWG HTML §8.10.1.6 (NavigatorPlugins): `javaEnabled()` is a **method**
    // whose steps "are to return false" — NOT a bool data property (the former
    // shape made `navigator.javaEnabled()` throw a TypeError).
    assert!(eval_bool("typeof navigator.javaEnabled === 'function';"));
    assert!(eval_bool("navigator.javaEnabled() === false;"));
}

#[test]
fn navigator_app_code_name_is_mozilla_per_spec() {
    // WHATWG HTML §8.10.1.1 (NavigatorID) fixes `appCodeName` to "Mozilla".
    assert_eq!(eval_string("navigator.appCodeName;"), "Mozilla");
}

#[test]
fn navigator_plugins_is_empty_collection() {
    // WHATWG HTML §8.10.1.6: elidex's *PDF viewer supported* is `false`, so the
    // `plugins` PluginArray is the empty list — but the interface shape is
    // present (length + item/namedItem/refresh).
    assert!(eval_bool("typeof navigator.plugins === 'object';"));
    assert_eq!(eval_number("navigator.plugins.length;"), 0.0);
    assert!(eval_bool("navigator.plugins.item(0) === null;"));
    assert!(eval_bool("navigator.plugins.namedItem('x') === null;"));
    assert!(eval_bool(
        "typeof navigator.plugins.refresh === 'function';"
    ));
    assert!(eval_bool("navigator.plugins.refresh() === undefined;"));
    // `[SameObject]`: repeated access yields the same object.
    assert!(eval_bool("navigator.plugins === navigator.plugins;"));
}

#[test]
fn navigator_mime_types_is_empty_collection() {
    // WHATWG HTML §8.10.1.6: empty MimeTypeArray (no `refresh()` — that is
    // PluginArray-only).
    assert!(eval_bool("typeof navigator.mimeTypes === 'object';"));
    assert_eq!(eval_number("navigator.mimeTypes.length;"), 0.0);
    assert!(eval_bool("navigator.mimeTypes.item(0) === null;"));
    assert!(eval_bool("navigator.mimeTypes.namedItem('x') === null;"));
    assert!(eval_bool("navigator.mimeTypes.refresh === undefined;"));
    assert!(eval_bool("navigator.mimeTypes === navigator.mimeTypes;"));
}

#[test]
fn navigator_collection_item_converts_present_arg_before_null() {
    // WHATWG HTML §8.10.1.6 `item(unsigned long)` / `namedItem(DOMString)` run
    // the WebIDL argument conversion before the (empty-list) lookup, so a present
    // argument with a throwing conversion propagates — matching the sibling
    // `collection_item_impl` in host/dom_collection.rs. A *missing* argument
    // returns `null` without throwing (the VM's lenient-arity collection idiom).
    assert!(eval_bool(
        "(() => { try { navigator.plugins.item({ valueOf() { throw 'x'; } }); return false; } \
         catch (e) { return true; } })();"
    ));
    assert!(eval_bool(
        "(() => { try { navigator.mimeTypes.namedItem({ toString() { throw 'x'; } }); return false; } \
         catch (e) { return true; } })();"
    ));
    // Missing argument: no throw, returns null.
    assert!(eval_bool("navigator.plugins.item() === null;"));
    assert!(eval_bool("navigator.mimeTypes.namedItem() === null;"));
    // Non-throwing present arg still returns null (empty collection).
    assert!(eval_bool("navigator.plugins.item(3) === null;"));
    assert!(eval_bool("navigator.mimeTypes.namedItem('Foo') === null;"));
}

#[test]
fn navigator_pdf_viewer_enabled_is_false() {
    // WHATWG HTML §8.10.1.6: `pdfViewerEnabled` returns the UA's *PDF viewer
    // supported* boolean, which is `false` for elidex.
    assert!(eval_bool("navigator.pdfViewerEnabled === false;"));
}

#[test]
fn navigator_hardware_concurrency_is_positive_integer() {
    let v = eval_number("navigator.hardwareConcurrency;");
    assert!(v >= 1.0 && v.fract() == 0.0, "hardwareConcurrency={v}");
}

#[test]
fn navigator_app_name_is_netscape_per_spec() {
    // WHATWG HTML §8.10.1.1 fixes `appName` to "Netscape" regardless of UA.
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
