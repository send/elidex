//! `Request` / `Response` interface tests (WHATWG Fetch §5.3 / §5.5).
//!
//! Covers ctor signatures, init-dict parsing, IDL readonly
//! attribute reads (internal-slot authoritative), `clone()` body
//! sharing, the three `Response` static factories, and the body-
//! status / statusText validation error paths.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

#[test]
fn request_ctor_from_url_string_defaults_method_get() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://example.com/a'); r.method + ' ' + r.url;"
        ),
        "GET http://example.com/a"
    );
}

#[test]
fn request_ctor_method_override_uppercases() {
    let mut vm = Vm::new();
    // WHATWG §5.3 step 23 canonicalises lowercase post / put / etc.
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://example.com/a', {method: 'post'}); r.method;"
        ),
        "POST"
    );
}

#[test]
fn request_ctor_forbidden_method_throws() {
    let mut vm = Vm::new();
    // `CONNECT` / `TRACE` / `TRACK` are forbidden per §4.6.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {method: 'CONNECT'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_ctor_headers_from_record() {
    let mut vm = Vm::new();
    // Init-dict `headers` member is filled into the companion
    // Headers object with the same lowercase-normalisation path
    // as `new Headers(init)`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {headers: {'X-A': '1'}}); r.headers.get('x-a');"
        ),
        "1"
    );
}

#[test]
fn request_clone_shares_body_text_equal() {
    let mut vm = Vm::new();
    // Body Arc is shared across the clone; both Requests report
    // the same `bodyUsed` (false) and reference identical data
    // through `body_data`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Request('http://x/', {method: 'POST', body: 'hi', headers: {'X-A': '1'}}); \
             var b = a.clone(); \
             b.method + ' ' + b.url + ' ' + b.headers.get('x-a') + ' ' + b.bodyUsed;"
        ),
        "POST http://x/ 1 false"
    );
}

#[test]
fn request_idl_url_resilient_to_delete() {
    let mut vm = Vm::new();
    // `Request.prototype.url` accessor reads from the internal
    // slot (`request_states`), so user-land `delete r.url` has no
    // effect on subsequent reads — matches PR5a2 R7.1 lesson.
    assert!(eval_bool(
        &mut vm,
        "var r = new Request('http://example.com/a'); \
         delete r.url; \
         r.url === 'http://example.com/a';"
    ));
}

#[test]
fn request_ctor_from_another_request_inherits_state() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Request('http://x/', {method: 'POST'}); \
             var b = new Request(a); \
             b.method + '|' + b.url;"
        ),
        "POST|http://x/"
    );
}

#[test]
fn request_ctor_rejects_non_string_non_request_input() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request(42); } catch (e) { r = e instanceof TypeError; } r;"
    ));
}

// ---------------------------------------------------------------------------
// `init.{mode, credentials, redirect, cache}` parsing — PR5-cors Stage 1.
// Spec defaults flow from the URL-input path; Request-clone path
// inherits unless `init.*` overrides.  Invalid enum strings throw
// TypeError per WebIDL §3.10.7.
// ---------------------------------------------------------------------------

#[test]
fn request_init_mode_no_cors_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {mode: 'no-cors'}); r.mode;"
        ),
        "no-cors"
    );
}

#[test]
fn request_init_mode_same_origin_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {mode: 'same-origin'}); r.mode;"
        ),
        "same-origin"
    );
}

#[test]
fn request_init_mode_navigate_throws() {
    let mut vm = Vm::new();
    // Spec §5.3 step 23: `init["mode"]` of "navigate" is reserved
    // for navigation requests and must throw from JS-facing ctors.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {mode: 'navigate'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_init_mode_unknown_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {mode: 'cors-please'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_init_credentials_omit_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {credentials: 'omit'}); r.credentials;"
        ),
        "omit"
    );
}

#[test]
fn request_init_credentials_include_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {credentials: 'include'}); r.credentials;"
        ),
        "include"
    );
}

#[test]
fn request_init_credentials_unknown_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {credentials: 'always'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_init_redirect_error_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {redirect: 'error'}); r.redirect;"
        ),
        "error"
    );
}

#[test]
fn request_init_redirect_manual_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {redirect: 'manual'}); r.redirect;"
        ),
        "manual"
    );
}

#[test]
fn request_init_redirect_unknown_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {redirect: 'sometimes'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_init_cache_no_store_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {cache: 'no-store'}); r.cache;"
        ),
        "no-store"
    );
}

#[test]
fn request_init_cache_force_cache_round_trips() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/', {cache: 'force-cache'}); r.cache;"
        ),
        "force-cache"
    );
}

#[test]
fn request_init_cache_unknown_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {cache: 'cache-it'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_init_defaults_when_not_set() {
    let mut vm = Vm::new();
    // Spec defaults for the URL-input path.
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Request('http://x/'); \
             r.mode + '|' + r.credentials + '|' + r.redirect + '|' + r.cache;"
        ),
        "cors|same-origin|follow|default"
    );
}

#[test]
fn request_clone_inherits_mode_credentials_redirect_cache() {
    let mut vm = Vm::new();
    // Cloning a Request with non-default options must carry them
    // forward when no override is supplied.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Request('http://x/', \
                 {mode: 'no-cors', credentials: 'omit', redirect: 'manual', cache: 'reload'}); \
             var b = new Request(a); \
             b.mode + '|' + b.credentials + '|' + b.redirect + '|' + b.cache;"
        ),
        "no-cors|omit|manual|reload"
    );
}

#[test]
fn request_clone_init_overrides_inherited_state() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Request('http://x/', {mode: 'no-cors', cache: 'reload'}); \
             var b = new Request(a, {mode: 'same-origin'}); \
             b.mode + '|' + b.cache;"
        ),
        "same-origin|reload"
    );
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[test]
fn response_ctor_defaults_status_200_type_default_ok_true() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Response(); \
             r.status + '|' + r.ok + '|' + r.statusText + '|' + r.type;"
        ),
        "200|true||default"
    );
}

#[test]
fn response_ctor_status_out_of_range_throws_range_error() {
    let mut vm = Vm::new();
    // < 200 or > 599 must reject (spec §5.5 "initialize a response" step 1).
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Response(null, {status: 99}); } \
         catch (e) { r = e instanceof RangeError; } r;"
    ));
}

#[test]
fn response_ctor_null_body_status_with_body_throws() {
    let mut vm = Vm::new();
    // 204 / 205 / 304 with a non-null body → TypeError per spec.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Response('oops', {status: 204}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn response_ctor_sets_content_type_for_string_body() {
    let mut vm = Vm::new();
    // String body adds `Content-Type: text/plain;charset=UTF-8`
    // unless the user-supplied headers already have one.
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Response('hi'); r.headers.get('content-type');"
        ),
        "text/plain;charset=UTF-8"
    );
}

#[test]
fn response_user_content_type_not_overridden() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = new Response('hi', {headers: {'Content-Type': 'application/x-test'}}); \
             r.headers.get('content-type');"
        ),
        "application/x-test"
    );
}

#[test]
fn response_headers_become_immutable_after_ctor() {
    let mut vm = Vm::new();
    // The companion Headers are flipped to `Immutable` at the end
    // of the ctor, so a subsequent `.append` throws TypeError.
    assert!(eval_bool(
        &mut vm,
        "var r = new Response('hi'); var thrown = false; \
         try { r.headers.append('x-a', '1'); } \
         catch (e) { thrown = e instanceof TypeError; } thrown;"
    ));
}

#[test]
fn response_idl_status_resilient_to_delete() {
    let mut vm = Vm::new();
    // Same internal-slot-authoritative pattern as Request.url.
    assert_eq!(
        eval_number(
            &mut vm,
            "var r = new Response(null, {status: 404}); delete r.status; r.status;"
        ),
        404.0
    );
}

#[test]
fn response_clone_preserves_state() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Response('hi', {status: 201, statusText: 'Created'}); \
             var b = a.clone(); \
             b.status + '|' + b.statusText + '|' + b.type;"
        ),
        "201|Created|default"
    );
}

#[test]
fn response_static_error_has_status_zero_and_type_error() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = Response.error(); r.status + '|' + r.type;"
        ),
        "0|error"
    );
}

#[test]
fn response_static_redirect_sets_location_header() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = Response.redirect('http://example.com/next', 302); \
             r.status + '|' + r.headers.get('location');"
        ),
        "302|http://example.com/next"
    );
}

#[test]
fn response_static_redirect_rejects_non_redirect_status() {
    let mut vm = Vm::new();
    // Only 301 / 302 / 303 / 307 / 308 are permitted.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { Response.redirect('http://x/', 200); } \
         catch (e) { r = e instanceof RangeError; } r;"
    ));
}

#[test]
fn response_static_json_sets_content_type_application_json() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var r = Response.json({a: 1}); r.headers.get('content-type');"
        ),
        "application/json"
    );
}

// --- R25: WHATWG Fetch §5.3 step 40 — GET/HEAD + body forbidden ---

#[test]
fn request_ctor_get_with_body_throws_type_error() {
    let mut vm = Vm::new();
    // WHATWG Fetch §5.3 step 40: GET with a body is a sync
    // TypeError (R25.1).
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {method:'GET', body:'x'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_ctor_head_with_body_throws_type_error() {
    let mut vm = Vm::new();
    // `HEAD` is symmetric to `GET` for this check (R25.1).
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {method:'HEAD', body:'x'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_ctor_default_method_get_with_body_throws() {
    let mut vm = Vm::new();
    // `method` defaults to `GET` when absent; step 40 applies to
    // the final state, so a body without an explicit method is
    // still a TypeError (R25.1).
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new Request('http://x/', {body:'x'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_ctor_get_with_null_body_ok() {
    let mut vm = Vm::new();
    // `body: null` explicitly clears the body — the final
    // Request has no body, so step 40 does not apply (R25.3
    // tri-state).
    assert_eq!(
        eval_string(
            &mut vm,
            "new Request('http://x/', {method:'GET', body:null}).method;"
        ),
        "GET"
    );
}

#[test]
fn request_ctor_post_with_body_ok() {
    let mut vm = Vm::new();
    // `POST` / any non-GET/HEAD method + body is fine (baseline).
    assert_eq!(
        eval_string(
            &mut vm,
            "new Request('http://x/', {method:'POST', body:'x'}).method;"
        ),
        "POST"
    );
}

#[test]
fn request_ctor_clone_post_with_method_get_throws() {
    let mut vm = Vm::new();
    // Clone path: source Request has a body; init overrides
    // method to GET without clearing the body → final state is
    // GET + body → TypeError (R25.1 checks *final* state, not
    // just `init.body`).
    assert!(eval_bool(
        &mut vm,
        "var post = new Request('http://x/', {method:'POST', body:'x'}); \
         var r = false; \
         try { new Request(post, {method:'GET'}); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn request_ctor_clone_post_with_method_get_and_null_body_ok() {
    let mut vm = Vm::new();
    // Clone path with explicit `body:null` clears the source's
    // body; final state is GET + no body → no TypeError (R25.3
    // tri-state handling).
    assert_eq!(
        eval_string(
            &mut vm,
            "var post = new Request('http://x/', {method:'POST', body:'x'}); \
             new Request(post, {method:'GET', body:null}).method;"
        ),
        "GET"
    );
}
