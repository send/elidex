//! `fetch(input, init?)` host global (WHATWG Fetch §5.1).
//!
//! Routes a JS-level fetch request through the embedding-supplied
//! [`NetworkHandle`] (see `Vm::install_network_handle`) and returns
//! a Promise that settles when the broker reply lands on a
//! subsequent [`super::super::Vm::tick_network`] call.
//!
//! ## Async lifecycle (M4-12 PR5-async-fetch)
//!
//! 1. `native_fetch` parses arguments, builds an
//!    [`elidex_net::Request`], and calls
//!    [`elidex_net::broker::NetworkHandle::fetch_async`] which
//!    returns a [`FetchId`] immediately.
//! 2. The Promise is created Pending and stored in
//!    [`super::super::VmInner::pending_fetches`] keyed by `FetchId`.
//!    If `init.signal` is set, the fetch_id is also pushed to
//!    [`super::super::VmInner::fetch_abort_observers`]`[signal_id]`
//!    and a reverse entry written to
//!    [`super::super::VmInner::fetch_signal_back_refs`] for O(1)
//!    prune on broker reply.
//! 3. The shell event loop later calls `vm.tick_network()`, which
//!    drains [`elidex_net::broker::NetworkHandle::drain_events`].
//!    For each `FetchResponse(id, result)`, the matching entry is
//!    removed from `pending_fetches`; the Promise is fulfilled with
//!    a fresh `Response` (success path) or rejected with a
//!    `TypeError("Failed to fetch: ...")` (broker error / abort).
//! 4. Mid-flight `controller.abort()` settles the Promise
//!    synchronously via [`super::abort::abort_signal`] (see that
//!    module for the fan-out).  The eventual broker reply for
//!    that fetch is silently dropped because its `pending_fetches`
//!    entry was already removed.
//!
//! ## Phase 2 scope (preserved)
//!
//! - Input as URL string or as a [`Request`] instance.  The VM's
//!   existing `Request` constructor handles the canonicalisation
//!   work; `fetch()` calls the same helpers (`parse_url`,
//!   `extract_body_bytes`) from `request_response.rs` so the
//!   behaviour matches byte-for-byte.
//! - `init.method` / `init.headers` / `init.body` / `init.signal`
//!   parsed in the obvious way.  `signal` is brand-checked and
//!   pre-flight-aborted.  `mode` / `credentials` / `cache` /
//!   `redirect` are accepted silently — full enforcement lands
//!   with subsequent stages of the PR5 series.
//! - Errors map per WHATWG §5.2: network failures / missing
//!   handle / bad URL / bad body all reject with **`TypeError`**
//!   (not `DOMException`).  Spec-prescribed text is
//!   `"Failed to fetch"`; the broker's error message is appended
//!   for diagnostics.
//! - Response is converted via the VM's existing Response
//!   scaffolding: new `ObjectKind::Response`, companion `Headers`
//!   with `Immutable` guard, body bytes in the shared
//!   `body_data` map.  `response_type` is `Basic` for successful
//!   responses (CORS classification lands with PR5-cors).

#![cfg(feature = "engine")]

use std::sync::Arc;

use bytes::Bytes;
use url::Url;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::super::VmInner;
use super::blob::reject_promise_sync;
use super::headers::HeadersGuard;
use super::request_response::{
    extract_body_bytes, parse_request_cache, parse_request_credentials, parse_request_mode,
    parse_request_redirect, parse_url, RedirectMode, RequestCache, RequestCredentials, RequestMode,
    ResponseState, ResponseType,
};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install the `fetch` global.  Runs during `register_globals()`
    /// after `register_response_global` (so `response_prototype` is
    /// populated when the first fetch response is constructed).
    pub(in crate::vm) fn register_fetch_global(&mut self) {
        let name = "fetch";
        let fn_id = self.create_native_function(name, native_fetch);
        let name_sid = self.strings.intern(name);
        self.globals.insert(name_sid, JsValue::Object(fn_id));
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// `fetch(input, init?)` (WHATWG Fetch §5.1).
///
/// Post-binding failures (URL parse, header validation, network
/// error, pre-flight abort, invalid `signal`) all reject the
/// returned Promise rather than throwing synchronously, matching
/// the WHATWG Fetch contract that `fetch()` returns a Promise for
/// every well-formed call.  The exceptions are WebIDL
/// binding-level checks that run *before* the method body and
/// must therefore throw synchronously (verified on Chrome /
/// Firefox / Safari):
///
/// - No arguments at all → "not enough arguments" (R19.1).
/// - `init` is a non-object / non-undefined / non-null — WebIDL
///   dictionary type conversion rejects the value (R20.1).
fn native_fetch(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL binding: missing required `input` → synchronous
    // TypeError, not a Promise rejection.  Must run *before*
    // `create_promise` so callers that never handed any argument
    // see the same shape (`try { fetch() } catch (e) { ... }`)
    // as browsers.
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to execute 'fetch': 1 argument required, but only 0 present.",
        ));
    }

    // WebIDL `RequestInit` is a dictionary argument.  Conversion
    // of a non-object / non-undefined / non-null value to a
    // dictionary fails at the binding layer, producing a
    // synchronous TypeError (same shape as `new Request(..., 42)`
    // and `new Response(..., 42)` — both already throw sync).
    // Must run before `create_promise` so the observable shape
    // matches browsers: `try { fetch(url, 42) } catch (e) { ... }`
    // catches here rather than `.catch(e => ...)`-ing a rejected
    // Promise (R20.1).
    let init_raw = args.get(1).copied().unwrap_or(JsValue::Undefined);
    if !matches!(
        init_raw,
        JsValue::Undefined | JsValue::Null | JsValue::Object(_)
    ) {
        return Err(VmError::type_error(
            "Failed to execute 'fetch': init must be an object",
        ));
    }

    let promise = super::super::natives_promise::create_promise(ctx.vm);

    // Root `promise` across every subsequent allocation.
    // `alloc_object` contract requires callers to root any
    // `ObjectId` reachable only through a Rust local whenever a
    // later alloc could trigger GC (see `vm/mod.rs::alloc_object`
    // and `vm/temp_root.rs`'s contract docs).  The guard below pushes the
    // Promise onto the VM stack; `temp_holder` + shadowed `ctx`
    // reborrow the guard so the rest of the function reads and
    // writes vm state through the rooted path without touching
    // the original outer `ctx` (whose `&mut vm` is borrowed by
    // the guard and thus frozen until the guard drops).
    //
    // Current runtime has `gc_enabled = false` inside every
    // native call, so the racy alloc-during-GC path this guards
    // against is unreachable today — but matching the invariant
    // elsewhere in the VM (`wrap_in_array_iterator`, event
    // constructors) keeps the codebase uniform and protects
    // against future refactors that relax the gate.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut temp_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut temp_holder;

    // Parse `init.signal` before building the Request so a bogus
    // `signal` value (non-AbortSignal primitive or DOM object)
    // rejects without first running the more expensive URL /
    // headers / body parse.  WHATWG Fetch §5.4 Request
    // constructor step 29 requires the brand check.  `init_raw`
    // above is already normalised to `Undefined`/`Null`/`Object(_)`
    // by the R20.1 binding-level guard — `extract_signal_from_init`
    // only needs to handle those three shapes.
    let signal = match extract_signal_from_init(ctx, init_raw) {
        Ok(sid) => sid,
        Err(err) => {
            let reason = ctx.vm.vm_error_to_thrown(&err);
            reject_promise_sync(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    // Pre-flight abort: WHATWG Fetch §5.1 main-fetch step 3.
    // Check *before* building the request so an already-aborted
    // signal short-circuits the whole pipeline.
    if let Some(signal_id) = signal {
        if let Some(reason) = pre_flight_abort_reason(ctx, signal_id) {
            reject_promise_sync(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    }

    // Build the broker-level Request.  Any validation failure
    // settles the Promise directly — no synchronous throw.
    let (request, cors_meta) = match build_net_request(ctx, args) {
        Ok(pair) => pair,
        Err(err) => {
            let reason = ctx.vm.vm_error_to_thrown(&err);
            reject_promise_sync(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    // No handle installed → reject immediately.  Matches
    // `NetworkHandle::disconnected()` semantics for callers that
    // never wired up a broker.
    let Some(handle) = ctx.vm.network_handle.clone() else {
        let err = VmError::type_error("Failed to fetch: no NetworkHandle installed on this VM");
        let reason = ctx.vm.vm_error_to_thrown(&err);
        reject_promise_sync(ctx.vm, promise, reason);
        return Ok(JsValue::Object(promise));
    };

    // Async broker dispatch.  Returns a `FetchId` immediately; the
    // reply lands on a future `vm.tick_network()` invocation.  The
    // pending Promise is registered in `pending_fetches` so the
    // tick handler can find it; if `signal` was supplied, the
    // fetch_id is also added to the abort fan-out so a
    // `controller.abort()` can route a CancelFetch to the broker
    // and reject the Promise synchronously.
    let fetch_id = handle.fetch_async(request);
    ctx.vm.pending_fetches.insert(fetch_id, promise);
    ctx.vm.pending_fetch_cors.insert(fetch_id, cors_meta);
    if let Some(signal_id) = signal {
        ctx.vm
            .fetch_abort_observers
            .entry(signal_id)
            .or_default()
            .push(fetch_id);
        ctx.vm.fetch_signal_back_refs.insert(fetch_id, signal_id);
    }

    Ok(JsValue::Object(promise))
}

// ---------------------------------------------------------------------------
// Signal extraction + pre-flight abort (WHATWG Fetch §5.1 / §5.4)
// ---------------------------------------------------------------------------

/// Read `init.signal` and validate its brand.  Returns:
/// - `Ok(None)` when `init` is `undefined` / `null`, when `init`
///   is an object without a `signal` own/inherited property, or
///   when the property value is `undefined` / `null` (WHATWG
///   Fetch §5.4 step 29: `null` is the explicit "no signal"
///   sentinel).
/// - `Ok(Some(id))` for a genuine `AbortSignal` instance (brand
///   checked via `ObjectKind::AbortSignal`).
/// - `Err(TypeError)` for any other non-null value, matching
///   WHATWG WebIDL §3.2.1 interface-type conversion.
///
/// Runs before `build_net_request` so a bad signal rejects early
/// without paying for URL / headers parsing.
fn extract_signal_from_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<Option<ObjectId>, VmError> {
    let opts_id = match init {
        JsValue::Undefined | JsValue::Null => return Ok(None),
        JsValue::Object(id) => id,
        _ => {
            // Non-object init is already rejected in
            // `parse_init_for_fetch` — this helper is called
            // earlier, so treat the same way: reject with the
            // same spec wording.
            return Err(VmError::type_error(
                "Failed to execute 'fetch': init must be an object",
            ));
        }
    };
    let signal_key = PropertyKey::String(ctx.vm.well_known.signal);
    let signal_val = ctx.get_property_value(opts_id, signal_key)?;
    match signal_val {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::Object(sid) if matches!(ctx.vm.get_object(sid).kind, ObjectKind::AbortSignal) => {
            Ok(Some(sid))
        }
        _ => Err(VmError::type_error(
            "Failed to execute 'fetch': member signal is not of type AbortSignal.",
        )),
    }
}

/// Return `Some(reason)` if `signal.aborted === true`, else
/// `None`.  The reason is materialised by `abort_signal()` at the
/// time `controller.abort()` ran, so reading `state.reason`
/// surfaces the already-constructed `DOMException("AbortError")`
/// (or the user-supplied value) without re-allocating.
fn pre_flight_abort_reason(ctx: &NativeContext<'_>, signal_id: ObjectId) -> Option<JsValue> {
    let state = ctx.vm.abort_signal_states.get(&signal_id)?;
    if state.aborted {
        Some(state.reason)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Request construction
// ---------------------------------------------------------------------------

/// Build an [`elidex_net::Request`] from `fetch()`'s arguments.
///
/// Two input shapes:
/// - **Request instance** — start with `method` / `url` / `headers`
///   / `body` from the Request's VM state; any member present in
///   `init` overrides the corresponding field (WHATWG Fetch §5.1
///   step 12, §5.3 Request ctor).
/// - **URL string** — parse against `navigation.current_url`;
///   `init.method` / `init.headers` / `init.body` supply the
///   remaining fields, defaulting to `GET` / empty / empty.
///
/// In both cases `init` is parsed via [`parse_init_overrides`],
/// which returns `None` for each field that the caller's `init`
/// did not explicitly set — `None` preserves the base, `Some`
/// replaces it.
///
/// Returns the broker `Request` plus a [`FetchCorsMeta`]
/// snapshot so the settlement step can run the CORS classifier
/// without re-deriving any of these values.
fn build_net_request(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<(elidex_net::Request, super::cors::FetchCorsMeta), VmError> {
    // `native_fetch` rejects the empty-args case with a synchronous
    // `VmError::type_error` before calling us (R19.1 — WebIDL
    // binding "not enough arguments").  An empty slice here would
    // mean a future caller bypassed that gate; prefer a clear
    // panic in that hypothetical over silent index-out-of-bounds.
    debug_assert!(
        !args.is_empty(),
        "build_net_request called with empty args — native_fetch must reject earlier",
    );
    let input = args[0];
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let overrides = parse_init_overrides(ctx, init)?;

    // Case 1: input is a Request instance — start with its state.
    if let JsValue::Object(obj_id) = input {
        if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Request) {
            let (mut method, url, mut headers, base_body, base_state) =
                request_base_from_vm(ctx, obj_id)?;
            if let Some(m) = overrides.method {
                method = m;
            }
            if let Some(h) = overrides.headers {
                headers = h;
            }
            // Tri-state body resolution (R25.3): `None` preserves
            // the source Request's body; `Some(None)` clears it;
            // `Some(Some(b))` replaces.
            let final_body: Option<Bytes> = match overrides.body {
                None => base_body,
                Some(None) => None,
                Some(Some(b)) => Some(b),
            };
            reject_get_head_with_body(&method, final_body.is_some())?;
            apply_default_content_type(&mut headers, overrides.body_ct_default.as_deref());
            let mode = overrides.mode.unwrap_or(base_state.mode);
            let credentials = overrides.credentials.unwrap_or(base_state.credentials);
            let redirect = overrides.redirect.unwrap_or(base_state.redirect);
            let cache = overrides.cache.unwrap_or(base_state.cache);
            apply_cache_mode_headers(&mut headers, cache);
            reject_same_origin_cross_origin(&ctx.vm.navigation.current_url, &url, mode)?;
            let origin = origin_for_request(&ctx.vm.navigation.current_url, &url);
            let cors_meta = super::cors::FetchCorsMeta {
                request_url: url.clone(),
                request_origin: origin.clone(),
                request_mode: mode,
                redirect_mode: redirect,
            };
            let mut request = elidex_net::Request {
                method,
                url,
                headers,
                body: final_body.unwrap_or_else(Bytes::new),
                origin,
                redirect,
                credentials,
            };
            attach_default_origin(&ctx.vm.navigation.current_url, &mut request);
            attach_default_referer(&ctx.vm.navigation.current_url, &mut request);
            return Ok((request, cors_meta));
        }
    }

    // Case 2: input is a URL string (or ToString-coerced).
    let url_sid = super::super::coerce::to_string(ctx.vm, input)?;
    let raw_url_owned = ctx.vm.strings.get_utf8(url_sid);
    let url = parse_url(ctx.vm, &raw_url_owned).map_err(|_| {
        VmError::type_error(format!(
            "Failed to execute 'fetch': Invalid URL '{raw_url_owned}'"
        ))
    })?;
    let method = overrides.method.unwrap_or_else(|| "GET".to_string());
    // URL-input path has no base body; the tri-state's outer
    // `Some(None)` / `None` both yield "no body".
    let final_body: Option<Bytes> = match overrides.body {
        None | Some(None) => None,
        Some(Some(b)) => Some(b),
    };
    reject_get_head_with_body(&method, final_body.is_some())?;
    let mut headers = overrides.headers.unwrap_or_default();
    apply_default_content_type(&mut headers, overrides.body_ct_default.as_deref());
    // URL-input path: spec defaults for the four enums unless
    // `init.*` overrides them.
    let mode = overrides.mode.unwrap_or(RequestMode::Cors);
    let credentials = overrides
        .credentials
        .unwrap_or(RequestCredentials::SameOrigin);
    let redirect = overrides.redirect.unwrap_or(RedirectMode::Follow);
    let cache = overrides.cache.unwrap_or(RequestCache::Default);
    apply_cache_mode_headers(&mut headers, cache);
    reject_same_origin_cross_origin(&ctx.vm.navigation.current_url, &url, mode)?;
    let origin = origin_for_request(&ctx.vm.navigation.current_url, &url);
    let cors_meta = super::cors::FetchCorsMeta {
        request_url: url.clone(),
        request_origin: origin.clone(),
        request_mode: mode,
        redirect_mode: redirect,
    };
    let mut request = elidex_net::Request {
        method,
        url,
        headers,
        body: final_body.unwrap_or_else(Bytes::new),
        origin,
        redirect,
        credentials,
    };
    attach_default_origin(&ctx.vm.navigation.current_url, &mut request);
    attach_default_referer(&ctx.vm.navigation.current_url, &mut request);
    Ok((request, cors_meta))
}

/// Reject a cross-origin fetch when `mode = "same-origin"`
/// (WHATWG Fetch §5.1 main-fetch step "If request's mode is
/// 'same-origin' and request's origin is not same origin with
/// request's URL, then return a network error.").  Returns
/// `Ok(())` when the request is same-origin or when `mode` is
/// not `SameOrigin`.  The synchronous `TypeError` is funnelled
/// through the caller's `reject_promise_sync` so observable
/// shape is a rejected Promise.
fn reject_same_origin_cross_origin(
    source: &Url,
    target: &Url,
    mode: RequestMode,
) -> Result<(), VmError> {
    if mode != RequestMode::SameOrigin {
        return Ok(());
    }
    if source.origin() == target.origin() {
        return Ok(());
    }
    Err(VmError::type_error(
        "Failed to fetch: cross-origin request blocked by mode='same-origin'",
    ))
}

/// Pick the origin to thread through to the broker as
/// `request.origin`.  Used by the cookie-attach gate (WHATWG
/// Fetch §3.1.7) and by the Stage-4 response_type CORS
/// classifier.  Returns the source's [`url::Origin`] when the
/// document is on an HTTP/HTTPS scheme (script-initiated
/// fetches always have a tuple origin); `None` for `about:blank`
/// / `data:` / etc. initiators with opaque origins where no
/// meaningful tuple-origin exists — broker `SameOrigin`
/// credentials gating treats `None` as "always-attach" (matches
/// pre-PR top-level navigation behaviour).
///
/// Returning [`url::Origin`] (rather than a full URL) ensures
/// the broker never sees the initiator's path / query /
/// fragment — Copilot R1 finding (PR #133): a `Url`-shaped
/// field with origin-only semantics is a misuse trap because
/// every consumer would have to remember to call `.origin()`
/// before comparing.
fn origin_for_request(source: &Url, _target: &Url) -> Option<url::Origin> {
    if matches!(source.scheme(), "http" | "https") {
        Some(source.origin())
    } else {
        None
    }
}

/// Inject the spec-prescribed `Cache-Control` / `Pragma`
/// headers based on `init.cache` (WHATWG Fetch §5.3 step 30
/// onward).  Existing user-set entries are left untouched —
/// per spec the cache-mode injection only fires when the same
/// header is not already present, mirroring the
/// `Content-Type` default path.
///
/// - `Default`: no-op.
/// - `NoStore`: append `Cache-Control: no-store`.
/// - `Reload`: append `Cache-Control: no-cache` + `Pragma:
///   no-cache` (matches Chrome / Firefox behaviour for
///   `cache: 'reload'`).
/// - `NoCache`: append `Cache-Control: max-age=0` (forces
///   server validation).
/// - `ForceCache` / `OnlyIfCached`: documented gap.  These
///   modes require an HTTP cache layer that isn't yet
///   implemented in elidex-net; the broker treats them as
///   `Default`.  Future work: wire to a dedicated
///   `PR-http-cache` slot once a cache backend lands.
fn apply_cache_mode_headers(headers: &mut Vec<(String, String)>, cache: RequestCache) {
    let already_set = |needle: &str, hs: &[(String, String)]| {
        hs.iter().any(|(name, _)| name.eq_ignore_ascii_case(needle))
    };
    match cache {
        RequestCache::Default | RequestCache::ForceCache | RequestCache::OnlyIfCached => {}
        RequestCache::NoStore => {
            if !already_set("cache-control", headers) {
                headers.push(("Cache-Control".to_string(), "no-store".to_string()));
            }
        }
        RequestCache::Reload => {
            if !already_set("cache-control", headers) {
                headers.push(("Cache-Control".to_string(), "no-cache".to_string()));
            }
            if !already_set("pragma", headers) {
                headers.push(("Pragma".to_string(), "no-cache".to_string()));
            }
        }
        RequestCache::NoCache => {
            if !already_set("cache-control", headers) {
                headers.push(("Cache-Control".to_string(), "max-age=0".to_string()));
            }
        }
    }
}

/// Splice `Content-Type: <ct>` into a broker-bound headers list
/// when the caller did not already provide one (case-insensitive
/// match — broker headers retain their original casing, but
/// duplicate `Content-Type` entries violate WHATWG §5 "extract a
/// body" step 2).  No-op when `ct` is `None`.
fn apply_default_content_type(headers: &mut Vec<(String, String)>, ct: Option<&str>) {
    let Some(ct) = ct else {
        return;
    };
    let already_set = headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"));
    if !already_set {
        headers.push(("Content-Type".to_string(), ct.to_string()));
    }
}

/// Attach the `Origin` header for cross-origin `fetch()` requests
/// (WHATWG Fetch §3.2 / §5.4).  Always set on cross-origin
/// HTTP(S) fetches regardless of `init.mode` — the header itself
/// is informational; the response-side gate that rejects opaque
/// CORS responses without `Access-Control-Allow-Origin` is the
/// policy enforcement point and lives with PR5-cors.  Non-fetch
/// paths (navigation, WebSocket, EventSource) attach their own
/// `Origin` upstream and never reach this helper, which returns
/// early for any non-HTTP/S source or target.
///
/// Same-origin requests do not attach `Origin` — the header is
/// reserved for cross-origin disclosure per browser convention.
/// In practice the early-return on a pre-existing `Origin` entry
/// is unreachable for script-initiated fetches because
/// [`super::headers::is_forbidden_request_header`] silently drops
/// user-set `Origin` at both the Request-guard `Headers` and the
/// URL-input init.headers snapshot step (WHATWG Fetch §4.6); the
/// guard remains as a defensive belt-and-braces in case a future
/// internal caller pre-populates `request.headers` before reaching
/// `build_net_request`.
fn attach_default_origin(source: &Url, request: &mut elidex_net::Request) {
    const ORIGIN: &str = "Origin";
    if request
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(ORIGIN))
    {
        return;
    }
    if !matches!(source.scheme(), "http" | "https") {
        return;
    }
    if !matches!(request.url.scheme(), "http" | "https") {
        return;
    }
    let source_origin = source.origin();
    if source_origin == request.url.origin() {
        return;
    }
    if source_origin.is_tuple() {
        request
            .headers
            .push((ORIGIN.to_string(), source_origin.ascii_serialization()));
    }
}

/// Attach the `Referer` header that WHATWG Fetch's default referrer
/// policy (`strict-origin-when-cross-origin`) would produce.
/// Script-initiated fetches no longer reach the early-return branch
/// because §4.6 forbidden-header enforcement strips user-set
/// `Referer` upstream — see [`attach_default_origin`] for the
/// matching analysis.  The pre-existing-entry guard is retained as
/// a belt-and-braces for future internal callers that pre-populate
/// `request.headers`.
///
/// Policy `strict-origin-when-cross-origin` (Fetch §3.2.5):
///
/// - Same-origin → full URL with fragment + userinfo stripped.
/// - Cross-origin without TLS downgrade → origin only.
/// - HTTPS → HTTP (TLS downgrade) → no header.
/// - Non-HTTP/HTTPS source or target → no header.
fn attach_default_referer(source: &Url, request: &mut elidex_net::Request) {
    const REFERER: &str = "Referer";
    let already_set = request
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(REFERER));
    if already_set {
        return;
    }
    if let Some(value) = compute_default_referer(source, &request.url) {
        request.headers.push((REFERER.to_string(), value));
    }
}

fn compute_default_referer(source: &Url, target: &Url) -> Option<String> {
    if !matches!(source.scheme(), "http" | "https") {
        return None;
    }
    if !matches!(target.scheme(), "http" | "https") {
        return None;
    }
    if source.scheme() == "https" && target.scheme() == "http" {
        // TLS downgrade — strict-origin-when-cross-origin strips.
        return None;
    }
    if source.origin() == target.origin() {
        // Same-origin: strip fragment + userinfo and serialise the
        // full URL.  The clones cannot fail because the `set_*`
        // calls all clear data without re-parsing.
        let mut clean = source.clone();
        clean.set_fragment(None);
        let _ = clean.set_username("");
        let _ = clean.set_password(None);
        Some(clean.to_string())
    } else {
        // Cross-origin: serialise the source's origin only.  Opaque
        // origins (would be unreachable here because we filtered
        // schemes above, but defensive in case of future
        // refactoring) cannot produce a tuple serialisation.
        let origin = source.origin();
        origin.is_tuple().then(|| origin.ascii_serialization())
    }
}

/// WHATWG Fetch §5.3 step 40: if method is `GET` / `HEAD` and the
/// final Request has a body, throw `TypeError`.  Shared between
/// the Request-input and URL-input `build_net_request` paths
/// (R25.1 / R25.2).  `fetch()` routes the resulting error through
/// `reject_promise_sync`, so the observable shape is a rejected
/// Promise — matching how `new Request(url, {method:'GET', body:
/// 'x'})` throws synchronously while `fetch(url, {method:'GET',
/// body:'x'})` rejects.
fn reject_get_head_with_body(method: &str, has_body: bool) -> Result<(), VmError> {
    if has_body && (method == "GET" || method == "HEAD") {
        return Err(VmError::type_error(format!(
            "Failed to execute 'fetch': Request with {method} method cannot have body"
        )));
    }
    Ok(())
}

/// Extract the `(method, url, headers, body, base_state)`
/// tuple from a VM `Request` instance.  Used as the base for the
/// Request-input path of `fetch()` before `init` overrides are
/// layered on.  Returns `body: Option<Bytes>` where `None` means
/// "the source Request has no body at all" (key absent in
/// `body_data`) and `Some(bytes)` means "has body with these
/// bytes" (possibly empty).  The presence distinction matters
/// for the WHATWG Fetch §5.3 step 40 GET/HEAD-without-body check
/// (R25.1): a cloned Request whose source has no body may switch
/// to `GET`/`HEAD` freely, but one whose source carries a body
/// cannot.  `base_state` carries the source's `mode` /
/// `credentials` / `redirect` enums so `init.*` overrides on
/// `fetch(req, init)` can replace them per WHATWG Fetch §5.1
/// step 12.
fn request_base_from_vm(
    ctx: &NativeContext<'_>,
    obj_id: ObjectId,
) -> Result<(String, Url, Vec<(String, String)>, Option<Bytes>, BaseState), VmError> {
    let state = ctx
        .vm
        .request_states
        .get(&obj_id)
        .expect("Request without request_states entry");

    let method = ctx.vm.strings.get_utf8(state.method_sid);
    let url_str = ctx.vm.strings.get_utf8(state.url_sid);
    let url = Url::parse(&url_str).map_err(|_| {
        VmError::type_error(format!(
            "Failed to execute 'fetch': Request URL '{url_str}' did not re-parse"
        ))
    })?;
    let base_state = BaseState {
        mode: state.mode,
        credentials: state.credentials,
        redirect: state.redirect,
        cache: state.cache,
    };

    let headers: Vec<(String, String)> = ctx
        .vm
        .headers_states
        .get(&state.headers_id)
        .map(|hs| {
            hs.list
                .iter()
                .map(|(n, v)| (ctx.vm.strings.get_utf8(*n), ctx.vm.strings.get_utf8(*v)))
                .collect()
        })
        .unwrap_or_default();

    // Snapshot the body bytes into an `Arc<[u8]>` so they can be
    // handed to `Bytes::from_owner` (whose owner bound is
    // `Send + Sync + 'static`).  The snapshot semantics match the
    // spec — once `fetch()` extracts the request body, subsequent
    // VM-side mutations through the same `body_data` entry must
    // not propagate to the in-flight HTTP request.  The previous
    // `Arc::clone(arc)` zero-copy handoff happened to deliver the
    // same observable behaviour because the engine cloned-and-
    // reinstalled an `Arc` on every TypedArray write; with owned
    // `Vec<u8>` storage the snapshot must be explicit.
    let body = ctx.vm.body_data.get(&obj_id).map(|bytes| {
        let arc: Arc<[u8]> = Arc::from(bytes.as_slice());
        Bytes::from_owner(arc)
    });

    Ok((method, url, headers, body, base_state))
}

/// `mode` / `credentials` / `redirect` / `cache` carried over
/// from a source `Request` into [`build_net_request`]'s
/// Request-input path.  Returned alongside the URL/method/
/// headers/body tuple from [`request_base_from_vm`] so the
/// caller can layer `init.*` overrides on top.
struct BaseState {
    mode: RequestMode,
    credentials: RequestCredentials,
    redirect: RedirectMode,
    cache: RequestCache,
}

/// Returned by [`parse_init_overrides`].
///
/// Method and headers are plain `Option<_>` — `None` means absent,
/// `Some(_)` means explicit.
///
/// The body slot is **tri-state** (R25.3):
/// - outer `None` — `init.body` was absent; preserve the base
///   Request's body (for the Request-input path) or use `None`
///   (URL-input path).
/// - `Some(None)` — `init.body` was explicitly `null`; clear any
///   base body.  The final Request has no body.
/// - `Some(Some(b))` — `init.body` was an explicit value; replace
///   with `b`.  Any non-`null`, non-`undefined` input lands here,
///   including the empty string which still counts as "has a
///   body" for the GET/HEAD check in `build_net_request`.
///
/// `body_ct_default` is the optional default `Content-Type`
/// derived from the body type (WHATWG Fetch §5 "extract a body"
/// step 4 / §5.3 step 38).  `None` when no default applies (e.g.
/// ArrayBuffer / clone path) or when `init.body` was not
/// present.  The fetch-call path splices this header before
/// relaying to the broker so a `fetch(..., {body: new FormData()})`
/// request goes out with the boundary-bearing
/// `multipart/form-data` Content-Type.
///
/// The four enum overrides — `mode` / `credentials` / `redirect`
/// / `cache` — are `None` when `init` did not set the member,
/// allowing the Request-input path to preserve the source's
/// values.  The URL-input path falls back to spec defaults
/// (`Cors` / `SameOrigin` / `Follow` / `Default`).  The Stage 1
/// landing in PR5-cors only validates and round-trips the values
/// through Request state; broker-side enforcement (same-origin
/// reject, redirect mode, credentials gating, cache header
/// injection) lands with Stages 2-5.
#[allow(dead_code)]
struct InitOverrides {
    method: Option<String>,
    headers: Option<Vec<(String, String)>>,
    body: Option<Option<Bytes>>,
    body_ct_default: Option<String>,
    mode: Option<RequestMode>,
    credentials: Option<RequestCredentials>,
    redirect: Option<RedirectMode>,
    cache: Option<RequestCache>,
}

/// Parse the `init` dict.  Every field is `Option<_>`; a present
/// value means `init` explicitly set it.  `undefined` (including
/// the field being absent entirely) always maps to `None`.
/// `null` handling is **field-specific** — see the per-field
/// "Null vs undefined" block below for the source-of-truth
/// semantics.  In short: both `headers: null` and `body: null`
/// explicitly override to the empty form — empty header list /
/// empty body bytes — matching `new Request(req, init)` and
/// browser Fetch (WebIDL nullable members).  `undefined` or an
/// absent field preserves the base.
fn parse_init_overrides(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<InitOverrides, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok(InitOverrides {
            method: None,
            headers: None,
            body: None,
            body_ct_default: None,
            mode: None,
            credentials: None,
            redirect: None,
            cache: None,
        }),
        JsValue::Object(opts_id) => {
            let method_sid_key = PropertyKey::String(ctx.vm.well_known.method);
            let headers_key = PropertyKey::String(ctx.vm.well_known.headers);
            let body_key = PropertyKey::String(ctx.vm.well_known.body);
            let mode_key = PropertyKey::String(ctx.vm.well_known.mode);
            let credentials_key = PropertyKey::String(ctx.vm.well_known.credentials);
            let redirect_key = PropertyKey::String(ctx.vm.well_known.redirect);
            let cache_key = PropertyKey::String(ctx.vm.well_known.cache);

            let method_val = ctx.get_property_value(opts_id, method_sid_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;
            let body_val = ctx.get_property_value(opts_id, body_key)?;
            let mode_val = ctx.get_property_value(opts_id, mode_key)?;
            let credentials_val = ctx.get_property_value(opts_id, credentials_key)?;
            let redirect_val = ctx.get_property_value(opts_id, redirect_key)?;
            let cache_val = ctx.get_property_value(opts_id, cache_key)?;

            let mode_override = parse_request_mode(ctx, mode_val, "Failed to execute 'fetch'")?;
            let credentials_override =
                parse_request_credentials(ctx, credentials_val, "Failed to execute 'fetch'")?;
            let redirect_override =
                parse_request_redirect(ctx, redirect_val, "Failed to execute 'fetch'")?;
            let cache_override = parse_request_cache(ctx, cache_val, "Failed to execute 'fetch'")?;

            // Method — shared forbidden-method filter with
            // `Request`'s ctor.
            let method_override = if matches!(method_val, JsValue::Undefined) {
                None
            } else {
                let sid = super::super::coerce::to_string(ctx.vm, method_val)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                Some(super::request_response::validate_http_method(
                    &raw,
                    "Failed to execute 'fetch'",
                )?)
            };

            // Headers — reuse the shared `new Headers(init)`
            // algorithm (lowercasing / validation /
            // Array-of-pairs / Record paths converge on the same
            // code) via `parse_headers_init_entries`, which
            // returns the parsed entries directly as
            // `Vec<(StringId, StringId)>` without allocating a
            // throwaway `Headers` JS object (R8.2).
            //
            // **Null vs undefined**: `undefined` (field absent)
            // returns `None` → base headers preserved.  `null`
            // returns `Some(empty)` → override to empty —
            // matching `new Request(req, {headers: null})` which
            // also clears the header list (R7.1).  Keeping the
            // two surfaces in sync is what user code expects
            // from the browser Fetch implementations.
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                JsValue::Null => Some(Vec::new()),
                _ => {
                    let entries = super::headers::parse_headers_init_entries(
                        ctx,
                        headers_val,
                        "Failed to execute 'fetch'",
                    )?;
                    // WHATWG Fetch §4.6 forbidden-request-header
                    // filter applies to the URL-input path too.
                    // The Request-input path filters via the
                    // companion Headers' `Request` guard during
                    // ctor; this path has no companion, so the
                    // filter happens here at snapshot time.
                    let snapshot: Vec<(String, String)> = entries
                        .into_iter()
                        .filter_map(|(n, v)| {
                            let name = ctx.vm.strings.get_utf8(n);
                            if super::headers::is_forbidden_request_header(&name) {
                                None
                            } else {
                                Some((name, ctx.vm.strings.get_utf8(v)))
                            }
                        })
                        .collect();
                    Some(snapshot)
                }
            };

            // Body — zero-copy handoff via `Bytes::from_owner`.
            // **Null vs undefined** (WHATWG Fetch §5.4 / WebIDL
            // nullable body): `undefined` means "field absent →
            // preserve base body"; `null` means "explicit override
            // to empty body" (matches `new Request(req, {body:
            // null})` and Chromium / Firefox Fetch), mirroring the
            // headers null-override semantics fixed in R7.1.
            // Tri-state (R25.3) — see [`InitOverrides`] doc.
            // - `undefined`: preserve base / default.
            // - `null`: explicit clear.  Must be distinguishable
            //   from an empty-but-present body so the GET/HEAD
            //   check in `build_net_request` can fire only when a
            //   body is actually present.
            // - anything else: explicit replace.
            let (body_override, body_ct_default) = match body_val {
                JsValue::Undefined => (None, None),
                JsValue::Null => (Some(None), None),
                _ => match extract_body_bytes(ctx, body_val)? {
                    None => (Some(None), None),
                    Some((bytes, Some(ct_override))) => {
                        let ct = ctx.vm.strings.get_utf8(ct_override);
                        (Some(Some(Bytes::from_owner(bytes))), Some(ct))
                    }
                    Some((bytes, None)) => {
                        let ct_default =
                            super::request_response::content_type_for_body(ctx, body_val)
                                .map(|sid| ctx.vm.strings.get_utf8(sid));
                        (Some(Some(Bytes::from_owner(bytes))), ct_default)
                    }
                },
            };

            Ok(InitOverrides {
                method: method_override,
                headers: headers_override,
                body: body_override,
                body_ct_default,
                mode: mode_override,
                credentials: credentials_override,
                redirect: redirect_override,
                cache: cache_override,
            })
        }
        _ => Err(VmError::type_error(
            "Failed to execute 'fetch': init must be an object",
        )),
    }
}

// ---------------------------------------------------------------------------
// Response construction (broker → VM)
// ---------------------------------------------------------------------------

/// Wrap a broker [`Response`](elidex_net::Response) in a VM
/// `Response` object.  Headers are lowercased name-side (matches
/// `new Response`'s behaviour) and guarded Immutable.  Body bytes
/// land in the shared `body_data` map so `.text()` / `.json()`
/// / `.arrayBuffer()` / `.blob()` work without further copies.
///
/// The [`CorsClassification`] argument selects the Response
/// shape:
/// - `Basic`: full headers, full body, status / url verbatim.
/// - `Cors`: headers filtered to CORS-safelisted +
///   `Access-Control-Expose-Headers` names; body / status / url
///   passed through.
/// - `Opaque` / `OpaqueRedirect` (`opaque_shape: true`): empty
///   headers, body dropped, status forced to 0, url emptied.
///   Spec-mandated to prevent leakage of cross-origin data.
pub(super) fn create_response_from_net(
    vm: &mut VmInner,
    response: elidex_net::Response,
    classification: super::cors::CorsClassification,
) -> ObjectId {
    let proto = vm.response_prototype;
    let inst_id = vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });

    // Root the freshly-allocated Response across the next two
    // allocations (the companion `create_headers` + the per-name
    // / per-value `intern` calls).  Before `response_states`
    // stores `inst_id` near the end of this function, the new
    // Response is reachable only through this Rust local — per
    // `alloc_object`'s contract, any subsequent alloc that
    // triggers GC would reclaim it.  Same defensive invariant
    // as `wrap_in_array_iterator` (R10) and `native_fetch`
    // (R13).  The current runtime runs this site with
    // `gc_enabled = false` (called from inside `native_fetch`),
    // so the hazard is unreachable today; the guard future-
    // proofs it.
    let mut g = vm.push_temp_root(JsValue::Object(inst_id));

    // Companion Headers — allocate mutable, splice, then flip
    // to Immutable (matches `new Response(...)` contract).
    //
    // `headers_id` is also rooted across the header-splice work.
    // `headers_states` is **not** itself a GC root (see
    // `gc::mark_roots` — the entry is reached only via
    // `response_states[inst_id].headers_id`), so until
    // `response_states.insert(...)` links the Headers into the
    // Response, `headers_id` is reachable only through this
    // Rust local.  Route every subsequent allocation through `g2`
    // to keep both `inst_id` and `headers_id` rooted across the
    // `strings.intern` / `body_data.insert` / `response_states
    // .insert` sequence below (R18.2).
    // Apply the CORS classification to the response shape.  An
    // opaque-shape response (Opaque / OpaqueRedirect) discards
    // all headers, body, status, and URL so cross-origin data
    // never leaks into JS.  A Cors-typed response filters
    // headers down to CORS-safelisted +
    // `Access-Control-Expose-Headers` names.  Basic / Default
    // pass through verbatim.
    let opaque_shape = classification.opaque_shape;
    let response_type = classification.response_type;
    let header_pairs: Vec<(String, String)> = if opaque_shape {
        Vec::new()
    } else if matches!(response_type, ResponseType::Cors) {
        super::cors::filter_headers_for_cors_response(response.headers)
    } else {
        response.headers
    };

    let headers_id = g.create_headers(HeadersGuard::None);
    let mut g2 = g.push_temp_root(JsValue::Object(headers_id));
    {
        // Route each broker-delivered header through the shared
        // `validate_and_normalise` helper so the resulting
        // `HeadersState` carries the **same** invariants as a
        // script-constructed `Headers` instance: lowercased
        // name, RFC 7230 token-valid name, CR/LF/NUL-free value,
        // HTTP-whitespace-trimmed value.  Malformed entries
        // (broker-side bug, not user input) are silently
        // skipped — defensive, preserves the invariant even if
        // the network layer later relaxes its own filters.
        for (name, value) in header_pairs {
            let name_sid = g2.strings.intern(&name);
            let value_sid = g2.strings.intern(&value);
            if let Ok((nn, nv)) =
                super::headers::validate_and_normalise(&mut g2, name_sid, value_sid, "response")
            {
                if let Some(state) = g2.headers_states.get_mut(&headers_id) {
                    state.list.push((nn, nv));
                }
            }
        }
        if let Some(state) = g2.headers_states.get_mut(&headers_id) {
            state.guard = HeadersGuard::Immutable;
        }
    }

    // Body bytes.  Skip the map insert for zero-byte responses
    // so `.body_data.contains_key(id)` keeps meaning "this
    // response actually carries bytes".  Opaque-shape responses
    // also skip the insert — body must be `null` (= absent).
    //
    // The HTTP response body is owned by `bytes::Bytes` (its own
    // ref-counted handle); we copy it into a fresh `Vec<u8>` for
    // installation in `body_data`, since that map's storage type
    // is owned `Vec<u8>` so subsequent TypedArray / DataView
    // writes can mutate it in place via `byte_io`.
    if !opaque_shape && !response.body.is_empty() {
        g2.body_data.insert(inst_id, response.body.to_vec());
    }

    // Status / url rewrite for opaque-shape responses (WHATWG
    // Fetch §3.1.4 / §3.1.6): status 0, url empty.  Basic /
    // Cors pass through.
    let final_status = if opaque_shape { 0 } else { response.status };
    let url_sid = if opaque_shape {
        g2.well_known.empty
    } else {
        g2.strings.intern(response.url.as_str())
    };
    let status_text_sid = g2.well_known.empty;
    let redirected = response.url_list.len() > 1;

    g2.response_states.insert(
        inst_id,
        ResponseState {
            status: final_status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type,
            redirected,
        },
    );
    drop(g2);
    // `inst_id` is now referenced from `response_states` (and
    // `headers_id` is referenced from its ResponseState field),
    // so dropping the root is safe.
    drop(g);
    inst_id
}

// `tick_network` / `settle_fetch` / `reject_pending_fetches_with_error`
// implementations live in [`super::fetch_tick`] to keep this file under
// the project's 1000-line convention (Copilot R4.2).  The split has
// no observable effect on call sites; both modules are sibling
// `pub(super)` peers under `host::`.
