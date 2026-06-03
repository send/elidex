//! Cache API native methods (WHATWG Service Workers §5.4 `Cache` / §5.5
//! `CacheStorage`; slot `#11-cache-api-vm` / D-19 PR-1).
//!
//! Every method is Promise-returning.  Per WebIDL §3.7.2.1 a
//! promise-returning operation turns *all* synchronous failures (bad
//! receiver brand, argument coercion, the algorithm's own `TypeError`s)
//! into a **rejected** Promise rather than a thrown exception — so each
//! native funnels through an `*_outcome` helper returning
//! `Result<CacheDelivery, VmError>` and [`finish`] converts an `Err` into
//! a rejection.  The backend call runs synchronously; the Promise settles
//! at the event-loop tail via [`super::settle_async`] (DR-A.1).
//!
//! `add` / `addAll` are intentionally absent — see the module docstring
//! (slot `#11-cache-add-fetch-integration`).

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::{marshal, CacheDelivery};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Convert an `*_outcome` result into the Promise to hand back: settle the
/// success outcome, or reject with the error's thrown value (WebIDL
/// promise-returning-operation wrapping).
fn finish(
    ctx: &mut NativeContext<'_>,
    result: Result<CacheDelivery, VmError>,
) -> Result<JsValue, VmError> {
    let outcome = match result {
        Ok(o) => o,
        Err(e) => {
            let reason = ctx.vm.vm_error_to_thrown(&e);
            CacheDelivery::Reject(reason)
        }
    };
    Ok(super::settle_async(ctx.vm, outcome))
}

/// Brand-check that `this` is the `caches` `CacheStorage` singleton.
fn require_cache_storage(ctx: &NativeContext<'_>, this: JsValue) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CacheStorage) {
            return Ok(());
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a CacheStorage",
    ))
}

/// Brand-check that `this` is a `Cache` and return its cache name.
fn require_cache_name(ctx: &NativeContext<'_>, this: JsValue) -> Result<String, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::Cache) {
            return ctx
                .vm
                .cache_handle_states
                .get(&id)
                .map(|s| s.cache_name.clone())
                .ok_or_else(|| VmError::type_error("Cache handle has no internal state"));
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a Cache",
    ))
}

/// Required `cacheName` `DOMString` argument (`ToString`-coerced).
fn cache_name_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    op: &str,
) -> Result<String, VmError> {
    let Some(&arg) = args.first() else {
        return Err(VmError::type_error(format!(
            "{op}: 1 argument required, but only 0 present."
        )));
    };
    let sid = super::super::super::coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

// ---------------------------------------------------------------------------
// CacheStorage (`caches`) — §5.5
// ---------------------------------------------------------------------------

/// `caches.open(cacheName)` → `Promise<Cache>` (§5.5.3).
pub(crate) fn native_caches_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = caches_open_outcome(ctx, this, args);
    finish(ctx, r)
}

fn caches_open_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    require_cache_storage(ctx, this)?;
    let name = cache_name_arg(ctx, args, "Failed to execute 'open' on 'CacheStorage'")?;
    let backend = ctx.vm.require_cache_backend()?;
    backend
        .with_conn(|conn| elidex_cache_api::storage::open(conn, &name))
        .map_err(|e| VmError::type_error(format!("{e}")))?;
    let cache_obj = marshal::build_cache_object(ctx.vm, &name);
    Ok(CacheDelivery::Resolve(JsValue::Object(cache_obj)))
}

/// `caches.has(cacheName)` → `Promise<boolean>` (§5.5.2).
pub(crate) fn native_caches_has(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = caches_has_outcome(ctx, this, args);
    finish(ctx, r)
}

fn caches_has_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    require_cache_storage(ctx, this)?;
    let name = cache_name_arg(ctx, args, "Failed to execute 'has' on 'CacheStorage'")?;
    let backend = ctx.vm.require_cache_backend()?;
    let exists =
        backend.with_conn(|conn| elidex_cache_api::storage::has(conn, &name).unwrap_or(false));
    Ok(CacheDelivery::Resolve(JsValue::Boolean(exists)))
}

/// `caches.delete(cacheName)` → `Promise<boolean>` (§5.5.4).
pub(crate) fn native_caches_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = caches_delete_outcome(ctx, this, args);
    finish(ctx, r)
}

fn caches_delete_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    require_cache_storage(ctx, this)?;
    let name = cache_name_arg(ctx, args, "Failed to execute 'delete' on 'CacheStorage'")?;
    let backend = ctx.vm.require_cache_backend()?;
    let deleted =
        backend.with_conn(|conn| elidex_cache_api::storage::delete(conn, &name).unwrap_or(false));
    Ok(CacheDelivery::Resolve(JsValue::Boolean(deleted)))
}

/// `caches.keys()` → `Promise<sequence<DOMString>>` (§5.5.5), creation
/// order.
pub(crate) fn native_caches_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = caches_keys_outcome(ctx, this);
    finish(ctx, r)
}

fn caches_keys_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
) -> Result<CacheDelivery, VmError> {
    require_cache_storage(ctx, this)?;
    let backend = ctx.vm.require_cache_backend()?;
    let names = backend.with_conn(|conn| elidex_cache_api::storage::keys(conn).unwrap_or_default());
    let mut elements = Vec::with_capacity(names.len());
    for name in &names {
        elements.push(JsValue::String(ctx.vm.strings.intern(name)));
    }
    let arr = ctx.vm.create_array_object(elements);
    Ok(CacheDelivery::Resolve(JsValue::Object(arr)))
}

/// `caches.match(request, options?)` → `Promise<Response | undefined>`
/// (§5.5.1) — searches each cache (or `options.cacheName` only).
pub(crate) fn native_caches_match(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = caches_match_outcome(ctx, this, args);
    finish(ctx, r)
}

fn caches_match_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    require_cache_storage(ctx, this)?;
    let (url, method, headers) = marshal::resolve_request(ctx, args.first())?;
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let opts = marshal::parse_query_options(ctx, options_arg)?;
    let only_cache = marshal::read_cache_name_option(ctx, options_arg)?;
    let backend = ctx.vm.require_cache_backend()?;
    let found = backend.with_conn(|conn| {
        let names = match &only_cache {
            Some(name) => vec![name.clone()],
            None => elidex_cache_api::storage::keys(conn).unwrap_or_default(),
        };
        for name in names {
            if let Ok(Some(entry)) =
                elidex_cache_api::store::match_request(conn, &name, &url, &method, &headers, &opts)
            {
                return Some(entry);
            }
        }
        None
    });
    Ok(match found {
        Some(entry) => {
            let resp = marshal::build_response_from_entry(ctx.vm, &entry);
            CacheDelivery::Resolve(JsValue::Object(resp))
        }
        None => CacheDelivery::Resolve(JsValue::Undefined),
    })
}

// ---------------------------------------------------------------------------
// Cache — §5.4
// ---------------------------------------------------------------------------

/// `cache.match(request, options?)` → `Promise<Response | undefined>`
/// (§5.4.1).
pub(crate) fn native_cache_match(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = cache_match_outcome(ctx, this, args);
    finish(ctx, r)
}

fn cache_match_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    let name = require_cache_name(ctx, this)?;
    let (url, method, headers) = marshal::resolve_request(ctx, args.first())?;
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let opts = marshal::parse_query_options(ctx, options_arg)?;
    let backend = ctx.vm.require_cache_backend()?;
    let found = backend.with_conn(|conn| {
        elidex_cache_api::store::match_request(conn, &name, &url, &method, &headers, &opts)
            .ok()
            .flatten()
    });
    Ok(match found {
        Some(entry) => {
            let resp = marshal::build_response_from_entry(ctx.vm, &entry);
            CacheDelivery::Resolve(JsValue::Object(resp))
        }
        None => CacheDelivery::Resolve(JsValue::Undefined),
    })
}

/// `cache.matchAll(request?, options?)` → `Promise<sequence<Response>>`
/// (§5.4.2).  No request → every stored response.
pub(crate) fn native_cache_match_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = cache_match_all_outcome(ctx, this, args);
    finish(ctx, r)
}

fn cache_match_all_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    let name = require_cache_name(ctx, this)?;
    let req_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let opts = marshal::parse_query_options(ctx, options_arg)?;
    let request = if matches!(req_arg, JsValue::Undefined) {
        None
    } else {
        Some(marshal::resolve_request(ctx, Some(&req_arg))?)
    };
    let backend = ctx.vm.require_cache_backend()?;
    let entries = backend.with_conn(|conn| match &request {
        None => elidex_cache_api::store::keys(conn, &name).unwrap_or_default(),
        Some((url, method, headers)) => {
            elidex_cache_api::store::match_all(conn, &name, url, method, headers, &opts)
                .unwrap_or_default()
        }
    });
    let mut elements = Vec::with_capacity(entries.len());
    for entry in &entries {
        elements.push(JsValue::Object(marshal::build_response_from_entry(
            ctx.vm, entry,
        )));
    }
    let arr = ctx.vm.create_array_object(elements);
    Ok(CacheDelivery::Resolve(JsValue::Object(arr)))
}

/// `cache.put(request, response)` → `Promise<undefined>` (§5.4.5).
pub(crate) fn native_cache_put(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = cache_put_outcome(ctx, this, args);
    finish(ctx, r)
}

fn cache_put_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    let name = require_cache_name(ctx, this)?;
    let req_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let resp_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (url, method, headers) = marshal::resolve_request(ctx, Some(&req_arg))?;
    // §5.4.5: only GET requests can be cached.
    if !method.eq_ignore_ascii_case("GET") {
        return Err(VmError::type_error(
            "Cache.put: request method must be 'GET'",
        ));
    }
    let entry = marshal::entry_from_response(ctx, url, method, headers, resp_arg)?;
    let backend = ctx.vm.require_cache_backend()?;
    backend
        .with_conn(|conn| elidex_cache_api::store::put(conn, &name, &entry))
        .map_err(|e| VmError::type_error(format!("{e}")))?;
    Ok(CacheDelivery::Resolve(JsValue::Undefined))
}

/// `cache.delete(request, options?)` → `Promise<boolean>` (§5.4.6).
pub(crate) fn native_cache_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = cache_delete_outcome(ctx, this, args);
    finish(ctx, r)
}

fn cache_delete_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    let name = require_cache_name(ctx, this)?;
    let (url, method, headers) = marshal::resolve_request(ctx, args.first())?;
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let opts = marshal::parse_query_options(ctx, options_arg)?;
    let backend = ctx.vm.require_cache_backend()?;
    let deleted = backend.with_conn(|conn| {
        elidex_cache_api::store::delete(conn, &name, &url, &method, &headers, &opts)
            .unwrap_or(false)
    });
    Ok(CacheDelivery::Resolve(JsValue::Boolean(deleted)))
}

/// `cache.keys(request?, options?)` → `Promise<sequence<Request>>`
/// (§5.4.7).  No request → every stored request key (creation order); a
/// request restricts the result to the entries matching it (honouring
/// `CacheQueryOptions`).
pub(crate) fn native_cache_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let r = cache_keys_outcome(ctx, this, args);
    finish(ctx, r)
}

fn cache_keys_outcome(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<CacheDelivery, VmError> {
    let name = require_cache_name(ctx, this)?;
    let req_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let entries = if matches!(req_arg, JsValue::Undefined) {
        let backend = ctx.vm.require_cache_backend()?;
        backend.with_conn(|conn| elidex_cache_api::store::keys(conn, &name).unwrap_or_default())
    } else {
        let (url, method, headers) = marshal::resolve_request(ctx, Some(&req_arg))?;
        let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let opts = marshal::parse_query_options(ctx, options_arg)?;
        let backend = ctx.vm.require_cache_backend()?;
        backend.with_conn(|conn| {
            elidex_cache_api::store::match_all(conn, &name, &url, &method, &headers, &opts)
                .unwrap_or_default()
        })
    };
    let mut elements = Vec::with_capacity(entries.len());
    for entry in &entries {
        elements.push(JsValue::Object(marshal::build_request_from_entry(
            ctx.vm, entry,
        )));
    }
    let arr = ctx.vm.create_array_object(elements);
    Ok(CacheDelivery::Resolve(JsValue::Object(arr)))
}
