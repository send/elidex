//! `URL` tests (WHATWG URL §6.1).
//!
//! Covers the constructor (absolute / relative + base), every IDL
//! accessor (read + write paths), the `searchParams` ↔ URL
//! bidirectional linkage, the `URL.canParse` / `URL.parse`
//! statics, brand checks, and a GC pruning round-trip.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

#[test]
fn ctor_absolute_url() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://example.com/path').href;"),
        "https://example.com/path"
    );
}

#[test]
fn ctor_relative_url_with_base() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URL('foo/bar', 'https://example.com/dir/').href;"
        ),
        "https://example.com/dir/foo/bar"
    );
}

#[test]
fn ctor_invalid_url_throws_type_error() {
    let mut vm = Vm::new();
    assert!(vm.eval("new URL('not a url');").is_err());
}

#[test]
fn ctor_invalid_base_throws_type_error() {
    let mut vm = Vm::new();
    assert!(vm.eval("new URL('foo', 'not a base');").is_err());
}

#[test]
fn ctor_requires_new() {
    let mut vm = Vm::new();
    assert!(vm.eval("URL('https://example.com');").is_err());
}

#[test]
fn ctor_coerces_arguments_via_to_string() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URL({toString() { return 'https://example.com/p'; }}).href;"
        ),
        "https://example.com/p"
    );
}

// ---------------------------------------------------------------------------
// Accessor getters
// ---------------------------------------------------------------------------

#[test]
fn href_getter_returns_serialization() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URL('https://u:p@example.com:8080/path?x=1#frag').href;"
        ),
        "https://u:p@example.com:8080/path?x=1#frag"
    );
}

#[test]
fn origin_getter_strips_userinfo_and_path() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URL('https://u:p@example.com:8080/p?x=1').origin;"
        ),
        "https://example.com:8080"
    );
}

#[test]
fn protocol_getter_includes_trailing_colon() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://example.com').protocol;"),
        "https:"
    );
}

#[test]
fn host_getter_includes_port_when_explicit() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://example.com:8080/').host;"),
        "example.com:8080"
    );
}

#[test]
fn hostname_getter_omits_port() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://example.com:8080/').hostname;"),
        "example.com"
    );
}

#[test]
fn port_getter_empty_when_default() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://example.com:443/').port;"),
        ""
    );
}

#[test]
fn pathname_getter_returns_path() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://x.com/a/b').pathname;"),
        "/a/b"
    );
}

#[test]
fn search_getter_includes_question_mark() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://x.com/?a=1&b=2').search;"),
        "?a=1&b=2"
    );
}

#[test]
fn search_getter_empty_when_no_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://x.com/').search;"),
        ""
    );
}

#[test]
fn hash_getter_includes_pound_sign() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://x.com/#frag').hash;"),
        "#frag"
    );
}

#[test]
fn username_password_getters() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new URL('https://u:p@x.com/').username;"),
        "u"
    );
    assert_eq!(
        eval_string(&mut vm, "new URL('https://u:p@x.com/').password;"),
        "p"
    );
}

// ---------------------------------------------------------------------------
// Accessor setters
// ---------------------------------------------------------------------------

#[test]
fn href_setter_full_reparse() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); \
             u.href = 'https://y.com/path?q=v'; \
             u.href;"
        ),
        "https://y.com/path?q=v"
    );
}

#[test]
fn href_setter_invalid_throws_type_error() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("let u = new URL('https://x.com/'); u.href = 'not a url';")
        .is_err());
}

#[test]
fn protocol_setter_strips_trailing_colon() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); u.protocol = 'http:'; u.protocol;"
        ),
        "http:"
    );
}

#[test]
fn host_setter_replaces_authority() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/p'); \
             u.host = 'y.com:9000'; \
             u.host;"
        ),
        "y.com:9000"
    );
}

#[test]
fn hostname_setter_keeps_port() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com:8080/'); \
             u.hostname = 'y.com'; \
             u.host;"
        ),
        "y.com:8080"
    );
}

#[test]
fn port_setter_clears_when_empty() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com:8080/'); u.port = ''; u.host;"
        ),
        "x.com"
    );
}

#[test]
fn port_setter_silently_ignores_garbage() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com:8080/'); u.port = 'not'; u.port;"
        ),
        "8080"
    );
}

#[test]
fn pathname_setter_replaces_path() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/a'); u.pathname = '/b/c'; u.pathname;"
        ),
        "/b/c"
    );
}

#[test]
fn search_setter_strips_leading_question_mark() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); u.search = '?a=1'; u.search;"
        ),
        "?a=1"
    );
}

#[test]
fn search_setter_accepts_no_question_mark() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); u.search = 'a=1'; u.search;"
        ),
        "?a=1"
    );
}

#[test]
fn hash_setter_strips_leading_pound() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); u.hash = '#top'; u.hash;"
        ),
        "#top"
    );
}

#[test]
fn password_setter_clear_with_empty_string() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://u:p@x.com/'); u.password = ''; u.password;"
        ),
        ""
    );
}

#[test]
fn username_setter_replaces() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://u:p@x.com/'); u.username = 'admin'; u.username;"
        ),
        "admin"
    );
}

// ---------------------------------------------------------------------------
// origin / searchParams (read-only)
// ---------------------------------------------------------------------------

#[test]
fn origin_is_readonly_silently_ignored() {
    let mut vm = Vm::new();
    // WebIDL accessor without setter swallows assignment in
    // sloppy mode (matches V8 / Firefox).  Read after write must
    // still return the original origin.
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); \
             try { u.origin = 'https://hijack/'; } catch(e) {} \
             u.origin;"
        ),
        "https://x.com"
    );
}

#[test]
fn search_params_identity_is_stable() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let u = new URL('https://x.com/?a=1'); u.searchParams === u.searchParams;"
    ));
}

#[test]
fn search_params_initial_entries_match_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new URL('https://x.com/?a=1&b=2').searchParams.toString();"
        ),
        "a=1&b=2"
    );
}

// ---------------------------------------------------------------------------
// searchParams ↔ URL bidirectional linkage
// ---------------------------------------------------------------------------

#[test]
fn search_params_append_updates_url_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/'); \
             u.searchParams.append('a', '1'); \
             u.searchParams.append('b', '2'); \
             u.search;"
        ),
        "?a=1&b=2"
    );
}

#[test]
fn search_params_delete_updates_url_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/?a=1&b=2&a=3'); \
             u.searchParams.delete('a'); \
             u.search;"
        ),
        "?b=2"
    );
}

#[test]
fn search_params_set_updates_url_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/?a=1'); \
             u.searchParams.set('a', '99'); \
             u.search;"
        ),
        "?a=99"
    );
}

#[test]
fn search_params_sort_updates_url_query() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/?b=2&a=1'); \
             u.searchParams.sort(); \
             u.search;"
        ),
        "?a=1&b=2"
    );
}

#[test]
fn url_search_setter_rebuilds_search_params() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/?old=1'); \
             u.search = 'new=2&extra=3'; \
             u.searchParams.get('new');"
        ),
        "2"
    );
}

#[test]
fn url_href_setter_rebuilds_search_params() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/?old=1'); \
             u.href = 'https://y.com/?fresh=42'; \
             u.searchParams.get('fresh');"
        ),
        "42"
    );
}

#[test]
fn standalone_search_params_does_not_update_url() {
    // A `URLSearchParams` not allocated through `new URL` must
    // not write into any `url_states` entry — defends the
    // back-edge contract.
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let p = new URLSearchParams('a=1'); p.append('b', '2'); p.toString();"
        ),
        "a=1&b=2"
    );
}

// ---------------------------------------------------------------------------
// toString / toJSON
// ---------------------------------------------------------------------------

#[test]
fn to_string_returns_href() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/p?a=1#h'); u.toString();"
        ),
        "https://x.com/p?a=1#h"
    );
}

#[test]
fn to_json_returns_href() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = new URL('https://x.com/p?a=1#h'); u.toJSON();"
        ),
        "https://x.com/p?a=1#h"
    );
}

#[test]
fn to_string_and_to_json_share_the_same_function_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "URL.prototype.toString === URL.prototype.toJSON;"
    ));
}

// ---------------------------------------------------------------------------
// Statics
// ---------------------------------------------------------------------------

#[test]
fn can_parse_returns_true_for_valid_url() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "URL.canParse('https://x.com/');"));
}

#[test]
fn can_parse_returns_false_for_invalid_url() {
    let mut vm = Vm::new();
    assert!(!eval_bool(&mut vm, "URL.canParse('not a url');"));
}

#[test]
fn can_parse_supports_base_argument() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "URL.canParse('foo', 'https://x.com/');"));
    assert!(!eval_bool(&mut vm, "URL.canParse('foo');"));
}

#[test]
fn parse_returns_null_for_invalid_url() {
    let mut vm = Vm::new();
    let v = vm.eval("URL.parse('not a url');").unwrap();
    assert!(matches!(v, JsValue::Null), "expected null, got {v:?}");
}

#[test]
fn parse_returns_url_instance_for_valid_url() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "URL.parse('https://x.com/p').href;"),
        "https://x.com/p"
    );
}

#[test]
fn parse_creates_searchparams_link() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let u = URL.parse('https://x.com/?a=1'); \
             u.searchParams.append('b', '2'); \
             u.search;"
        ),
        "?a=1&b=2"
    );
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

#[test]
fn brand_check_throws_on_alien_receiver() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("Object.getOwnPropertyDescriptor(URL.prototype, 'href').get.call({});")
        .is_err());
}

// ---------------------------------------------------------------------------
// GC
// ---------------------------------------------------------------------------

#[test]
fn prototype_survives_gc_after_global_removal() {
    // Mirror the `URLSearchParams` regression: even after
    // `delete globalThis.URL`, `VmInner::url_prototype` keeps the
    // intrinsic alive across a forced GC, so a stashed binding
    // can still construct.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.SavedURL = URL; \
         delete globalThis.URL;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        eval_string(&mut vm, "new SavedURL('https://x.com/p').href;"),
        "https://x.com/p"
    );
}

#[test]
fn gc_drops_url_state_when_instance_unreachable() {
    // Allocate a transient URL behind a reachable wrapper, then
    // drop the reference + force a GC; the per-instance
    // `url_states` entry must be pruned by sweep, otherwise the
    // recycled `ObjectId` slot would inherit stale URL state.
    let mut vm = Vm::new();
    vm.eval("(function() { new URL('https://gone.example/'); })();")
        .unwrap();
    let pre = vm.inner.url_states.len();
    vm.inner.collect_garbage();
    let post = vm.inner.url_states.len();
    assert!(
        post < pre,
        "expected url_states sweep ({pre} → {post}) but no entries were pruned"
    );
}

#[test]
fn gc_keeps_url_alive_through_search_params_reference() {
    // The symmetric back-edge: hold only the `searchParams`
    // reference (URL wrapper unreachable from script), force GC,
    // then mutate and verify the URL wrapper plus its
    // `url_states` entry survived (mutation routing through
    // `usp_parent_url` would otherwise drop on the floor).
    // Pin the searchParams reference on `globalThis` so it
    // survives across `vm.eval` calls — `let` declarations are
    // script-local in this VM.
    let mut vm = Vm::new();
    vm.eval("globalThis.p = new URL('https://x.com/?a=1').searchParams;")
        .unwrap();
    vm.inner.collect_garbage();
    // After GC, mutate via the orphaned-from-the-script-side URL
    // reference — the rewrite path still finds the parent URL
    // because `usp_parent_url` keeps the URL marked.
    assert_eq!(
        eval_string(&mut vm, "p.append('b', '2'); p.toString();"),
        "a=1&b=2"
    );
}

// ---------------------------------------------------------------------------
// ObjectKind brand
// ---------------------------------------------------------------------------

#[test]
fn ctor_promotes_kind_to_url_variant() {
    let mut vm = Vm::new();
    let v = vm.eval("new URL('https://x.com/');").unwrap();
    let JsValue::Object(id) = v else {
        panic!("expected Object, got {v:?}");
    };
    assert!(matches!(vm.inner.get_object(id).kind, ObjectKind::URL));
}
