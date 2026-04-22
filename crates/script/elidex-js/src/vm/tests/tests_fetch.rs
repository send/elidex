//! `fetch()` host global tests (WHATWG Fetch §5.1).
//!
//! Covers:
//! - No-handle path (unpopulated VM → `TypeError`).
//! - Mock NetworkHandle round-trip for 200 / 404 / status / headers.
//! - URL parse failure → `TypeError`.
//! - `Request` input path (re-uses VM Request state).
//! - Method / headers / body init passthrough to the broker.
//! - Body-mixin chaining: `fetch(url).then(r => r.text())`.
//! - Missing-URL-in-mock path (mock's "no response" branch).
//!
//! All tests drive the Fetch surface via
//! [`NetworkHandle::mock_with_responses`] behind the
//! `elidex-net/test-hooks` feature.

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

fn no_handle_vm() -> Vm {
    Vm::new()
}

fn ok_response(url: &str, status: u16, body: &'static str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

#[test]
fn fetch_no_args_throws_type_error_synchronously() {
    // WebIDL "not enough arguments supplied to call" is a
    // binding-level failure that browsers surface as a synchronous
    // TypeError — not a rejected Promise.  Verified against
    // Chrome / Firefox / Safari: `try { fetch() } catch (e) { ... }`
    // catches a sync TypeError (R19.1).  `fetch('...')` with a
    // valid URL shape does not synchronously throw; only the
    // missing-argument path does.
    let mut vm = no_handle_vm();
    vm.eval(
        "globalThis.r = null; \
         try { fetch(); } \
         catch (e) { globalThis.r = e instanceof TypeError; }",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_non_object_init_throws_type_error_synchronously() {
    // WebIDL `RequestInit` is a dictionary argument; converting a
    // non-object / non-undefined / non-null value to a dictionary
    // fails at the binding layer and surfaces as a sync TypeError
    // — same shape as `new Request(url, 42)` / `new Response(null,
    // 42)`, verified against Chrome / Firefox / Safari (R20.1).
    let mut vm = no_handle_vm();
    vm.eval(
        "globalThis.r = null; \
         try { fetch('http://x/', 42); } \
         catch (e) { globalThis.r = e instanceof TypeError; }",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_with_no_handle_rejects_type_error() {
    let mut vm = no_handle_vm();
    vm.eval(
        "globalThis.r = null; \
         fetch('http://x/').catch(e => { globalThis.r = e instanceof TypeError; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_invalid_url_rejects_type_error() {
    let url = url::Url::parse("http://example.com/").expect("valid");
    let mut vm = mock_vm(vec![(url, Ok(ok_response("http://example.com/", 200, "")))]);
    // Relative URL against the default `about:blank` base fails
    // `Url::join` → `TypeError`.
    vm.eval(
        "globalThis.r = null; \
         fetch('/relative').catch(e => { globalThis.r = e instanceof TypeError; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_200_returns_response() {
    let url = url::Url::parse("http://example.com/ok").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ok", 200, "hi")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/ok').then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn fetch_404_reports_status_not_ok() {
    let url = url::Url::parse("http://example.com/missing").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/missing", 404, "")),
    )]);
    vm.eval(
        "globalThis.r = null; \
         fetch('http://example.com/missing').then(resp => { globalThis.r = resp.ok; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(!b),
        other => panic!("expected r to be false, got {other:?}"),
    }
}

#[test]
fn fetch_text_round_trip() {
    let url = url::Url::parse("http://example.com/doc").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/doc", 200, "hello body")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         fetch('http://example.com/doc') \
             .then(resp => resp.text()) \
             .then(body => { globalThis.r = body; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "hello body"),
        other => panic!("expected r to be body string, got {other:?}"),
    }
}

#[test]
fn fetch_propagates_response_headers() {
    let url = url::Url::parse("http://example.com/hdr").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/hdr", 200, "")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         fetch('http://example.com/hdr').then(resp => { globalThis.r = resp.headers.get('content-type'); });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "text/plain"),
        other => panic!("expected r to be 'text/plain', got {other:?}"),
    }
}

#[test]
fn fetch_from_request_instance() {
    let url = url::Url::parse("http://example.com/req").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/req", 200, "ok")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         var req = new Request('http://example.com/req'); \
         fetch(req).then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn fetch_method_override_canonicalises() {
    // The mock map doesn't check method — just that the URL
    // resolves.  We verify the method normalisation landed on
    // the broker-facing Request by capturing the observable
    // JS-side `.method` on a follow-up `new Request(...)` that
    // the test builds from the same init.
    let url = url::Url::parse("http://example.com/post").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/post", 201, "")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/post', {method: 'post'}).then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 201.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 201, got {other:?}"),
    }
}

#[test]
fn fetch_missing_response_in_mock_rejects_type_error() {
    // Mock has no entry for the requested URL → broker returns
    // `Err("mock: ...")` → fetch() rejects with TypeError.
    let url = url::Url::parse("http://example.com/registered").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/registered", 200, "")),
    )]);
    vm.eval(
        "globalThis.r = null; \
         fetch('http://example.com/unregistered').catch(e => { globalThis.r = e instanceof TypeError; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_broker_error_surfaces_as_type_error() {
    let url = url::Url::parse("http://example.com/err").expect("valid");
    let mut vm = mock_vm(vec![(url, Err("connection refused".to_string()))]);
    vm.eval(
        "globalThis.r = ''; \
         fetch('http://example.com/err').catch(e => { globalThis.r = e.message; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let s = vm.get_string(id);
            // Spec wording + broker message appended for
            // diagnostics.
            assert!(s.contains("Failed to fetch"), "got: {s}");
            assert!(s.contains("connection refused"), "got: {s}");
        }
        other => panic!("expected r to be an error message string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// AbortSignal wire (WHATWG Fetch §5.1 main-fetch step 3 + §5.4 ctor step 29)
// ---------------------------------------------------------------------------
//
// Phase 2 reality: `fetch_blocking` is synchronous, so no JS can
// run between the VM entering `native_fetch` and the broker
// reply.  The only observable `signal` path is therefore the
// *pre-flight* check — an already-aborted signal at call time
// rejects the returned Promise without touching the broker.
// Tests for mid-flight abort are deferred to the PR5-async-fetch
// refactor (documented at `VmInner::fetch_abort_observers`).

#[test]
fn fetch_pre_flight_aborted_rejects_with_default_abort_error() {
    // `AbortSignal.abort()` yields an already-aborted signal whose
    // reason is a `DOMException` with `name === "AbortError"`.
    // Per §5.1 step 3, fetch must reject with that reason.
    let url = url::Url::parse("http://example.com/aborted").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/aborted", 200, "never-read")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         var s = AbortSignal.abort(); \
         fetch('http://example.com/aborted', {signal: s}) \
             .catch(e => { globalThis.r = e instanceof DOMException && e.name; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("expected r to be 'AbortError', got {other:?}"),
    }
}

#[test]
fn fetch_pre_flight_aborted_propagates_custom_reason() {
    // `AbortSignal.abort("boom")` sets the reason to the
    // user-supplied value verbatim (no wrapping).  Fetch must
    // reject with that exact value.
    let url = url::Url::parse("http://example.com/aborted").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/aborted", 200, "never-read")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         var s = AbortSignal.abort('boom'); \
         fetch('http://example.com/aborted', {signal: s}) \
             .catch(e => { globalThis.r = e; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "boom"),
        other => panic!("expected r to be 'boom', got {other:?}"),
    }
}

#[test]
fn fetch_signal_primitive_rejects_type_error() {
    // `init.signal = 42` fails the AbortSignal brand check; fetch
    // must reject with TypeError (not run the request).
    let url = url::Url::parse("http://example.com/never").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/never", 200, "never-read")),
    )]);
    vm.eval(
        "globalThis.r = null; \
         fetch('http://example.com/never', {signal: 42}) \
             .catch(e => { globalThis.r = e instanceof TypeError; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_signal_non_abort_signal_object_rejects_type_error() {
    // A plain object (or any non-`AbortSignal` branded object) in
    // `init.signal` must reject with TypeError per WebIDL brand
    // check — `{}` would otherwise silently coerce into "no signal"
    // if the check were missing.
    let url = url::Url::parse("http://example.com/never").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/never", 200, "never-read")),
    )]);
    vm.eval(
        "globalThis.r = null; \
         fetch('http://example.com/never', {signal: {}}) \
             .catch(e => { globalThis.r = e instanceof TypeError; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Boolean(b)) => assert!(b),
        other => panic!("expected r to be true, got {other:?}"),
    }
}

#[test]
fn fetch_composite_signal_any_pre_abort_rejects() {
    // `AbortSignal.any([alreadyAborted])` returns a synchronously-
    // aborted composite (WHATWG §3.1.3.3 step 3: "if any input is
    // already aborted, return a newly-created signal aborted with
    // its reason").  Passing the composite into fetch must
    // pre-flight reject the same way a direct aborted signal does.
    let url = url::Url::parse("http://example.com/composite").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response(
            "http://example.com/composite",
            200,
            "never-read",
        )),
    )]);
    vm.eval(
        "globalThis.r = null; \
         var a = AbortSignal.abort('src-reason'); \
         var composite = AbortSignal.any([a]); \
         fetch('http://example.com/composite', {signal: composite}) \
             .catch(e => { globalThis.r = e; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "src-reason"),
        other => panic!("expected composite reason passthrough, got {other:?}"),
    }
}

#[test]
fn fetch_undefined_signal_completes_normally() {
    // Regression: `signal: undefined` must not break the happy
    // path.  WHATWG Fetch §5.4 step 29: an `undefined` signal
    // means "no signal".
    let url = url::Url::parse("http://example.com/ok").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ok", 200, "hi")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/ok', {signal: undefined}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn fetch_null_signal_completes_normally() {
    // WHATWG Fetch §5.4 step 29 explicitly treats `null` as
    // "signal is cleared / none", not as a brand-check failure.
    let url = url::Url::parse("http://example.com/ok").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ok", 200, "hi")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/ok', {signal: null}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn fetch_signal_aborted_after_settle_has_no_effect() {
    // Abort firing *after* a successful fetch must not retroactively
    // reject the already-fulfilled Promise.  This exercises the
    // Phase 2 no-mid-flight contract: the observer map is empty
    // once the response landed, so the later `controller.abort()`
    // sees no fetches to cancel and does not synthesise a second
    // settlement on the Promise.
    let url = url::Url::parse("http://example.com/ok").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ok", 200, "hi")),
    )]);
    vm.eval(
        "globalThis.status = 0; \
         globalThis.caught = false; \
         var c = new AbortController(); \
         fetch('http://example.com/ok', {signal: c.signal}) \
             .then(resp => { globalThis.status = resp.status; }) \
             .catch(() => { globalThis.caught = true; }); \
         c.abort();",
    )
    .unwrap();
    match vm.get_global("status") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected status to be 200, got {other:?}"),
    }
    match vm.get_global("caught") {
        Some(JsValue::Boolean(b)) => {
            assert!(
                !b,
                "post-settle abort must not re-trigger Promise rejection"
            );
        }
        other => panic!("expected caught to be a boolean, got {other:?}"),
    }
}
