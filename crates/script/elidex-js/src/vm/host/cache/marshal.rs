//! `JsValue` ↔ `CachedEntry` / `Response` / `Request` marshalling for the
//! Cache API host bindings (`#11-cache-api-vm` / D-19 PR-1).
//!
//! All matching / storage algorithm stays in `elidex-cache-api`; this file
//! only converts between the engine-independent [`CachedEntry`] and the
//! VM's payload-free `Request` / `Response` wrappers + their out-of-band
//! state (`request_states` / `response_states` / `headers_states` /
//! `body_data`).

#![cfg(feature = "engine")]

use elidex_cache_api::{CachedEntry, MatchOptions, ResponseType as CacheResponseType};

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::super::headers::HeadersGuard;
use super::super::request_response::{
    parse_url, RedirectMode, RequestCache, RequestCredentials, RequestMode, RequestState,
    ResponseState, ResponseType,
};
use super::CacheHandleState;

/// A request argument resolved into `(url, method, headers)` — the §5
/// "query" / "request" input shared by `match` / `matchAll` / `put` /
/// `delete` / `keys`.
pub(super) type ResolvedRequest = (String, String, Vec<(String, String)>);

// ---------------------------------------------------------------------------
// ResponseType <-> CacheResponseType (same six variants, two crates)
// ---------------------------------------------------------------------------

fn response_rt_to_cache_rt(rt: ResponseType) -> CacheResponseType {
    match rt {
        ResponseType::Basic => CacheResponseType::Basic,
        ResponseType::Cors => CacheResponseType::Cors,
        ResponseType::Default => CacheResponseType::Default,
        ResponseType::Error => CacheResponseType::Error,
        ResponseType::Opaque => CacheResponseType::Opaque,
        ResponseType::OpaqueRedirect => CacheResponseType::OpaqueRedirect,
    }
}

fn cache_rt_to_response_rt(rt: CacheResponseType) -> ResponseType {
    match rt {
        CacheResponseType::Basic => ResponseType::Basic,
        CacheResponseType::Cors => ResponseType::Cors,
        CacheResponseType::Default => ResponseType::Default,
        CacheResponseType::Error => ResponseType::Error,
        CacheResponseType::Opaque => ResponseType::Opaque,
        CacheResponseType::OpaqueRedirect => ResponseType::OpaqueRedirect,
    }
}

// ---------------------------------------------------------------------------
// Build a `Cache` wrapper (caches.open / the per-cache façade)
// ---------------------------------------------------------------------------

/// Allocate a `Cache` wrapper (`ObjectKind::Cache`) for `cache_name` +
/// register its [`CacheHandleState`].  Several `Cache` instances may name
/// the same cache (every `caches.open("v1")` returns a fresh wrapper);
/// they all route to the same `entries` rows, so identity carries no
/// per-instance data beyond the name.
pub(super) fn build_cache_object(vm: &mut VmInner, cache_name: &str) -> ObjectId {
    let proto = vm.cache_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Cache,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.cache_handle_states.insert(
        id,
        CacheHandleState {
            cache_name: cache_name.to_owned(),
        },
    );
    id
}

// ---------------------------------------------------------------------------
// CachedEntry -> Response / Request (cache.match / matchAll / keys)
// ---------------------------------------------------------------------------

/// Intern `(name, value)` header pairs and install them as the `list` of
/// the freshly-created Headers `headers_id`.  Names are lowercased (the
/// `headers_states` `list` invariant); values are stored verbatim.
fn install_header_list(vm: &mut VmInner, headers_id: ObjectId, pairs: &[(String, String)]) {
    let mut list = Vec::with_capacity(pairs.len());
    for (name, value) in pairs {
        let name_sid = vm.strings.intern(&name.to_ascii_lowercase());
        let value_sid = vm.strings.intern(value);
        list.push((name_sid, value_sid));
    }
    if let Some(state) = vm.headers_states.get_mut(&headers_id) {
        state.list = list;
    }
}

/// Build a `Response` wrapper from a stored [`CachedEntry`] (the
/// `cache.match` / `matchAll` result).  The companion Headers is
/// `Immutable` (a fetched/stored response surface, WHATWG Fetch §5.5).
pub(super) fn build_response_from_entry(vm: &mut VmInner, entry: &CachedEntry) -> ObjectId {
    let proto = vm.response_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Root the just-allocated Response across `create_headers` (which
    // allocates a companion Headers and can GC before the Response is
    // reachable from any root — it lives only in `id` until the state insert).
    let mut g = vm.push_temp_root(JsValue::Object(id));

    let headers_id = g.create_headers(HeadersGuard::Immutable);
    install_header_list(&mut g, headers_id, &entry.response_headers);

    if !entry.response_body.is_empty() {
        g.body_data.insert(id, entry.response_body.clone());
    }

    let status_text_sid = g.strings.intern(&entry.response_status_text);
    // Fetch §2.2.6: `response.url` is the final URL after redirects (last in
    // the chain), or the **empty string** when the URL list is empty (a
    // synthetic `new Response(...)`).  Do NOT synthesize it from the request
    // URL — the Cache "match" algorithm returns the stored response's own
    // URL, so a synthetic response's `url` must stay `""` across a put/match
    // round-trip.
    let final_url = entry.response_url_list.last().cloned().unwrap_or_default();
    let url_sid = g.strings.intern(&final_url);
    g.response_states.insert(
        id,
        ResponseState {
            status: entry.response_status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type: cache_rt_to_response_rt(entry.response_type),
            redirected: entry.response_url_list.len() > 1,
        },
    );
    id
}

/// Build a `Request` wrapper from a stored [`CachedEntry`] (the
/// `cache.keys()` result).  Mode / credentials / cache / redirect are not
/// persisted by the cache, so the reconstructed key uses the Request
/// constructor defaults.
pub(super) fn build_request_from_entry(vm: &mut VmInner, entry: &CachedEntry) -> ObjectId {
    let proto = vm.request_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Request,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Root the just-allocated Request across `create_headers` (allocates a
    // companion Headers and can GC before the Request is root-reachable).
    let mut g = vm.push_temp_root(JsValue::Object(id));

    let headers_id = g.create_headers(HeadersGuard::Request);
    install_header_list(&mut g, headers_id, &entry.request_headers);

    let method_sid = g.strings.intern(&entry.request_method);
    let url_sid = g.strings.intern(&entry.request_url);
    g.request_states.insert(
        id,
        RequestState {
            method_sid,
            url_sid,
            headers_id,
            redirect: RedirectMode::Follow,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
        },
    );
    id
}

// ---------------------------------------------------------------------------
// Request arg -> (url, method, headers)  (the §5 "query" / "request" input)
// ---------------------------------------------------------------------------

/// Snapshot a Headers `list` into owned `(name, value)` String pairs.
fn headers_to_owned(vm: &VmInner, headers_id: ObjectId) -> Vec<(String, String)> {
    let Some(state) = vm.headers_states.get(&headers_id) else {
        return Vec::new();
    };
    // Borrow the list in place — `headers_states` and `strings` are disjoint
    // `VmInner` fields, so both shared borrows coexist; no need to clone the
    // `(StringId, StringId)` list first.
    state
        .list
        .iter()
        .map(|(k, v)| (vm.strings.get_utf8(*k), vm.strings.get_utf8(*v)))
        .collect()
}

/// Resolve a Cache API request argument into `(url, method, headers)`.
///
/// A `Request` object contributes its method + headers; any other value
/// is treated as a URL string (the spec "if request is a string, invoke
/// the Request constructor" path — GET, no headers, resolved against the
/// current base URL).
pub(super) fn resolve_request(
    ctx: &mut NativeContext<'_>,
    arg: Option<&JsValue>,
    op: &str,
) -> Result<ResolvedRequest, VmError> {
    // A missing argument (0 args to a required-`request` op — `match` /
    // `delete`) is a WebIDL TypeError, surfaced as a rejected Promise.  The
    // optional-request callers (`matchAll` / `keys`) handle the no-request
    // case themselves and only reach here with an explicit value.  `op` is
    // the WebIDL-style "'<operation>' on '<interface>'" label so the message
    // matches the rest of the VM's required-argument errors.
    let Some(&arg) = arg else {
        return Err(VmError::type_error(format!(
            "Failed to execute {op}: 1 argument required, but only 0 present."
        )));
    };
    if let JsValue::Object(id) = arg {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::Request) {
            let (method_sid, url_sid, headers_id) = {
                let st = ctx.vm.request_states.get(&id).ok_or_else(|| {
                    VmError::type_error("Cache: Request argument has no internal state")
                })?;
                (st.method_sid, st.url_sid, st.headers_id)
            };
            let method = ctx.vm.strings.get_utf8(method_sid);
            let url = ctx.vm.strings.get_utf8(url_sid);
            let headers = headers_to_owned(ctx.vm, headers_id);
            return Ok((url, method, headers));
        }
    }
    // URL-string path (also covers ToString-coercible values).
    let sid = super::super::super::coerce::to_string(ctx.vm, arg)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    let url = parse_url(ctx.vm, &raw)?.to_string();
    Ok((url, "GET".to_owned(), Vec::new()))
}

// ---------------------------------------------------------------------------
// Response arg -> CachedEntry  (cache.put)
// ---------------------------------------------------------------------------

/// Build a [`CachedEntry`] from a put's request tuple + a `Response`
/// object.  Enforces the §5.4.5 put rejections that depend on the
/// response: status 206 (partial) and `Vary: *`.  The non-GET-method
/// rejection is checked by the caller against the resolved request method
/// before this runs.
pub(super) fn entry_from_response(
    ctx: &mut NativeContext<'_>,
    request_url: String,
    request_method: String,
    request_headers: Vec<(String, String)>,
    response_arg: JsValue,
) -> Result<CachedEntry, VmError> {
    let JsValue::Object(response_id) = response_arg else {
        return Err(VmError::type_error(
            "Cache.put: response argument is not a Response",
        ));
    };
    if !matches!(ctx.vm.get_object(response_id).kind, ObjectKind::Response) {
        return Err(VmError::type_error(
            "Cache.put: response argument is not a Response",
        ));
    }
    // §5.4.5 step 2: a Response whose body is already consumed (`disturbed`)
    // or locked to a reader cannot be cached — reject before reading bytes
    // (otherwise the spent body would silently store as empty).
    if ctx.vm.disturbed.contains(&response_id)
        || super::super::body_mixin::is_body_locked(ctx.vm, response_id)
    {
        return Err(VmError::type_error(
            "Cache.put: response body is already used",
        ));
    }
    let (status, response_type, status_text_sid, url_sid, headers_id, redirected) = {
        let st = ctx.vm.response_states.get(&response_id).ok_or_else(|| {
            VmError::type_error("Cache.put: Response argument has no internal state")
        })?;
        (
            st.status,
            st.response_type,
            st.status_text_sid,
            st.url_sid,
            st.headers_id,
            st.redirected,
        )
    };

    // §5.4.5: a 206 (partial) response cannot be cached.
    if status == 206 {
        return Err(VmError::type_error(
            "Cache.put: a partial response (206) cannot be cached",
        ));
    }

    let response_status_text = ctx.vm.strings.get_utf8(status_text_sid);
    let response_headers = headers_to_owned(ctx.vm, headers_id);
    let response_body = ctx
        .vm
        .body_data
        .get(&response_id)
        .cloned()
        .unwrap_or_default();
    let final_url = ctx.vm.strings.get_utf8(url_sid);

    // §5.4.5: reject Vary:*; otherwise capture the request-side values the
    // response's Vary header references (the cache match key).  The Vary
    // algorithm lives in `elidex-cache-api` next to its consumer
    // `entry_matches`; host/ only maps the `Vary: *` rejection to a JS
    // TypeError (the crate's only error from this call).
    let vary_headers =
        elidex_cache_api::entry::compute_vary_key(&response_headers, &request_headers).map_err(
            |_| VmError::type_error("Cache.put: a response with 'Vary: *' cannot be cached"),
        )?;

    let is_opaque = matches!(
        response_type,
        ResponseType::Opaque | ResponseType::OpaqueRedirect
    );
    // Fetch §5.5: the `redirected` getter is "URL list's size is greater than
    // 1" and the `url` getter is the last entry of the response's URL list
    // (Fetch §2.2.6).  Those are the only two URL-list observables that survive
    // a Cache round-trip, so preserve `redirected` by storing the [request,
    // final] endpoints when the response was redirected (intermediate redirect
    // hops are not Cache-observable).
    let response_url_list = if final_url.is_empty() {
        Vec::new()
    } else if redirected {
        vec![request_url.clone(), final_url]
    } else {
        vec![final_url]
    };

    Ok(CachedEntry {
        request_url,
        request_method,
        request_headers,
        response_status: status,
        response_status_text,
        response_headers,
        response_body,
        response_url_list,
        response_type: response_rt_to_cache_rt(response_type),
        vary_headers,
        is_opaque,
    })
}

// ---------------------------------------------------------------------------
// CacheQueryOptions -> MatchOptions
// ---------------------------------------------------------------------------

/// Parse a `CacheQueryOptions` dictionary into [`MatchOptions`].  Reads
/// `ignoreSearch` / `ignoreMethod` / `ignoreVary` via ordinary `Get`
/// (prototype walk + accessor getters), each `ToBoolean`-coerced.  Per
/// WebIDL §3.2.17 dictionary conversion, `undefined` / `null` yield the
/// empty dictionary (all members default), while a non-nullish, non-object
/// argument is a `TypeError` (it is NOT `ToObject`-boxed — there is no
/// prototype read of a wrapper).  `cacheName` (a `CacheStorage.match`-only
/// member) is read by that caller, not here; it is reached only after this
/// gate, so it never sees a primitive argument.
pub(super) fn parse_query_options(
    ctx: &mut NativeContext<'_>,
    arg: JsValue,
) -> Result<MatchOptions, VmError> {
    let mut opts = MatchOptions::default();
    match arg {
        // WebIDL §3.2.17: `undefined` / `null` → empty dictionary.
        JsValue::Undefined | JsValue::Null => {}
        JsValue::Object(id) => {
            opts.ignore_search = read_bool_member(ctx, id, "ignoreSearch")?;
            opts.ignore_method = read_bool_member(ctx, id, "ignoreMethod")?;
            opts.ignore_vary = read_bool_member(ctx, id, "ignoreVary")?;
        }
        // WebIDL §3.2.17 step 1: a non-nullish, non-object value is not a
        // valid dictionary → TypeError (surfaced as a rejected Promise).
        _ => {
            return Err(VmError::type_error(
                "Failed to read the 'CacheQueryOptions' dictionary: argument is not an object",
            ));
        }
    }
    Ok(opts)
}

fn read_bool_member(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    member: &str,
) -> Result<bool, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(member));
    let val = ctx.get_property_value(obj_id, key)?;
    Ok(super::super::super::coerce::to_boolean(ctx.vm, val))
}

/// Read the optional `cacheName` member of a `CacheStorage.match` options
/// dict (§5.5.1) — when present, the cross-cache search is restricted to
/// that one cache.  Returns `None` for absent / `undefined`; an explicit
/// `null` is *present* and coerces to the `DOMString` `"null"` (WebIDL
/// non-nullable `DOMString` conversion, parity with `cache_name_arg`).
pub(super) fn read_cache_name_option(
    ctx: &mut NativeContext<'_>,
    arg: JsValue,
) -> Result<Option<String>, VmError> {
    let JsValue::Object(id) = arg else {
        return Ok(None);
    };
    let key = PropertyKey::String(ctx.vm.strings.intern("cacheName"));
    let val = ctx.get_property_value(id, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
    Ok(Some(ctx.vm.strings.get_utf8(sid)))
}
