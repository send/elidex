//! End-to-end integration for the M4-12 fetch/Request/Response/
//! Headers/Blob surface.  Exercises multi-object chains (fetch →
//! clone → text/json) and cross-API error contracts (`TypeError`
//! on invalid URL, `RangeError` on out-of-range status, etc.) so
//! a regression in any one component breaks a real user scenario
//! rather than only a narrow unit test.
//!
//! These tests deliberately live in their own file so the
//! per-interface test modules (`tests_fetch`, `tests_body_mixin`,
//! `tests_headers`, `tests_request_response`, `tests_blob`) stay
//! focused on their own API surface.  Running a fetch from a
//! headers test would hide a coverage gap the other way around.

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};

use super::super::value::JsValue;
use super::super::Vm;

fn mock_vm(responses: Vec<(url::Url, Result<NetResponse, String>)>) -> Vm {
    let mut vm = Vm::new();
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(responses)));
    vm
}

fn json_response(url: &str, body: &'static str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 200,
        headers: vec![("content-type".to_string(), "application/json".to_string())],
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

#[test]
fn fetch_clone_lets_body_be_consumed_twice() {
    // `Response.clone()` yields a second Response that shares the
    // same body bytes but an independent `bodyUsed` latch, so a
    // single fetch can be parsed twice without an extra network
    // round-trip.  This is the idiom service worker handlers use:
    // read once for parsing, again for forwarding.
    let url = url::Url::parse("http://example.com/api").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(json_response("http://example.com/api", "{\"v\":42}")),
    )]);
    vm.eval(
        "globalThis.r_json = 0; \
         globalThis.r_text = ''; \
         fetch('http://example.com/api').then(resp => { \
             var copy = resp.clone(); \
             resp.json().then(o => { globalThis.r_json = o.v; }); \
             copy.text().then(t => { globalThis.r_text = t; }); \
         });",
    )
    .unwrap();
    match vm.get_global("r_json") {
        Some(JsValue::Number(n)) => assert!((n - 42.0).abs() < f64::EPSILON),
        other => panic!("expected r_json to be 42, got {other:?}"),
    }
    match vm.get_global("r_text") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "{\"v\":42}"),
        other => panic!("expected r_text to be JSON body, got {other:?}"),
    }
}

#[test]
fn response_from_multi_part_blob_round_trips_text() {
    // Multi-part Blob concatenates its parts in order (WHATWG File
    // API §3.3 step 3).  Wrapping in a Response and reading back
    // via `.text()` must recover the same byte stream, confirming
    // the Blob → body-store → UTF-8 decode path is consistent
    // across the three crates involved (Blob storage, body mixin
    // read, StringPool intern).
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.r = ''; \
         new Response(new Blob(['a', 'b', 'c'])).text() \
             .then(t => { globalThis.r = t; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "abc"),
        other => panic!("expected r to be 'abc', got {other:?}"),
    }
}

#[test]
fn headers_iteration_sorts_lowercased_names_with_combined_values() {
    // WHATWG Fetch §5.2 "sort and combine": iteration lowercases
    // names, byte-sorts them in ascending order, and joins
    // duplicate-name values with `", "`.  Three names inserted
    // out of ASCII order must iterate out sorted, and a
    // duplicate name with two `append` calls must appear once
    // with the values combined.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.r = ''; \
         var h = new Headers(); \
         h.append('X-B', '1'); \
         h.append('A-Thing', '2'); \
         h.append('Z-Last', '3'); \
         h.append('A-Thing', 'extra'); \
         var collected = []; \
         h.forEach((v, k) => collected.push(k + '=' + v)); \
         globalThis.r = collected.join('|');",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(
            vm.get_string(id),
            "a-thing=2, extra|x-b=1|z-last=3",
            "expected sort-and-combine output"
        ),
        other => panic!("expected r to be sort-and-combine CSV, got {other:?}"),
    }
}

#[test]
fn response_redirect_static_sets_status_and_location() {
    // `Response.redirect(url, 302)` returns an opaque-redirect-
    // shaped Response: spec-required status in 30x, headers
    // carrying the absolute Location, body empty.  Confirms the
    // static factory routes through the same Immutable-headers
    // path as a regular Response ctor (so user code can't
    // mutate the Location header afterwards).
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.r_status = 0; \
         globalThis.r_loc = ''; \
         var r = Response.redirect('http://example.com/target', 302); \
         globalThis.r_status = r.status; \
         globalThis.r_loc = r.headers.get('location');",
    )
    .unwrap();
    match vm.get_global("r_status") {
        Some(JsValue::Number(n)) => assert!((n - 302.0).abs() < f64::EPSILON),
        other => panic!("expected r_status to be 302, got {other:?}"),
    }
    match vm.get_global("r_loc") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "http://example.com/target"),
        other => panic!("expected r_loc to be absolute URL, got {other:?}"),
    }
}

#[test]
fn response_ctor_rejects_out_of_range_status_with_range_error() {
    // WHATWG §5.5 "initialize a response" step 1: `init.status`
    // must be in [200, 599]; out of range throws RangeError
    // (distinct from TypeError).  The ctor is the only place this
    // distinction is observable at the Fetch surface, so the
    // assertion also pins the `f64_to_uint16` → range check
    // ordering.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.r = ''; \
         try { new Response('', {status: 900}); } \
         catch (e) { globalThis.r = e instanceof RangeError ? e.name : 'not-range'; }",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "RangeError"),
        other => panic!("expected RangeError, got {other:?}"),
    }
}

#[test]
fn request_ctor_rejects_invalid_url_with_type_error() {
    // `new Request('/relative')` with the default `about:blank`
    // navigation base has no host to join against, so
    // `Url::parse` / `base.join` both fail → TypeError (not
    // RangeError / DOMException).  Mirrors the fetch-side
    // `fetch_invalid_url_rejects_type_error` but exercises the
    // synchronous ctor path rather than the Promise-reject path.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.r = ''; \
         try { new Request('/relative'); } \
         catch (e) { globalThis.r = e instanceof TypeError ? e.name : 'not-type'; }",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected TypeError, got {other:?}"),
    }
}
