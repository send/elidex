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
use super::headers::HeadersGuard;
use super::request_response::{extract_body_bytes, parse_url, ResponseState, ResponseType};

/// Thin wrappers over [`super::super::natives_promise::settle_promise`] so
/// the call sites below read like the old `resolve_promise_sync` /
/// `reject_promise_sync` helpers that `blob.rs` still uses.  The key
/// behavioural difference is that these go through the *normal*
/// settlement path: rejections that land without an attached reaction
/// are queued on [`VmInner::pending_rejections`] so the end-of-drain
/// unhandled-rejection scan can surface them (WHATWG HTML §8.1.5.7).
/// The Body-mixin `_sync` variants intentionally skip the queue because
/// their callers chain `.catch()` immediately; `fetch()` callers don't
/// have the same guarantee — a bare `fetch(url)` with no `.catch` is
/// a common idiom, and browsers warn on its rejection.
fn fetch_resolve(vm: &mut VmInner, promise: ObjectId, value: JsValue) {
    let _ = super::super::natives_promise::settle_promise(vm, promise, false, value);
}

fn fetch_reject(vm: &mut VmInner, promise: ObjectId, reason: JsValue) {
    let _ = super::super::natives_promise::settle_promise(vm, promise, true, reason);
}

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
/// Always returns a Promise — every error path rejects rather
/// than throwing synchronously, matching spec (`fetch()` never
/// synchronously throws, even for obviously bogus inputs).
fn native_fetch(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let promise = super::super::natives_promise::create_promise(ctx.vm);

    // Root `promise` across every subsequent allocation.
    // `alloc_object` contract requires callers to root any
    // `ObjectId` reachable only through a Rust local whenever a
    // later alloc could trigger GC (see `vm/mod.rs::alloc_object`
    // and `feedback_gc_safety.md`).  The guard below pushes the
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
    // constructor step 29 requires the brand check.
    let init_raw = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let signal = match extract_signal_from_init(ctx, init_raw) {
        Ok(sid) => sid,
        Err(err) => {
            let reason = ctx.vm.vm_error_to_thrown(&err);
            fetch_reject(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    // Pre-flight abort: WHATWG Fetch §5.1 main-fetch step 3.
    // Check *before* building the request so an already-aborted
    // signal short-circuits the whole pipeline.
    if let Some(signal_id) = signal {
        if let Some(reason) = pre_flight_abort_reason(ctx, signal_id) {
            fetch_reject(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    }

    // Build the broker-level Request.  Any validation failure
    // settles the Promise directly — no synchronous throw.
    let request = match build_net_request(ctx, args) {
        Ok(req) => req,
        Err(err) => {
            let reason = ctx.vm.vm_error_to_thrown(&err);
            fetch_reject(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    // No handle installed → reject immediately.  Matches
    // `NetworkHandle::disconnected()` semantics for callers that
    // never wired up a broker.
    let Some(handle) = ctx.vm.network_handle.clone() else {
        let err = VmError::type_error("Failed to fetch: no NetworkHandle installed on this VM");
        let reason = ctx.vm.vm_error_to_thrown(&err);
        fetch_reject(ctx.vm, promise, reason);
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
            fetch_resolve(ctx.vm, promise, JsValue::Object(resp_id));
        }
        Err(msg) => {
            // Spec §5.2 "Network error" → TypeError, not
            // DOMException.  Preserve the broker's message for
            // diagnostics but wrap in the spec-prescribed wording.
            let err = VmError::type_error(format!("Failed to fetch: {msg}"));
            let reason = ctx.vm.vm_error_to_thrown(&err);
            fetch_reject(ctx.vm, promise, reason);
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
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to execute 'fetch': 1 argument required, but only 0 present.",
        ));
    }
    let input = args[0];
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let (method_override, headers_override, body_override) = parse_init_overrides(ctx, init)?;

    // Case 1: input is a Request instance — start with its state.
    if let JsValue::Object(obj_id) = input {
        if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Request) {
            let (mut method, url, mut headers, mut body) = request_base_from_vm(ctx, obj_id)?;
            if let Some(m) = method_override {
                method = m;
            }
            if let Some(h) = headers_override {
                headers = h;
            }
            if let Some(b) = body_override {
                body = b;
            }
            return Ok(elidex_net::Request {
                method,
                url,
                headers,
                body,
            });
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
    Ok(elidex_net::Request {
        method: method_override.unwrap_or_else(|| "GET".to_string()),
        url,
        headers: headers_override.unwrap_or_default(),
        body: body_override.unwrap_or_else(Bytes::new),
    })
}

/// Extract the `(method, url, headers, body)` tuple from a VM
/// `Request` instance.  Used as the base for the Request-input
/// path of `fetch()` before `init` overrides are layered on.
fn request_base_from_vm(
    ctx: &NativeContext<'_>,
    obj_id: ObjectId,
) -> Result<(String, Url, Vec<(String, String)>, Bytes), VmError> {
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
    // allocation `body_data` already rooted.
    let body = ctx
        .vm
        .body_data
        .get(&obj_id)
        .map_or_else(Bytes::new, |arc| Bytes::from_owner(Arc::clone(arc)));

    Ok((method, url, headers, body))
}

/// `(method?, headers?, body?)` returned by [`parse_init_overrides`].
/// `None` means the caller's `init` did not set that field — the
/// base (Request-input) or default (URL-input) applies.
type InitOverrides = (Option<String>, Option<Vec<(String, String)>>, Option<Bytes>);

/// Parse the `init` dict.  Every field is `Option<_>`; a present
/// value means `init` explicitly set it.  `undefined` (including
/// the field being absent entirely) always maps to `None`.
/// `null` handling is **field-specific** — see the per-field
/// "Null vs undefined" block below for the source-of-truth
/// semantics.  In short: `headers: null` overrides to empty
/// (matching Request ctor), `body: null` stays as "no override"
/// in Phase 2 (spec-correct null-clears-body lands with the
/// async fetch refactor that implements the full §5.3 walk).
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
            // `extract_body_bytes` already returns `None` for
            // `undefined` / `null`, matching our "field not
            // present" semantics.
            let body_override = if matches!(body_val, JsValue::Undefined) {
                None
            } else {
                extract_body_bytes(ctx, body_val)?.map(Bytes::from_owner)
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

    // Companion Headers — allocate mutable, splice, then flip
    // to Immutable (matches `new Response(...)` contract).
    let headers_id = vm.create_headers(HeadersGuard::None);
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
            let name_sid = vm.strings.intern(&name);
            let value_sid = vm.strings.intern(&value);
            if let Ok((nn, nv)) =
                super::headers::validate_and_normalise(vm, name_sid, value_sid, "response")
            {
                if let Some(state) = vm.headers_states.get_mut(&headers_id) {
                    state.list.push((nn, nv));
                }
            }
        }
        if let Some(state) = vm.headers_states.get_mut(&headers_id) {
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
    // noise (see `m4-12-post-pr5a-fetch-roadmap.md` §PR-spec-polish).
    if !response.body.is_empty() {
        let bytes: Arc<[u8]> = Arc::from(&response.body[..]);
        vm.body_data.insert(inst_id, bytes);
    }

    let url_sid = vm.strings.intern(response.url.as_str());
    let status_text_sid = vm.well_known.empty;
    let redirected = response.url_list.len() > 1;

    vm.response_states.insert(
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
    inst_id
}
