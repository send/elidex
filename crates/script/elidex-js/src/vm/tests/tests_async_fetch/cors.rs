//! PR5-cors Stages 3 / 4 / 5 + signal back-ref pruning + the
//! `pending_fetch_cors` meta-missing fail-closed regression.
//!
//! - **Stage 3** — same-origin reject + Origin / redirect /
//!   credentials thread; verifies the broker-bound
//!   `elidex_net::Request` carries the values selected by `init.*`
//!   plus the source document's origin.
//! - **Stage 4** — `Response.type` CORS classification matrix
//!   (basic / cors / opaque / opaqueredirect + header-filter +
//!   network-error rejection).
//! - **Stage 5** — cache-mode header injection (WHATWG Fetch §5.3
//!   step 30).  `force-cache` / `only-if-cached` are documented
//!   no-ops because elidex-net does not yet implement an HTTP
//!   cache layer.
//!
//! Companion to [`super::lifecycle`] (basic Promise lifecycle +
//! `install_network_handle`) and [`super::abort`]
//! (`controller.abort()` fan-out).

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{drain, mock_vm, ok_response};

fn vm_with_origin_and_mock(
    document_url: &str,
    target: &str,
    response: Result<NetResponse, String>,
) -> (Vm, Rc<NetworkHandle>) {
    let mut vm = Vm::new();
    vm.inner.navigation.current_url = url::Url::parse(document_url).expect("valid document URL");
    let parsed = url::Url::parse(target).expect("valid target URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![(parsed, response)]));
    vm.install_network_handle(handle.clone());
    (vm, handle)
}

// ---------------------------------------------------------------------------
// PR5-cors Stage 3: same-origin reject + Origin / redirect / credentials
// thread.  Verifies the broker-bound `elidex_net::Request` carries the
// values selected by `init.*` plus the source document's origin.
// ---------------------------------------------------------------------------

#[test]
fn fetch_threads_same_origin_credentials_redirect_to_broker() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "fetch('http://example.com/api', \
              {credentials: 'omit', redirect: 'manual'});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Omit);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Manual);
    assert_eq!(
        req.origin,
        Some(url::Url::parse("http://example.com/page").unwrap().origin())
    );
}

#[test]
fn fetch_threads_request_state_to_broker_when_input_is_request() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "var req = new Request('http://example.com/api', \
             {credentials: 'include', redirect: 'error'}); \
         fetch(req);",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Include);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Error);
}

#[test]
fn fetch_threads_request_mode_to_broker() {
    // PR5-cors-preflight: `init.mode` is threaded into the broker
    // `Request.mode` so the NetClient::send preflight stage can
    // distinguish Cors / NoCors / SameOrigin without round-trip
    // conversion.  Default for `fetch()` is `Cors`.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {mode: 'no-cors'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].mode, elidex_net::RequestMode::NoCors);
}

#[test]
fn fetch_default_mode_is_cors() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    // Spec default for the fetch() URL-string input path is
    // `Cors` (see the `build_net_request` URL-input branch in
    // fetch.rs).
    assert_eq!(logged[0].mode, elidex_net::RequestMode::Cors);
}

#[test]
fn fetch_init_overrides_request_state_for_redirect_credentials() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "var req = new Request('http://example.com/api', \
             {credentials: 'include', redirect: 'follow'}); \
         fetch(req, {credentials: 'omit', redirect: 'manual'});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Omit);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Manual);
}

#[test]
fn fetch_same_origin_mode_rejects_cross_origin_url_with_typeerror() {
    // mode='same-origin' + cross-origin URL → synchronous rejection
    // before the broker is even contacted.  The mock has no entry
    // for the target, so a successful broker dispatch would return
    // a "no response for ..." error — different from the
    // TypeError we expect.
    let (mut vm, _handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(ok_response("http://other.com/api", "should-not-reach")),
    );
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://other.com/api', {mode: 'same-origin'}) \
             .catch(e => { globalThis.r = e.message; });",
    )
    .unwrap();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let msg = vm.get_string(id);
            assert!(
                msg.contains("cross-origin") || msg.contains("same-origin"),
                "expected same-origin rejection, got {msg:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn fetch_same_origin_mode_passes_same_origin_url() {
    let (mut vm, _handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/api', {mode: 'same-origin'}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected 200, got {other:?}"),
    }
}

#[test]
fn opaque_origin_initiator_emits_origin_null_header() {
    // Copilot R4 regression: an opaque-origin script (data: /
    // about:blank) doing a CORS-mode cross-origin fetch must
    // send `Origin: null` so the server can satisfy the CORS
    // check against the spec-mandated serialisation of opaque
    // origins.  Pre-R4, `attach_default_origin` skipped any
    // non-HTTP(S) source, so opaque-origin fetches went out
    // without an Origin header and CORS gates that key on its
    // presence would silently fail.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "about:blank",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let origin_header = logged[0]
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("origin"))
        .map(|(_, v)| v.as_str());
    assert_eq!(
        origin_header,
        Some("null"),
        "opaque initiator must send `Origin: null` for cross-origin HTTP target"
    );
}

#[test]
fn fetch_threads_opaque_origin_for_about_blank_initiator() {
    // Copilot R3 fix: `about:blank` script-initiated fetches
    // produce an opaque origin (Origin::Opaque, ascii_serialization
    // = "null") rather than `None`.  The previous behaviour —
    // returning `None` for non-HTTP(S) — caused the classifier to
    // short-circuit to `Basic`, which was a CORS bypass.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "about:blank",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let origin = logged[0]
        .origin
        .as_ref()
        .expect("script-initiated fetch always carries Some(origin)");
    // Opaque origins serialise as "null" per HTML §3.2.1.2.
    assert_eq!(origin.ascii_serialization(), "null");
    assert!(!origin.is_tuple());
}

// ---------------------------------------------------------------------------
// PR5-cors Stage 4: response_type CORS classification matrix.  These
// tests verify the JS-observable `Response.type` value (and the
// associated header / body / status / url filtering) for each fetch
// scenario.
// ---------------------------------------------------------------------------

fn cors_response(url: &str, allow_origin: Option<&str>) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("x-custom".to_string(), "secret".to_string()),
    ];
    if let Some(origin) = allow_origin {
        headers.push((
            "Access-Control-Allow-Origin".to_string(),
            origin.to_string(),
        ));
    }
    NetResponse {
        status: 200,
        headers,
        body: bytes::Bytes::from_static(b"ok"),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
        is_redirect_tainted: false,
        credentialed_network: false,
    }
}

fn redirect_302_response(url: &str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 302,
        headers: vec![("location".to_string(), "/elsewhere".to_string())],
        body: bytes::Bytes::new(),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
        is_redirect_tainted: false,
        credentialed_network: false,
    }
}

fn read_string(vm: &Vm, key: &str) -> String {
    match vm.get_global(key) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected {key} to be a string, got {other:?}"),
    }
}

#[test]
fn response_type_basic_for_same_origin() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "globalThis.t = ''; \
         fetch('http://example.com/api').then(r => { globalThis.t = r.type; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "basic");
}

#[test]
fn response_type_cors_for_cross_origin_with_acao() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response(
            "http://other.com/api",
            Some("http://example.com"),
        )),
    );
    vm.eval(
        "globalThis.t = ''; \
         fetch('http://other.com/api').then(r => { globalThis.t = r.type; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "cors");
}

#[test]
fn response_type_opaque_for_no_cors_cross_origin() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", None)),
    );
    vm.eval(
        "globalThis.t = ''; \
         globalThis.s = 0; \
         fetch('http://other.com/api', {mode: 'no-cors'}) \
             .then(r => { globalThis.t = r.type; globalThis.s = r.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "opaque");
    // Opaque responses report status 0.
    match vm.get_global("s") {
        Some(JsValue::Number(n)) => assert!((n - 0.0).abs() < f64::EPSILON),
        other => panic!("expected status 0, got {other:?}"),
    }
}

#[test]
fn cors_check_failure_rejects_with_typeerror() {
    // Cross-origin cors mode without an `Access-Control-Allow-Origin`
    // header → spec says this becomes a network error and the
    // Promise rejects with TypeError.  The mock returns a 200 OK
    // without ACAO; the classifier treats it as `NetworkError`.
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", None)),
    );
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://other.com/api') \
             .catch(e => { globalThis.r = e.message; });",
    )
    .unwrap();
    drain(&mut vm);
    let msg = read_string(&vm, "r");
    assert!(
        msg.to_lowercase().contains("cors") || msg.contains("Access-Control"),
        "expected CORS rejection, got: {msg}"
    );
}

#[test]
fn cors_filter_drops_non_safelisted_headers() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", Some("*"))),
    );
    vm.eval(
        "globalThis.ct = ''; \
         globalThis.cust = 'unset'; \
         fetch('http://other.com/api').then(r => { \
             globalThis.ct = r.headers.get('content-type'); \
             globalThis.cust = r.headers.get('x-custom'); \
         });",
    )
    .unwrap();
    drain(&mut vm);
    // CORS-safelisted (`content-type`) is exposed.
    assert_eq!(read_string(&vm, "ct"), "application/json");
    // Custom header that is not in the safelist and not in
    // `Access-Control-Expose-Headers` — `headers.get` returns
    // null (== JS undefined for the test global slot? No — null
    // string-coerces to `null`).  Spec: when name not present,
    // `Headers.prototype.get` returns null.
    match vm.get_global("cust") {
        Some(JsValue::Null) => {}
        other => panic!("expected null for filtered header, got {other:?}"),
    }
}

#[test]
fn opaque_redirect_response_for_manual_redirect_3xx() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(redirect_302_response("http://example.com/api")),
    );
    vm.eval(
        "globalThis.t = ''; \
         globalThis.s = -1; \
         fetch('http://example.com/api', {redirect: 'manual'}) \
             .then(r => { globalThis.t = r.type; globalThis.s = r.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "opaqueredirect");
    match vm.get_global("s") {
        Some(JsValue::Number(n)) => assert!((n - 0.0).abs() < f64::EPSILON),
        other => panic!("expected status 0, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// PR5-cors Stage 5: cache-mode header injection (WHATWG Fetch §5.3 step 30).
// `force-cache` / `only-if-cached` are documented no-ops because elidex-net
// does not yet implement an HTTP cache layer.
// ---------------------------------------------------------------------------

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

#[test]
fn cache_no_store_appends_cache_control_no_store() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'no-store'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("no-store")
    );
}

#[test]
fn cache_reload_appends_cache_control_no_cache_and_pragma() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'reload'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("no-cache")
    );
    assert_eq!(header_value(&logged[0].headers, "pragma"), Some("no-cache"));
}

#[test]
fn cache_no_cache_appends_max_age_zero() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'no-cache'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("max-age=0")
    );
}

#[test]
fn cache_default_does_not_inject_headers() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'default'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert!(header_value(&logged[0].headers, "cache-control").is_none());
    assert!(header_value(&logged[0].headers, "pragma").is_none());
}

#[test]
fn user_set_cache_control_is_preserved() {
    // PR5-cors's cache-mode injection only fires when the same
    // header isn't already present — user-set headers win.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "fetch('http://example.com/api', \
             {cache: 'no-store', headers: {'Cache-Control': 'public, max-age=60'}});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("public, max-age=60")
    );
}

// ---------------------------------------------------------------------------
// Signal back-ref pruning + `pending_fetch_cors` meta-missing fail-closed
// regression — both keyed off the CORS-aware settle path so they live with
// the rest of the CORS coverage.
// ---------------------------------------------------------------------------

#[test]
fn signal_back_refs_pruned_on_settlement() {
    // After a successful `tick_network` settle, the back-refs
    // table must be empty — otherwise a subsequent `controller.
    // abort()` would chase a stale FetchId and try to send a
    // redundant CancelFetch.
    let url = url::Url::parse("http://example.com/sig-prune").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/sig-prune", "ok")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         globalThis.c = new AbortController(); \
         fetch('http://example.com/sig-prune', {signal: c.signal}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(
        vm.inner.pending_fetches.len(),
        0,
        "pending_fetches must be empty after settle"
    );
    assert_eq!(
        vm.inner.fetch_signal_back_refs.len(),
        0,
        "fetch_signal_back_refs must be empty after settle"
    );
    // Aborting now is a no-op for the already-settled fetch — and
    // must not double-fire any Promise reaction.
    vm.eval("c.abort();").unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("late abort must not retro-reject: {other:?}"),
    }
}

/// Copilot R2 regression: when `settle_fetch` lands on a
/// `FetchId` whose `pending_fetch_cors` entry is missing (an
/// internal bookkeeping bug, not a user-visible state), the
/// Promise must be **rejected** with a `TypeError` rather than
/// silently fall through to a permissive `Basic` classification
/// (which would disable CORS enforcement for that fetch).
///
/// We can't easily reproduce a "real" bookkeeping bug from the
/// public API, so this test reaches into `vm.inner` to drop the
/// CORS meta entry between dispatch and `tick_network` — the
/// `pending_fetches` Promise survives but its meta is gone.
#[test]
fn settle_fetch_rejects_when_cors_meta_missing() {
    let url = url::Url::parse("http://example.com/api").unwrap();
    let mut vm = mock_vm(vec![(url, Ok(ok_response("http://example.com/api", "ok")))]);
    vm.inner.navigation.current_url = url::Url::parse("http://example.com/page").unwrap();
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://example.com/api') \
             .then(resp => { globalThis.r = 'resolved-' + resp.status; }) \
             .catch(e => { globalThis.r = 'rejected-' + e.message; });",
    )
    .unwrap();
    // Sanity: dispatch installed both maps.
    assert_eq!(vm.inner.pending_fetches.len(), 1);
    assert_eq!(vm.inner.pending_fetch_cors.len(), 1);
    // Drop the CORS meta entry only — this simulates the
    // bookkeeping bug Copilot R2 flagged.
    vm.inner.pending_fetch_cors.clear();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let msg = vm.get_string(id);
            assert!(
                msg.starts_with("rejected-") && msg.contains("missing CORS metadata"),
                "expected fail-closed rejection, got: {msg}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}
