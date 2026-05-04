//! [`elidex_net::Request`] construction for `fetch(input, init?)`.
//!
//! Converts a JS-side `(input, init)` pair into the broker-level
//! Request shape, layering `init.*` overrides on top of either a
//! source [`elidex_net::Request`] instance or a URL string.
//! Auto-attaches the WHATWG-mandated `Origin` and `Referer`
//! headers; injects the `init.cache` mode's spec
//! `Cache-Control` / `Pragma` defaults.

use std::sync::Arc;

use bytes::Bytes;
use url::Url;

use super::super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError,
};
use super::super::request_response::{
    extract_body_bytes, parse_request_cache, parse_request_credentials, parse_request_mode,
    parse_request_redirect, parse_url, RedirectMode, RequestCache, RequestCredentials, RequestMode,
};

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
/// Returns the broker `Request` plus a
/// [`super::super::cors::FetchCorsMeta`] snapshot so the
/// settlement step can run the CORS classifier without re-deriving
/// any of these values.
pub(super) fn build_net_request(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<(elidex_net::Request, super::super::cors::FetchCorsMeta), VmError> {
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
            let cors_meta = super::super::cors::FetchCorsMeta {
                request_url: url.clone(),
                request_origin: origin.clone(),
                request_mode: mode,
                redirect_mode: redirect,
            };
            let mut request = elidex_net::Request {
                method,
                url,
                headers,
                body: final_body.unwrap_or_default(),
                origin,
                redirect,
                credentials,
                mode,
            };
            attach_default_origin(&ctx.vm.navigation.current_url, &mut request);
            attach_default_referer(&ctx.vm.navigation.current_url, &mut request);
            return Ok((request, cors_meta));
        }
    }

    // Case 2: input is a URL string (or ToString-coerced).
    let url_sid = super::super::super::coerce::to_string(ctx.vm, input)?;
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
    let cors_meta = super::super::cors::FetchCorsMeta {
        request_url: url.clone(),
        request_origin: origin.clone(),
        request_mode: mode,
        redirect_mode: redirect,
    };
    let mut request = elidex_net::Request {
        method,
        url,
        headers,
        body: final_body.unwrap_or_default(),
        origin,
        redirect,
        credentials,
        mode,
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
/// classifier.  Always returns `Some(source.origin())` for any
/// script-initiated fetch — including opaque-origin initiators
/// like `data:` URLs, which serialize as `null` and never match
/// any tuple origin (so SameOrigin credentials are stripped and
/// CORS classification runs the cross-origin path, matching
/// WHATWG Fetch §3.2.5 + §3.1.7 for opaque origins).
///
/// `Request.origin = None` is reserved for **embedder-driven
/// callers** that bypass the VM-side fetch path (the navigation
/// pipeline's initial document load, favicon prefetch, etc.) —
/// those genuinely have no script-origin context.  That path is
/// untouched by this PR and continues to use the
/// `..Default::default()` shape with `origin: None`.
///
/// Returning [`url::Origin`] (rather than a full URL) ensures
/// the broker never sees the initiator's path / query /
/// fragment — Copilot R1 finding (PR #133): a `Url`-shaped
/// field with origin-only semantics is a misuse trap because
/// every consumer would have to remember to call `.origin()`
/// before comparing.
///
/// Copilot R3 (findings 2+3): the previous "HTTP/HTTPS only,
/// else None" gate caused `data:` / `about:blank` script
/// initiators to short-circuit to `ResponseType::Basic` in the
/// classifier, which is a CORS bypass.  Fixed by threading
/// every script-side fetch's source origin through verbatim.
fn origin_for_request(source: &Url, _target: &Url) -> Option<url::Origin> {
    Some(source.origin())
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
/// early for any non-HTTP/S target.
///
/// **Source scheme**: any scheme is allowed.  Opaque-origin
/// initiators (`data:` / `about:blank` scripts) emit
/// `Origin: null` (the WHATWG-mandated serialisation of an
/// opaque origin per HTML §3.2.1.2) so a CORS-mode cross-origin
/// fetch from such a script can satisfy a server that gates
/// ACAO on the `Origin` header's presence (Copilot R4 finding).
/// Tuple origins emit the standard `scheme://host[:port]` form.
///
/// Same-origin requests do not attach `Origin` — the header is
/// reserved for cross-origin disclosure per browser convention.
/// In practice the early-return on a pre-existing `Origin` entry
/// is unreachable for script-initiated fetches because
/// [`super::super::headers::is_forbidden_request_header`] silently drops
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
    if !matches!(request.url.scheme(), "http" | "https") {
        return;
    }
    let source_origin = source.origin();
    if source_origin == request.url.origin() {
        return;
    }
    // Always emit Origin for cross-origin HTTP(S) targets.
    // `ascii_serialization()` returns `"null"` for opaque
    // origins (HTML §3.2.1.2) — that's the spec-mandated value
    // for opaque-initiator CORS requests.
    request
        .headers
        .push((ORIGIN.to_string(), source_origin.ascii_serialization()));
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
/// (`Cors` / `SameOrigin` / `Follow` / `Default`).
///
/// `build_net_request` enforces these values end-to-end:
/// - `mode = SameOrigin` rejects cross-origin URLs synchronously
///   with `TypeError` (Stage 3).
/// - `credentials` is threaded into the broker `Request` so
///   `should_attach_cookies` gates Cookie attach + storage;
///   classifier uses it to enforce the credentialed-CORS
///   strict ACAO/ACAC rules (Stage 4 + Copilot R3).
/// - `redirect` is threaded so the broker's `follow_redirects`
///   honours Error / Manual modes (Stage 2).
/// - `cache` triggers `apply_cache_mode_headers` to inject the
///   spec `Cache-Control` / `Pragma` headers (Stage 5).
/// - `mode` also drives `response_type` classification at
///   settlement time (Stage 4 — `host/cors.rs`).
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
                let sid = super::super::super::coerce::to_string(ctx.vm, method_val)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                Some(super::super::request_response::validate_http_method(
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
                    let entries = super::super::headers::parse_headers_init_entries(
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
                            if super::super::headers::is_forbidden_request_header(&name) {
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
                            super::super::request_response::content_type_for_body(ctx, body_val)
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
