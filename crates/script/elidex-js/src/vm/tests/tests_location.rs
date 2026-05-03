//! PR4b C6: `location` host-global tests.
//!
//! Covers initial state (`about:blank`), href round-trip via setter,
//! component getters (protocol/host/…/origin), assign / replace /
//! toString / reload stubs, and the history-entry bookkeeping that
//! `assign` vs `replace` performs.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn location_is_object() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "typeof location;"), "object");
}

#[test]
fn location_href_defaults_to_about_blank() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}

#[test]
fn location_href_setter_updates_current_url() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "location.href = 'https://example.com/a?x=1#y'; location.href;"
        ),
        "https://example.com/a?x=1#y"
    );
}

#[test]
fn location_component_getters() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://example.com:8443/a/b?x=1#y';")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.protocol;"), "https:");
    assert_eq!(eval_string(&mut vm, "location.host;"), "example.com:8443");
    assert_eq!(eval_string(&mut vm, "location.hostname;"), "example.com");
    assert_eq!(eval_string(&mut vm, "location.port;"), "8443");
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a/b");
    assert_eq!(eval_string(&mut vm, "location.search;"), "?x=1");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "#y");
    assert_eq!(
        eval_string(&mut vm, "location.origin;"),
        "https://example.com:8443"
    );
}

#[test]
fn location_component_getters_no_port_no_query() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://elidex.test/';").unwrap();
    assert_eq!(eval_string(&mut vm, "location.host;"), "elidex.test");
    assert_eq!(eval_string(&mut vm, "location.port;"), "");
    assert_eq!(eval_string(&mut vm, "location.search;"), "");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "");
}

#[test]
fn location_pathname_defaults_to_slash_for_authority_urls() {
    // WHATWG URL §4.4: absolute URLs with an authority but no
    // explicit path have pathname `/`, not the empty string.
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://example.com';").unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_string(&mut vm, "location.host;"), "example.com");

    // Same with port.
    vm.eval("location.href = 'http://example.com:8080';")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_string(&mut vm, "location.port;"), "8080");

    // Query + fragment after bare host still normalise pathname to "/".
    vm.eval("location.href = 'https://example.com?q=1#f';")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_string(&mut vm, "location.search;"), "?q=1");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "#f");
}

#[test]
fn location_origin_is_null_for_opaque_schemes() {
    let mut vm = Vm::new();
    // `about:blank` — no authority, opaque origin → "null".
    assert_eq!(eval_string(&mut vm, "location.origin;"), "null");
}

#[test]
fn location_to_string_matches_href() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://example.com/';").unwrap();
    assert_eq!(
        eval_string(&mut vm, "location.toString();"),
        "https://example.com/"
    );
    // §7.1.12 step 9 → §7.1.1.1 routes `'' + location` through the
    // Location wrapper's `toString()`, so the concatenation now produces
    // the href just like the explicit `.toString()` call above.
    assert_eq!(
        eval_string(&mut vm, "'' + location;"),
        "https://example.com/"
    );
}

#[test]
fn location_assign_pushes_new_history_entry() {
    let mut vm = Vm::new();
    // Start: about:blank (1 entry). `assign` adds one more each call.
    // `history` global lands in C7; read via the internal nav state.
    vm.eval("location.assign('https://a/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 2);
    vm.eval("location.assign('https://b/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 3);
    assert_eq!(vm.inner.navigation.history_index, 2);
}

#[test]
fn location_replace_overwrites_current_entry() {
    let mut vm = Vm::new();
    // `replace` overwrites in place — history length stays 1.
    vm.eval("location.replace('https://a/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 1);
    vm.eval("location.replace('https://b/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 1);
    assert_eq!(eval_string(&mut vm, "location.href;"), "https://b/");
}

#[test]
fn location_reload_is_no_op_but_callable() {
    let mut vm = Vm::new();
    vm.eval("location.reload();").unwrap();
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}

// ---------------------------------------------------------------------------
// WHATWG `url` crate canonicalisation
// ---------------------------------------------------------------------------

#[test]
fn location_href_canonicalises_host_case() {
    // WHATWG URL §4.4: host is lowercased at parse time.
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://HOST.EXAMPLE/a';").unwrap();
    assert_eq!(eval_string(&mut vm, "location.host;"), "host.example");
}

#[test]
fn location_href_strips_default_port() {
    // Default port for the scheme is stripped.
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://host:80/a';").unwrap();
    assert_eq!(eval_string(&mut vm, "location.host;"), "host");
    assert_eq!(eval_string(&mut vm, "location.port;"), "");
    assert_eq!(eval_string(&mut vm, "location.href;"), "http://host/a");
}

#[test]
fn location_href_setter_resolves_relative_against_base() {
    // `location.href = 'bar'` against `https://site/foo/` lands at
    // `https://site/foo/bar` via `Url::join` (WHATWG URL §4.5).
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://site/foo/';").unwrap();
    vm.eval("location.href = 'bar';").unwrap();
    assert_eq!(
        eval_string(&mut vm, "location.href;"),
        "https://site/foo/bar"
    );
}

#[test]
fn location_href_setter_throws_dom_exception_on_invalid_url() {
    // Unresolvable relative URL on `about:blank` base → SyntaxError
    // DOMException throw path.
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var thrown = null;\
             try { location.href = '\\u0000'; } \
             catch (e) { thrown = e; }\
             thrown && thrown.name === 'SyntaxError' \
             && thrown instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
}
