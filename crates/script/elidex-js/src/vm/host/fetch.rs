//! `fetch(input, init?)` host global (WHATWG Fetch §5.1).
//!
//! Routes a JS-level fetch request through the embedding-supplied
//! [`NetworkHandle`] (see `Vm::install_network_handle`) and returns
//! a Promise that settles synchronously against the blocking
//! broker call.
//!
//! ## Phase 2 scope
//!
//! - Input as URL string or as a [`Request`] instance.  The VM's
//!   existing `Request` constructor handles the canonicalisation
//!   work; `fetch()` calls the same helpers (`parse_url`,
//!   `extract_body_bytes`) from `request_response.rs` so the
//!   behaviour matches byte-for-byte.
//! - `init.method` / `init.headers` / `init.body` / `init.signal`
//!   parsed in the obvious way.  `signal` is brand-checked and
//!   pre-flight-aborted (see the Phase 2 limitation below).
//!   `mode` / `credentials` / `cache` / `redirect` are accepted
//!   silently and ignored until the async fetch refactor threads
//!   them through the broker.
//! - Errors map per WHATWG §5.2: network failures / missing
//!   handle / bad URL / bad body all reject with **`TypeError`**
//!   (not `DOMException`).  Spec-prescribed text is
//!   `"Failed to fetch"`; the broker's error message is appended
//!   for diagnostics.
//! - Response is converted via the VM's existing Response
//!   scaffolding: new `ObjectKind::Response`, companion `Headers`
//!   with `Immutable` guard, body bytes in the shared
//!   `body_data` map.  `response_type` is `Basic` for successful
//!   responses (CORS classification lands with the fetch refactor
//!   that threads through an Origin).
//!
//! ## Phase 2 limitation (intentional)
//!
//! `NetworkHandle::fetch_blocking` blocks the content thread, so
//! the Promise is fulfilled / rejected *before* `fetch()` returns
//! to JS.  User code still observes the expected asynchronous
//! shape (`.then` / `await` schedule a microtask), but `signal`-
//! based mid-flight cancellation cannot fire — there is no JS
//! tick between the broker send and the broker reply.  The only
//! effective `signal` path in Phase 2 is the **pre-flight** check
//! implemented below: if `signal.aborted === true` before the
//! broker call, we reject immediately with `signal.reason`.
//! `VmInner::fetch_abort_observers` holds the wire for the
//! mid-flight path; it stays empty in Phase 2 and will be
//! populated by the PR5-async-fetch refactor.

#![cfg(feature = "engine")]

use std::sync::Arc;

use bytes::Bytes;
use url::Url;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::super::VmInner;
use super::blob::{reject_promise_sync, resolve_promise_sync};
use super::headers::HeadersGuard;
use super::request_response::{extract_body_bytes, parse_url, ResponseState, ResponseType};

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
    let request = match build_net_request(ctx, args) {
        Ok(req) => req,
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

    // Blocking broker call.  `signal` is not registered in
    // `fetch_abort_observers` here: the blocking broker call is
    // synchronous, so no JS listener can fire
    // `controller.abort()` before the reply.  The PR5-async-fetch
    // refactor will insert `(signal, broker-fetch-id)` registration
    // + broker-reply pruning at exactly this site.
    match handle.fetch_blocking(request) {
        Ok(response) => {
            let resp_id = create_response_from_net(ctx.vm, response);
            resolve_promise_sync(ctx.vm, promise, JsValue::Object(resp_id));
        }
        Err(msg) => {
            // Spec §5.2 "Network error" → TypeError, not
            // DOMException.  Preserve the broker's message for
            // diagnostics but wrap in the spec-prescribed wording.
            let err = VmError::type_error(format!("Failed to fetch: {msg}"));
            let reason = ctx.vm.vm_error_to_thrown(&err);
            reject_promise_sync(ctx.vm, promise, reason);
        }
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
fn build_net_request(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<elidex_net::Request, VmError> {
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

    let (method_override, headers_override, body_override) = parse_init_overrides(ctx, init)?;

    // Case 1: input is a Request instance — start with its state.
    if let JsValue::Object(obj_id) = input {
        if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Request) {
            let (mut method, url, mut headers, base_body) = request_base_from_vm(ctx, obj_id)?;
            if let Some(m) = method_override {
                method = m;
            }
            if let Some(h) = headers_override {
                headers = h;
            }
            // Tri-state body resolution (R25.3): `None` preserves
            // the source Request's body; `Some(None)` clears it;
            // `Some(Some(b))` replaces.
            let final_body: Option<Bytes> = match body_override {
                None => base_body,
                Some(None) => None,
                Some(Some(b)) => Some(b),
            };
            reject_get_head_with_body(&method, final_body.is_some())?;
            let mut request = elidex_net::Request {
                method,
                url,
                headers,
                body: final_body.unwrap_or_else(Bytes::new),
            };
            attach_default_referer(&ctx.vm.navigation.current_url, &mut request);
            return Ok(request);
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
    let method = method_override.unwrap_or_else(|| "GET".to_string());
    // URL-input path has no base body; the tri-state's outer
    // `Some(None)` / `None` both yield "no body".
    let final_body: Option<Bytes> = match body_override {
        None | Some(None) => None,
        Some(Some(b)) => Some(b),
    };
    reject_get_head_with_body(&method, final_body.is_some())?;
    let mut request = elidex_net::Request {
        method,
        url,
        headers: headers_override.unwrap_or_default(),
        body: final_body.unwrap_or_else(Bytes::new),
    };
    attach_default_referer(&ctx.vm.navigation.current_url, &mut request);
    Ok(request)
}

/// Attach the `Referer` header that WHATWG Fetch's default referrer
/// policy (`strict-origin-when-cross-origin`) would produce, but only
/// if the caller has not already supplied one.
///
/// Phase 2 is opportunistic: forbidden-header enforcement (which
/// would normally drop a script-supplied `Referer` per WHATWG Fetch
/// §4.6) lives in PR5-async-fetch.  Until that lands we leave a
/// caller-set value alone — that is the worst case for spec
/// strictness but matches existing test expectations and never
/// produces a duplicate header.
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

/// Extract the `(method, url, headers, body)` tuple from a VM
/// `Request` instance.  Used as the base for the Request-input
/// path of `fetch()` before `init` overrides are layered on.
/// Returns `body: Option<Bytes>` where `None` means "the source
/// Request has no body at all" (key absent in `body_data`) and
/// `Some(bytes)` means "has body with these bytes" (possibly
/// empty).  The presence distinction matters for the WHATWG Fetch
/// §5.3 step 40 GET/HEAD-without-body check (R25.1): a cloned
/// Request whose source has no body may switch to `GET`/`HEAD`
/// freely, but one whose source carries a body cannot.
fn request_base_from_vm(
    ctx: &NativeContext<'_>,
    obj_id: ObjectId,
) -> Result<(String, Url, Vec<(String, String)>, Option<Bytes>), VmError> {
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

    // `Bytes::from_owner(Arc::clone(arc))` hands the `Arc<[u8]>`
    // to the `Bytes` instance as its owner, so no byte copy
    // happens — the broker reads directly from the same
    // allocation `body_data` already rooted.  `Option<Bytes>`
    // preserves the "no body" vs "empty body" distinction (see
    // fn doc).
    let body = ctx
        .vm
        .body_data
        .get(&obj_id)
        .map(|arc| Bytes::from_owner(Arc::clone(arc)));

    Ok((method, url, headers, body))
}

/// `(method?, headers?, body?)` returned by [`parse_init_overrides`].
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
type InitOverrides = (
    Option<String>,
    Option<Vec<(String, String)>>,
    Option<Option<Bytes>>,
);

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
        JsValue::Undefined | JsValue::Null => Ok((None, None, None)),
        JsValue::Object(opts_id) => {
            let method_sid_key = PropertyKey::String(ctx.vm.well_known.method);
            let headers_key = PropertyKey::String(ctx.vm.well_known.headers);
            let body_key = PropertyKey::String(ctx.vm.well_known.body);

            let method_val = ctx.get_property_value(opts_id, method_sid_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;
            let body_val = ctx.get_property_value(opts_id, body_key)?;

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
                    let snapshot: Vec<(String, String)> = entries
                        .into_iter()
                        .map(|(n, v)| (ctx.vm.strings.get_utf8(n), ctx.vm.strings.get_utf8(v)))
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
            let body_override = match body_val {
                JsValue::Undefined => None,
                JsValue::Null => Some(None),
                _ => Some(extract_body_bytes(ctx, body_val)?.map(Bytes::from_owner)),
            };

            Ok((method_override, headers_override, body_override))
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
fn create_response_from_net(vm: &mut VmInner, response: elidex_net::Response) -> ObjectId {
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
        for (name, value) in response.headers {
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
    // response actually carries bytes".
    //
    // **Phase 2 cost**: `response.body` is `bytes::Bytes` (which
    // is itself ref-counted) but we copy into `Arc<[u8]>` because
    // `VmInner::body_data` is typed `Arc<[u8]>` to match the
    // broker-independent Request / Blob / ArrayBuffer paths.
    // Switching the side-table type to `Bytes` (or adding a
    // zero-copy shim) is plausible but intrudes on every body-
    // mixin reader in `body_mixin.rs` and every GC sweep site in
    // `gc.rs`; deferred to the PR5-streams tranche which
    // refactors body storage to a stream-compatible wrapper
    // anyway.  The copy is observable only on large fetch
    // responses — for script-sized bodies it's below measurement
    // noise, tracked for the post-PR5a-fetch spec-polish pass.
    if !response.body.is_empty() {
        let bytes: Arc<[u8]> = Arc::from(&response.body[..]);
        g2.body_data.insert(inst_id, bytes);
    }

    let url_sid = g2.strings.intern(response.url.as_str());
    let status_text_sid = g2.well_known.empty;
    let redirected = response.url_list.len() > 1;

    g2.response_states.insert(
        inst_id,
        ResponseState {
            status: response.status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type: ResponseType::Basic,
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
