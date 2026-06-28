//! `cookieStore` / `CookieStore` interface tests (Cookie Store API §3 / §6.1)
//! — S5-2 minor-window-parity.
//!
//! `get` / `getAll` / `set` / `delete` are `Promise`-returning; the methods
//! resolve synchronously (the jar op is sync), so a `.then` attached in the same
//! `eval` runs during the post-script microtask drain. Resolved values are
//! captured into top-level `var`s and read back via `vm.get_global` (the
//! `eval_global_*` async-observation pattern).

#![cfg(feature = "engine")]

use std::sync::Arc;

use elidex_ecs::EcsDom;
use elidex_net::CookieJar;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn global_number(vm: &Vm, name: &str) -> f64 {
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} number, got {other:?}"),
    }
}

fn global_string(vm: &Vm, name: &str) -> String {
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} string, got {other:?}"),
    }
}

/// Run `body` against a VM bound to an `https://example.com/` document with a
/// fresh `CookieJar` installed (so `cookieStore` round-trips are observable).
fn with_cookie_vm(body: impl FnOnce(&mut Vm)) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url::Url::parse("https://example.com/").expect("valid URL");
    let jar = Arc::new(CookieJar::new());
    vm.host_data()
        .expect("host_data installed by bind_vm")
        .install_cookie_jar(jar);
    body(&mut vm);
    vm.unbind();
}

// --- presence + identity ---------------------------------------------------

#[test]
fn cookie_store_is_an_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "typeof cookieStore === 'object' && cookieStore !== null"
    ));
}

#[test]
fn cookie_store_is_same_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "window.cookieStore === window.cookieStore"
    ));
}

#[test]
fn cookie_store_is_cookie_store_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "cookieStore instanceof CookieStore"));
    // EventTarget surface inherited (no `EventTarget` global constructor in the
    // VM, so test the inherited method rather than `instanceof EventTarget`).
    assert!(eval_bool(
        &mut vm,
        "typeof Object.getPrototypeOf(CookieStore.prototype).addEventListener === 'function'"
    ));
}

#[test]
fn cookie_store_constructor_is_illegal() {
    super::assert_illegal_constructor("CookieStore");
}

#[test]
fn methods_present() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "['get','getAll','set','delete'].every(m => typeof cookieStore[m] === 'function')"
    ));
}

#[test]
fn method_brand_checks_receiver() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; try { cookieStore.get.call({}, 'x'); } \
         catch (e) { threw = e instanceof TypeError; } threw"
    ));
}

#[test]
fn get_returns_a_promise() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "cookieStore.get('x') instanceof Promise"
    ));
    assert!(eval_bool(
        &mut vm,
        "cookieStore.getAll() instanceof Promise"
    ));
    assert!(eval_bool(
        &mut vm,
        "cookieStore.set('a', '1') instanceof Promise"
    ));
    assert!(eval_bool(
        &mut vm,
        "cookieStore.delete('a') instanceof Promise"
    ));
}

// --- read / write round-trips ----------------------------------------------

// Resolved values are exported via `globalThis.*` (the `eval_global_*` /
// `tests_blob` async-observation precedent — a top-level `var` does not land in
// the `globals` map a strict-mode VM reads via `get_global`). Names avoid the
// real `name` / `value` window globals.

#[test]
fn get_unknown_resolves_null() {
    with_cookie_vm(|vm| {
        vm.eval("globalThis.got = 'unset'; cookieStore.get('missing').then(v => { globalThis.got = v; });")
            .unwrap();
        assert!(matches!(vm.get_global("got"), Some(JsValue::Null)));
    });
}

#[test]
fn get_all_empty_resolves_empty_array() {
    with_cookie_vm(|vm| {
        vm.eval("globalThis.n = -1; cookieStore.getAll().then(l => { globalThis.n = l.length; });")
            .unwrap();
        assert!((global_number(vm, "n")).abs() < f64::EPSILON);
    });
}

#[test]
fn set_then_get_all_round_trips() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.n = -1; globalThis.cname = ''; globalThis.cval = ''; \
             cookieStore.set('session', 'abc'); \
             cookieStore.getAll().then(l => { \
               globalThis.n = l.length; \
               if (l[0]) { globalThis.cname = l[0].name; globalThis.cval = l[0].value; } });",
        )
        .unwrap();
        assert!((global_number(vm, "n") - 1.0).abs() < f64::EPSILON);
        assert_eq!(global_string(vm, "cname"), "session");
        assert_eq!(global_string(vm, "cval"), "abc");
    });
}

#[test]
fn set_then_get_by_name() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.cval = 'unset'; \
             cookieStore.set('token', 'xyz'); \
             cookieStore.get('token').then(c => { globalThis.cval = c ? c.value : null; });",
        )
        .unwrap();
        assert_eq!(global_string(vm, "cval"), "xyz");
    });
}

#[test]
fn set_via_options_object() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.cval = 'unset'; \
             cookieStore.set({ name: 'opt', value: 'dict' }); \
             cookieStore.get('opt').then(c => { globalThis.cval = c ? c.value : null; });",
        )
        .unwrap();
        assert_eq!(global_string(vm, "cval"), "dict");
    });
}

#[test]
fn delete_removes_cookie() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.after = -1; \
             cookieStore.set('temp', '1'); \
             cookieStore.delete('temp'); \
             cookieStore.getAll().then(l => { globalThis.after = l.length; });",
        )
        .unwrap();
        assert!((global_number(vm, "after")).abs() < f64::EPSILON);
    });
}

#[test]
fn get_all_filters_by_name() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.n = -1; \
             cookieStore.set('a', '1'); cookieStore.set('b', '2'); \
             cookieStore.getAll('a').then(l => { globalThis.n = l.length; });",
        )
        .unwrap();
        assert!((global_number(vm, "n") - 1.0).abs() < f64::EPSILON);
    });
}

#[test]
fn cookie_list_item_has_expected_fields() {
    with_cookie_vm(|vm| {
        vm.eval(
            "globalThis.keys = ''; \
             cookieStore.set('k', 'v'); \
             cookieStore.get('k').then(c => { globalThis.keys = Object.keys(c).sort().join(','); });",
        )
        .unwrap();
        // boa-parity field superset (create-a-CookieListItem note).
        assert_eq!(
            global_string(vm, "keys"),
            "domain,expires,name,path,sameSite,secure,value"
        );
    });
}

// --- cookie-averse fallback ------------------------------------------------

#[test]
fn get_without_jar_resolves_null() {
    // No HostData / jar (cookie-averse): get resolves null, no throw.
    let mut vm = Vm::new();
    vm.eval("globalThis.got = 'unset'; cookieStore.get('x').then(v => { globalThis.got = v; });")
        .unwrap();
    assert!(matches!(vm.get_global("got"), Some(JsValue::Null)));
}

#[test]
fn onchange_handler_attribute_present() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "cookieStore.onchange === null"));
}
