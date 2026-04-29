//! PR6: cookie + referrer wiring tests.
//!
//! Three slices:
//!
//! 1. `document.cookie` round-trips via the shell-owned [`CookieJar`]
//!    installed on `HostData` (HttpOnly + Secure-on-non-HTTPS filters
//!    happen inside the jar; we just verify the surface).
//! 2. `document.referrer` reads from `NavigationState::referrer`,
//!    populated via [`Vm::set_navigation_referrer`].
//! 3. `fetch()` injects the `Referer` header per the WHATWG Fetch
//!    default policy `strict-origin-when-cross-origin`.  We use a
//!    mock `NetworkHandle` whose request log we drain to inspect the
//!    final headers.

#![cfg(feature = "engine")]

use std::rc::Rc;
use std::sync::Arc;

use elidex_ecs::EcsDom;
use elidex_net::broker::NetworkHandle;
use elidex_net::{CookieJar, HttpVersion, Response as NetResponse};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

// ---------------------------------------------------------------------------
// document.cookie
// ---------------------------------------------------------------------------

fn ok_response(url: &str, status: u16, body: &'static str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status,
        headers: Vec::new(),
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

/// Bind `vm` against fresh test scaffolding pointing at `current_url`,
/// optionally installing a fresh `CookieJar` so `document.cookie` is
/// observable.  Returns the jar (`None` when `install_jar` is false) so
/// the caller can pre-seed cookies if needed.
///
/// # Safety
///
/// `session` and `dom` must outlive the bind cycle.  Caller drives
/// `vm.unbind()` before they drop.
#[allow(unsafe_code)]
unsafe fn bind_at_url(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    current_url: &str,
    install_jar: bool,
) -> Option<Arc<CookieJar>> {
    let doc = dom.create_document_root();
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let url = url::Url::parse(current_url).expect("valid URL");
    vm.inner.navigation.current_url = url;
    if !install_jar {
        return None;
    }
    let jar = Arc::new(CookieJar::new());
    vm.host_data()
        .expect("host_data installed by bind_vm")
        .install_cookie_jar(jar.clone());
    Some(jar)
}

#[test]
fn cookie_default_is_empty_when_no_jar() {
    // With document bound but no CookieJar, the surface stays
    // cookie-averse and the getter returns the empty string.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    let v = vm.eval("document.cookie").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string, got {v:?}");
    };
    assert_eq!(vm.get_string(id), "");

    vm.unbind();
}

#[test]
fn cookie_setter_no_op_when_no_jar() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    // Setter must not throw and must leave the getter at "".
    let v = vm
        .eval("document.cookie = 'session=abc'; document.cookie")
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "");

    vm.unbind();
}

#[test]
fn cookie_setter_no_jar_does_not_throw_on_symbol() {
    // Cookie-averse Documents must silently ignore assignments
    // even when the value is a Symbol — the spec's no-op contract
    // wins over WebIDL's USVString coercion-throws-TypeError step
    // for our self-hosted setter, mirroring the original PR4f
    // stub's non-throwing behaviour.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    // Must not throw — `Symbol()` would otherwise fail USVString
    // coercion if we coerced before checking the jar.
    vm.eval("document.cookie = Symbol('s'); document.cookie")
        .unwrap();

    vm.unbind();
}

#[test]
fn cookie_setter_writes_to_jar_and_getter_reads_back() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
    };

    let v = vm
        .eval("document.cookie = 'session=abc'; document.cookie")
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "session=abc");

    vm.unbind();
}

#[test]
fn cookie_getter_filters_http_only_cookies() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
        .expect("install_jar=true returns Some")
    };

    // Seed via the network-side API (which can set HttpOnly).
    // `store_from_response` takes a header list, so we hand it two
    // synthetic `Set-Cookie` headers.
    let url = url::Url::parse("https://example.com/").unwrap();
    jar.store_from_response(
        &url,
        &[
            (
                "Set-Cookie".to_string(),
                "session=abc; HttpOnly".to_string(),
            ),
            ("Set-Cookie".to_string(), "theme=dark".to_string()),
        ],
    );

    let v = vm.eval("document.cookie").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    // Only the non-HttpOnly cookie reaches the script.
    assert_eq!(vm.get_string(id), "theme=dark");

    vm.unbind();
}

#[test]
fn cookie_getter_returns_multiple_cookies_separated_by_semicolons() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
    };

    let v = vm
        .eval(
            "document.cookie = 'a=1';
             document.cookie = 'b=2';
             document.cookie",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    let s = vm.get_string(id);
    // Cookie ordering follows RFC 6265 (longest-path first, then
    // earliest creation); two equal-path cookies preserve insertion
    // order.  Either pair containing both names is acceptable.
    assert!(
        s == "a=1; b=2" || s == "b=2; a=1",
        "unexpected cookie string: {s}"
    );

    vm.unbind();
}

// ---------------------------------------------------------------------------
// document.referrer
// ---------------------------------------------------------------------------

#[test]
fn referrer_default_is_empty() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    let v = vm.eval("document.referrer").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "");

    vm.unbind();
}

#[test]
fn referrer_returns_set_url_after_set_navigation_referrer() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    vm.set_navigation_referrer(Some(
        url::Url::parse("https://prev.example.com/path?q=1").unwrap(),
    ));
    let v = vm.eval("document.referrer").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "https://prev.example.com/path?q=1");

    vm.unbind();
}

#[test]
fn referrer_setter_strips_fragment_and_userinfo() {
    // `document.referrer` shares the WHATWG Fetch §3.2.5 referrer
    // serialisation surface with the Referer header, which never
    // exposes fragments or basic-auth credentials.  Verify the
    // setter strips both before storing.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    vm.set_navigation_referrer(Some(
        url::Url::parse("https://user:pw@prev.example.com/path?q=1#secret").unwrap(),
    ));

    let v = vm.eval("document.referrer").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(
        vm.get_string(id),
        "https://prev.example.com/path?q=1",
        "fragment and userinfo must not leak through document.referrer"
    );

    vm.unbind();
}

#[test]
fn referrer_clears_back_to_empty_on_none() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };

    vm.set_navigation_referrer(Some(url::Url::parse("https://prev.com/").unwrap()));
    vm.set_navigation_referrer(None);
    let v = vm.eval("document.referrer").unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// document.cookie / referrer brand-bypass (Copilot R1)
// ---------------------------------------------------------------------------
//
// `document_receiver` returns `Ok(None)` when the receiver is a
// non-HostObject (e.g. a plain `{}` from
// `getter.call({})` / `setter.call({}, '...')`).  The VM's other
// document accessors short-circuit on that branch; the cookie /
// referrer accessors must do the same so detached calls cannot leak
// or mutate the bound document's storage.

#[test]
fn cookie_getter_call_with_plain_object_returns_empty_string() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
        .expect("install_jar=true returns Some")
    };
    let url = url::Url::parse("https://example.com/").unwrap();
    jar.store_from_response(
        &url,
        &[("Set-Cookie".to_string(), "leaked=yes".to_string())],
    );

    // Detached call: receiver is a plain object, not the bound
    // Document.  The brand check must short-circuit before the jar
    // is read.
    let v = vm
        .eval(
            "var get = Object.getOwnPropertyDescriptor(document, 'cookie').get;
             get.call({});",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(
        vm.get_string(id),
        "",
        "plain-object receiver must not leak the bound document's cookies"
    );

    vm.unbind();
}

#[test]
fn cookie_setter_call_with_plain_object_does_not_mutate_jar() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
        .expect("install_jar=true returns Some")
    };

    vm.eval(
        "var set = Object.getOwnPropertyDescriptor(document, 'cookie').set;
         set.call({}, 'malicious=1');",
    )
    .unwrap();

    let url = url::Url::parse("https://example.com/").unwrap();
    let cookies = jar.cookies_for_script(&url);
    assert_eq!(
        cookies, "",
        "plain-object receiver must not mutate the bound document's cookie jar"
    );

    vm.unbind();
}

#[test]
fn cookie_getter_on_cloned_document_returns_empty_string() {
    // `document.cloneNode(true)` produces a HostObject whose kind is
    // `Document` — it passes `document_receiver`'s brand check but
    // is not the bound browsing-context document.  The accessor
    // must treat it as cookie-averse so cookies cannot leak through
    // a detached clone.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
        .expect("install_jar=true returns Some")
    };
    let url = url::Url::parse("https://example.com/").unwrap();
    jar.store_from_response(
        &url,
        &[("Set-Cookie".to_string(), "leaked=yes".to_string())],
    );

    let v = vm
        .eval(
            "var clone = document.cloneNode(true);
             var get = Object.getOwnPropertyDescriptor(document, 'cookie').get;
             get.call(clone);",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(
        vm.get_string(id),
        "",
        "cloned Document receiver must not leak the bound document's cookies"
    );

    vm.unbind();
}

#[test]
fn cookie_setter_on_cloned_document_does_not_mutate_jar() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let jar = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            true,
        )
        .expect("install_jar=true returns Some")
    };

    vm.eval(
        "var clone = document.cloneNode(true);
         var set = Object.getOwnPropertyDescriptor(document, 'cookie').set;
         set.call(clone, 'leak=1');",
    )
    .unwrap();

    let url = url::Url::parse("https://example.com/").unwrap();
    let cookies = jar.cookies_for_script(&url);
    assert_eq!(
        cookies, "",
        "cloned Document receiver must not mutate the bound document's cookie jar"
    );

    vm.unbind();
}

#[test]
fn referrer_getter_on_cloned_document_returns_empty_string() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };
    vm.set_navigation_referrer(Some(
        url::Url::parse("https://leaked-referrer.example/").unwrap(),
    ));

    let v = vm
        .eval(
            "var clone = document.cloneNode(true);
             var get = Object.getOwnPropertyDescriptor(document, 'referrer').get;
             get.call(clone);",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(
        vm.get_string(id),
        "",
        "cloned Document receiver must not leak NavigationState.referrer"
    );

    vm.unbind();
}

#[test]
fn referrer_getter_call_with_plain_object_returns_empty_string() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    #[allow(unsafe_code)]
    let _ = unsafe {
        bind_at_url(
            &mut vm,
            &mut session,
            &mut dom,
            "https://example.com/",
            false,
        )
    };
    vm.set_navigation_referrer(Some(
        url::Url::parse("https://leaked-referrer.example/").unwrap(),
    ));

    let v = vm
        .eval(
            "var get = Object.getOwnPropertyDescriptor(document, 'referrer').get;
             get.call({});",
        )
        .unwrap();
    let JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(
        vm.get_string(id),
        "",
        "plain-object receiver must not leak NavigationState.referrer"
    );

    vm.unbind();
}

// ---------------------------------------------------------------------------
// fetch Referer header (strict-origin-when-cross-origin)
// ---------------------------------------------------------------------------

fn vm_with_url_and_mock(current_url: &str, mocks: Vec<&str>) -> (Vm, Rc<NetworkHandle>) {
    let mut vm = Vm::new();
    let url = url::Url::parse(current_url).expect("valid URL");
    vm.inner.navigation.current_url = url;
    let responses: Vec<_> = mocks
        .into_iter()
        .map(|u| {
            let parsed = url::Url::parse(u).expect("valid mock URL");
            (parsed.clone(), Ok(ok_response(u, 200, "")))
        })
        .collect();
    let handle = Rc::new(NetworkHandle::mock_with_responses(responses));
    vm.install_network_handle(handle.clone());
    (vm, handle)
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Drain the mock handle's request log and assert that exactly one
/// request was sent, returning it.  Tests that expect a single
/// `fetch()` call use this so a missing or duplicated request fails
/// with a clear message instead of an out-of-bounds index panic.
fn single_logged_request(handle: &NetworkHandle) -> elidex_net::Request {
    let mut logged = handle.drain_recorded_requests();
    assert_eq!(
        logged.len(),
        1,
        "expected exactly one fetch request, got {}",
        logged.len()
    );
    logged.remove(0)..Default::default()
}

#[test]
fn fetch_same_origin_attaches_full_referer() {
    let (mut vm, handle) = vm_with_url_and_mock(
        "https://example.com/page#frag",
        vec!["https://example.com/api"],
    );
    vm.eval("fetch('https://example.com/api');").unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    // Same-origin: full URL with fragment stripped.
    assert_eq!(referer, "https://example.com/page");
}

#[test]
fn fetch_cross_origin_attaches_origin_only_referer() {
    let (mut vm, handle) =
        vm_with_url_and_mock("https://example.com/page", vec!["https://other.com/api"]);
    vm.eval("fetch('https://other.com/api');").unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    // Cross-origin same-TLS: origin only.
    assert_eq!(referer, "https://example.com");
}

#[test]
fn fetch_https_to_http_omits_referer() {
    let (mut vm, handle) =
        vm_with_url_and_mock("https://example.com/page", vec!["http://other.com/api"]);
    vm.eval("fetch('http://other.com/api');").unwrap();
    let req = single_logged_request(&handle);
    assert!(
        header_value(&req.headers, "referer").is_none(),
        "TLS downgrade must strip the Referer header"
    );
}

#[test]
fn fetch_about_blank_source_omits_referer() {
    let (mut vm, handle) = vm_with_url_and_mock("about:blank", vec!["https://example.com/api"]);
    vm.eval("fetch('https://example.com/api');").unwrap();
    let req = single_logged_request(&handle);
    assert!(
        header_value(&req.headers, "referer").is_none(),
        "non-network source schemes must not produce a Referer"
    );
}

#[test]
fn fetch_strips_userinfo_and_fragment_from_referer() {
    let (mut vm, handle) = vm_with_url_and_mock(
        "https://user:pw@example.com/page?q=1#frag",
        vec!["https://example.com/api"],
    );
    vm.eval("fetch('https://example.com/api');").unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    // Userinfo and fragment stripped per the policy; query preserved.
    assert_eq!(referer, "https://example.com/page?q=1");
}

#[test]
fn fetch_user_set_referer_is_dropped_by_forbidden_header_filter() {
    // M4-12 PR5-async-fetch: WHATWG Fetch §4.6 forbidden-request-
    // header enforcement silently drops a caller-set `Referer`
    // before the auto-attach step runs, so the outgoing Request
    // carries the policy-derived value (cross-origin → source
    // origin only) rather than the user override.  Pre-PR5 (no
    // guard) the override won — that test reflected the legacy
    // behaviour and is updated alongside the guard landing.
    let (mut vm, handle) =
        vm_with_url_and_mock("https://example.com/page", vec!["https://other.com/api"]);
    vm.eval("fetch('https://other.com/api', {headers: {'Referer': 'https://manual.example/'}});")
        .unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    assert_eq!(referer, "https://example.com");
}

#[test]
fn fetch_http_to_http_attaches_origin_referer() {
    let (mut vm, handle) =
        vm_with_url_and_mock("http://example.com/page", vec!["http://other.com/api"]);
    vm.eval("fetch('http://other.com/api');").unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    // Cross-origin HTTP→HTTP: no TLS downgrade, send origin.
    assert_eq!(referer, "http://example.com");
}

#[test]
fn fetch_http_source_to_https_target_attaches_origin_referer() {
    let (mut vm, handle) =
        vm_with_url_and_mock("http://example.com/page", vec!["https://other.com/api"]);
    vm.eval("fetch('https://other.com/api');").unwrap();
    let req = single_logged_request(&handle);
    let referer = header_value(&req.headers, "referer").expect("referer present");
    // HTTP → HTTPS is a "TLS upgrade" (not downgrade); cross-origin
    // → origin only.
    assert_eq!(referer, "http://example.com");
}
