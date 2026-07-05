//! `location` host-global tests — S1c enqueue-only navigation model.
//!
//! Getters read `current_url` (committed by the shell's `set_current_url`); the
//! setters (`href=`/`assign`/`replace`) and `reload()` are *enqueue-only*: they
//! parse + validate the URL synchronously (throwing `SyntaxError` on a bad URL)
//! then record a `NavigationRequest`, leaving `current_url` unchanged.

#![cfg(feature = "engine")]

use elidex_script_session::{NavigationRequest, NavigationType};

use super::super::value::JsValue;
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

/// Commit a base URL via the shell's `set_current_url` path (the enqueue-only
/// setters no longer mutate `current_url`).
fn set_base(vm: &mut Vm, url: &str) {
    vm.inner
        .navigation
        .set_current_url(Some(url::Url::parse(url).unwrap()));
}

/// Drain the navigation request the last setter enqueued.
fn take_nav(vm: &mut Vm) -> NavigationRequest {
    vm.inner
        .navigation
        .pending_navigation
        .take()
        .expect("a navigation request was enqueued")
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
fn location_href_setter_enqueues_and_leaves_url_unchanged() {
    let mut vm = Vm::new();
    set_base(&mut vm, "https://example.com/");
    // The setter enqueues but does NOT commit — a same-script read returns the
    // OLD URL (spec-correct async navigation, matching browsers).
    vm.eval("location.href = 'https://other.com/a?x=1#y';")
        .unwrap();
    assert_eq!(
        eval_string(&mut vm, "location.href;"),
        "https://example.com/"
    );
    let nav = take_nav(&mut vm);
    assert_eq!(nav.url, "https://other.com/a?x=1#y");
    assert_eq!(nav.nav_type, NavigationType::Push);
}

#[test]
fn location_component_getters() {
    let mut vm = Vm::new();
    set_base(&mut vm, "https://example.com:8443/a/b?x=1#y");
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
    set_base(&mut vm, "http://elidex.test/");
    assert_eq!(eval_string(&mut vm, "location.host;"), "elidex.test");
    assert_eq!(eval_string(&mut vm, "location.port;"), "");
    assert_eq!(eval_string(&mut vm, "location.search;"), "");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "");
}

#[test]
fn location_pathname_defaults_to_slash_for_authority_urls() {
    // WHATWG URL §4.4: absolute URLs with an authority but no explicit path have
    // pathname `/`.
    let mut vm = Vm::new();
    set_base(&mut vm, "https://example.com");
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_string(&mut vm, "location.host;"), "example.com");

    set_base(&mut vm, "http://example.com:8080");
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_string(&mut vm, "location.port;"), "8080");

    set_base(&mut vm, "https://example.com?q=1#f");
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
    set_base(&mut vm, "https://example.com/");
    assert_eq!(
        eval_string(&mut vm, "location.toString();"),
        "https://example.com/"
    );
    // `'' + location` routes through the Location wrapper's `toString()`.
    assert_eq!(
        eval_string(&mut vm, "'' + location;"),
        "https://example.com/"
    );
}

#[test]
fn location_assign_enqueues_navigation() {
    let mut vm = Vm::new();
    vm.eval("location.assign('https://a/')").unwrap();
    let nav = take_nav(&mut vm);
    assert_eq!(nav.url, "https://a/");
    assert_eq!(nav.nav_type, NavigationType::Push);
}

#[test]
fn location_replace_enqueues_replace_navigation() {
    let mut vm = Vm::new();
    vm.eval("location.replace('https://a/')").unwrap();
    let nav = take_nav(&mut vm);
    assert_eq!(nav.url, "https://a/");
    assert_eq!(nav.nav_type, NavigationType::Replace);
    // Enqueue-only: `current_url` unchanged.
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}

#[test]
fn location_reload_enqueues_reload_to_current_url() {
    let mut vm = Vm::new();
    set_base(&mut vm, "https://example.com/page");
    vm.eval("location.reload();").unwrap();
    let nav = take_nav(&mut vm);
    assert_eq!(nav.url, "https://example.com/page");
    // `reload()` is `NavigationType::Reload` (a distinct algorithm, §7.4.3), not
    // `Replace` — the two-bool `replace` could not distinguish them.
    assert_eq!(nav.nav_type, NavigationType::Reload);
    assert_eq!(
        eval_string(&mut vm, "location.href;"),
        "https://example.com/page"
    );
}

// ---------------------------------------------------------------------------
// WHATWG `url` crate canonicalisation — asserted on the *enqueued* URL (the
// setter parses + canonicalises before enqueueing).
// ---------------------------------------------------------------------------

#[test]
fn location_href_setter_canonicalises_host_case() {
    // WHATWG URL §4.4: host is lowercased at parse time.
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://HOST.EXAMPLE/a';").unwrap();
    assert_eq!(take_nav(&mut vm).url, "http://host.example/a");
}

#[test]
fn location_href_setter_strips_default_port() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://host:80/a';").unwrap();
    assert_eq!(take_nav(&mut vm).url, "http://host/a");
}

#[test]
fn location_href_setter_resolves_relative_against_base() {
    // `location.href = 'bar'` against `https://site/foo/` resolves to
    // `https://site/foo/bar` via `Url::join` (WHATWG URL §4.4) before enqueue.
    let mut vm = Vm::new();
    set_base(&mut vm, "https://site/foo/");
    vm.eval("location.href = 'bar';").unwrap();
    assert_eq!(take_nav(&mut vm).url, "https://site/foo/bar");
}

#[test]
fn location_href_setter_throws_dom_exception_on_invalid_url() {
    // Unresolvable relative URL on `about:blank` base → SyntaxError DOMException
    // (the setter parses synchronously, before any enqueue).
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
    // Nothing enqueued on the throw path.
    assert!(vm.inner.navigation.pending_navigation.is_none());
}

#[test]
fn location_assign_throws_dom_exception_on_invalid_url() {
    // `assign` shares the synchronous parse + SyntaxError path with `href=`, but
    // is a distinct native fn with its own error message — assert it throws and
    // enqueues nothing (the throw aborts before the enqueue).
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var thrown = null;\
             try { location.assign('\\u0000'); } \
             catch (e) { thrown = e; }\
             thrown && thrown.name === 'SyntaxError' \
             && thrown instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    assert!(vm.inner.navigation.pending_navigation.is_none());
}
