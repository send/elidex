//! M4-12 PR5-async-fetch: WHATWG Fetch §4.6 forbidden-request-
//! header enforcement.
//!
//! The Request-companion `Headers` carries the `Request` guard,
//! which silently no-ops every mutation that targets a name in
//! the §4.6 list (Cookie / Host / Origin / Referer / Set-Cookie /
//! Connection / Content-Length / etc., plus the `Sec-` and
//! `Proxy-` byte-prefixes).  Standalone `new Headers()` retains
//! the `None` guard and accepts forbidden names — they are part
//! of the per-Request gate, not a global ban.

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};

use super::super::value::JsValue;
use super::super::Vm;

fn ok_response(url: &str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 200,
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

fn mock_vm(responses: Vec<(url::Url, Result<NetResponse, String>)>) -> (Vm, Rc<NetworkHandle>) {
    let mut vm = Vm::new();
    let handle = Rc::new(NetworkHandle::mock_with_responses(responses));
    vm.install_network_handle(Rc::clone(&handle));
    (vm, handle)
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

#[test]
fn standalone_headers_accept_forbidden_names() {
    // No guard on a bare `new Headers(...)` — the §4.6 filter is a
    // per-Request gate, not a global ban.  The user can build a
    // Headers with any name and inspect it freely; the filter only
    // fires when the Headers becomes a Request's companion.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.cookie = ''; \
         var h = new Headers(); \
         h.append('Cookie', 'a=1'); \
         globalThis.cookie = h.get('cookie');",
    )
    .unwrap();
    match vm.get_global("cookie") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "a=1"),
        other => panic!("standalone Headers must accept Cookie, got {other:?}"),
    }
}

#[test]
fn request_headers_drop_forbidden_init_entries() {
    // `new Request(url, {headers: {Cookie: 'a=1', X-Custom: 'ok'}})`:
    // the Cookie entry is silently dropped at companion-Headers
    // fill time; X-Custom passes through.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.cookie = 'unset'; \
         globalThis.custom = ''; \
         var req = new Request('http://example.com/', { \
             headers: {Cookie: 'a=1', 'X-Custom': 'ok'} \
         }); \
         globalThis.cookie = req.headers.get('cookie'); \
         globalThis.custom = req.headers.get('x-custom');",
    )
    .unwrap();
    match vm.get_global("cookie") {
        Some(JsValue::Null) => {}
        other => panic!("Cookie must be filtered → null, got {other:?}"),
    }
    match vm.get_global("custom") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "ok"),
        other => panic!("X-Custom must pass through, got {other:?}"),
    }
}

#[test]
fn request_headers_post_ctor_forbidden_append_silently_noops() {
    // After construction the user calls `req.headers.append('Cookie',
    // ...)` — must silently no-op rather than throw.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.threw = false; \
         globalThis.cookie = 'unset'; \
         var req = new Request('http://example.com/'); \
         try { req.headers.append('Cookie', 'a=1'); } \
         catch (_) { globalThis.threw = true; } \
         globalThis.cookie = req.headers.get('cookie');",
    )
    .unwrap();
    match vm.get_global("threw") {
        Some(JsValue::Boolean(b)) => assert!(!b, "forbidden append must not throw"),
        other => panic!("expected boolean, got {other:?}"),
    }
    match vm.get_global("cookie") {
        Some(JsValue::Null) => {}
        other => panic!("Cookie must remain unset after silent no-op, got {other:?}"),
    }
}

#[test]
fn request_headers_post_ctor_forbidden_set_silently_noops() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.host = 'unset'; \
         var req = new Request('http://example.com/', {headers: {'X-Custom': 'ok'}}); \
         req.headers.set('Host', 'evil.example.com'); \
         globalThis.host = req.headers.get('host');",
    )
    .unwrap();
    match vm.get_global("host") {
        Some(JsValue::Null) => {}
        other => panic!("Host must remain unset, got {other:?}"),
    }
}

#[test]
fn request_headers_delete_forbidden_silently_noops() {
    // `delete` on a forbidden name is also gated — even though
    // there's nothing to remove, the no-op preserves spec parity.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.threw = false; \
         var req = new Request('http://example.com/'); \
         try { req.headers.delete('Cookie'); } \
         catch (_) { globalThis.threw = true; }",
    )
    .unwrap();
    match vm.get_global("threw") {
        Some(JsValue::Boolean(b)) => assert!(!b),
        other => panic!("expected boolean, got {other:?}"),
    }
}

#[test]
fn request_headers_drop_sec_prefix() {
    // Per §4.6, every name starting with the case-insensitive
    // `Sec-` byte-prefix is forbidden.  Includes `Sec-Fetch-*`,
    // `Sec-WebSocket-*`, etc.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.sec = 'unset'; \
         var req = new Request('http://example.com/', { \
             headers: {'Sec-Fetch-Mode': 'cors'} \
         }); \
         globalThis.sec = req.headers.get('sec-fetch-mode');",
    )
    .unwrap();
    match vm.get_global("sec") {
        Some(JsValue::Null) => {}
        other => panic!("Sec-Fetch-Mode must be filtered, got {other:?}"),
    }
}

#[test]
fn request_headers_drop_proxy_prefix() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.px = 'unset'; \
         var req = new Request('http://example.com/', { \
             headers: {'Proxy-Authorization': 'Bearer x'} \
         }); \
         globalThis.px = req.headers.get('proxy-authorization');",
    )
    .unwrap();
    match vm.get_global("px") {
        Some(JsValue::Null) => {}
        other => panic!("Proxy-Authorization must be filtered, got {other:?}"),
    }
}

#[test]
fn fetch_url_input_init_headers_drop_forbidden_names() {
    // The URL-input fetch path does not allocate a companion
    // Headers — the entries snapshot bypasses the Request guard
    // path.  Forbidden filtering happens at snapshot time inside
    // `parse_init_overrides`.  Verified via the broker's
    // recorded-requests log.
    let url = url::Url::parse("http://example.com/api").expect("valid");
    let (mut vm, handle) = mock_vm(vec![(url.clone(), Ok(ok_response(url.as_str())))]);
    vm.eval(
        "fetch('http://example.com/api', { \
             headers: {Cookie: 'a=1', 'X-Custom': 'ok'} \
         });",
    )
    .unwrap();
    let recorded = handle.drain_recorded_requests();
    assert_eq!(recorded.len(), 1, "expected one outbound request");
    let req = &recorded[0];
    assert!(
        header_value(&req.headers, "cookie").is_none(),
        "Cookie must be filtered out of broker request: {:?}",
        req.headers
    );
    assert_eq!(
        header_value(&req.headers, "x-custom").as_deref(),
        Some("ok"),
        "X-Custom must reach the broker"
    );
}

#[test]
fn fetch_user_set_origin_dropped_in_favour_of_auto_attach() {
    // §4.6 forbids user-set `Origin`.  Cross-origin fetch attaches
    // its own Origin via `attach_default_origin` — the user value
    // is silently dropped first and the policy value wins.
    let url = url::Url::parse("http://other.example/api").expect("valid");
    let (mut vm, handle) = mock_vm(vec![(url.clone(), Ok(ok_response(url.as_str())))]);
    vm.inner.navigation.current_url = url::Url::parse("http://example.com/page").unwrap();
    vm.eval(
        "fetch('http://other.example/api', { \
             headers: {Origin: 'http://malicious.example'} \
         });",
    )
    .unwrap();
    let recorded = handle.drain_recorded_requests();
    assert_eq!(recorded.len(), 1, "expected one outbound request");
    let req = &recorded[0];
    let origin = header_value(&req.headers, "origin").expect("auto-attached Origin");
    assert_eq!(origin, "http://example.com");
}

#[test]
fn standalone_headers_set_cookie_works_until_attached_to_request() {
    // `Set-Cookie` is only forbidden on Request guard.  A bare
    // Headers can carry it.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.sc = ''; \
         var h = new Headers(); \
         h.append('Set-Cookie', 'sid=1'); \
         globalThis.sc = h.get('set-cookie');",
    )
    .unwrap();
    match vm.get_global("sc") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "sid=1"),
        other => panic!("standalone Set-Cookie must work, got {other:?}"),
    }
}
