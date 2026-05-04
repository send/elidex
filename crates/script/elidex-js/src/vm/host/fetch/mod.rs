//! `fetch(input, init?)` host global (WHATWG Fetch §5.1).
//!
//! Routes a JS-level fetch request through the embedding-supplied
//! [`elidex_net::broker::NetworkHandle`] (see
//! `Vm::install_network_handle`) and returns a Promise that
//! settles when the broker reply lands on a subsequent
//! [`super::super::Vm::tick_network`] call.
//!
//! ## Async lifecycle (M4-12 PR5-async-fetch)
//!
//! 1. [`native_fetch`] parses arguments, builds an
//!    [`elidex_net::Request`], and calls
//!    [`elidex_net::broker::NetworkHandle::fetch_async`] which
//!    returns a [`elidex_net::FetchId`] immediately.
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
//! - Input as URL string or as a [`elidex_net::Request`] instance.
//!   The VM's existing `Request` constructor handles the
//!   canonicalisation work; `fetch()` calls the same helpers
//!   (`parse_url`, `extract_body_bytes`) from `request_response.rs`
//!   so the behaviour matches byte-for-byte.
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
//!
//! ## File layout
//!
//! Originally a single `fetch.rs` (~1140 LoC); split out into the
//! project's standard 1000-line file convention as part of M4-12
//! PR-file-split-b (slot #10.5b):
//!
//! - this `mod.rs` — registration + [`native_fetch`] entry +
//!   `signal` extraction / pre-flight abort.
//! - [`dispatch`] — [`elidex_net::Request`] construction
//!   (`build_net_request` + the `init` parser + the auto-attached
//!   header helpers).
//! - [`response_install`] — broker reply → VM `Response` object
//!   ([`create_response_from_net`]).

#![cfg(feature = "engine")]

mod dispatch;
mod response_install;

pub(super) use response_install::create_response_from_net;

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError};
use super::super::VmInner;
use super::blob::reject_promise_sync;
use dispatch::build_net_request;

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
    let mut temp_holder = super::super::value::NativeContext { vm: &mut g };
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
            // Non-object init is already rejected by the
            // binding-level guard in [`native_fetch`] before
            // this helper runs; the branch is defensive in case
            // a future caller forwards a raw value here.  Reject
            // with the same spec wording either way.
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
