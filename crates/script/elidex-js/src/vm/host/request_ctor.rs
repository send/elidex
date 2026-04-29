//! `new Request(input, init?)` constructor + private helpers
//! (WHATWG Fetch §5.3).
//!
//! Split from [`super::request_response`] to keep both files below
//! the 1000-line convention (cleanup tranche 2).  The constructor
//! body and three of its private helpers
//! ([`resolve_request_input`] / [`parse_request_init`] /
//! [`normalise_method`]) live here; the wider Request / Response
//! infrastructure — enums, side-table state structs, the Response
//! ctor / static factories, and the shared HTTP helpers
//! ([`super::request_response::validate_http_method`] /
//! [`super::request_response::extract_body_bytes`] /
//! [`super::request_response::parse_url`] /
//! [`super::request_response::fill_headers_like`] /
//! [`super::request_response::copy_headers_entries`]) — stays in
//! the parent module.

#![cfg(feature = "engine")]

use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, StringId, VmError,
};
use super::headers::HeadersGuard;
use super::request_response::{
    content_type_for_body, copy_headers_entries, ensure_content_type, extract_body_bytes,
    fill_headers_like, parse_request_cache, parse_request_credentials, parse_request_mode,
    parse_request_redirect, parse_url, validate_http_method, RedirectMode, RequestCache,
    RequestCredentials, RequestMode, RequestState,
};

/// Returned by [`resolve_request_input`]: when the input is a
/// `Request` instance, the resolved fields carry that Request's
/// state (URL / method / headers / body / mode / credentials /
/// redirect / cache); for a URL-string input the four enum
/// fields default per spec (`Cors` / `SameOrigin` / `Follow` /
/// `Default`) and headers / body are absent.
struct RequestInputParts {
    url_sid: StringId,
    method_sid: StringId,
    source_headers: Option<ObjectId>,
    source_body: Option<Vec<u8>>,
    base_mode: RequestMode,
    base_credentials: RequestCredentials,
    base_redirect: RedirectMode,
    base_cache: RequestCache,
}

/// Returned by [`parse_request_init`].  Each enum override is
/// `None` when `init` did not set the corresponding member; the
/// constructor preserves the base Request's value (or the
/// per-input default for the URL-string path) in that case.
///
/// The `body` slot is **tri-state**: `None` means the caller
/// didn't set `body` at all (preserve the Request clone's base
/// body); `Some(None)` means the caller explicitly set
/// `body: null` and expects the base body to be cleared;
/// `Some(Some(b))` is an explicit replacement with `b`.  The
/// distinction matters because WHATWG Fetch §5.3 step 40 forbids
/// `GET`/`HEAD` requests from carrying a body — a cleared body
/// must not trigger that check, while an explicit empty-string
/// body must (R25.1 / R25.3).
///
/// `body_ct_default` is the optional default `Content-Type`
/// derived from the body type (WHATWG Fetch §5.3 step 38 /
/// §5 "extract a body").  Carried separately from `body` so the
/// FormData boundary-bearing `multipart/form-data; boundary=…`
/// Content-Type (only known after serialisation) and the static
/// [`content_type_for_body`] mapping for String /
/// URLSearchParams / Blob bodies share one channel.  `None`
/// when no default applies (e.g. ArrayBuffer body, or `body:
/// null` cleared path).
struct RequestInitParts {
    method: Option<StringId>,
    headers: Option<JsValue>,
    body: Option<Option<Vec<u8>>>,
    body_ct_default: Option<StringId>,
    mode: Option<RequestMode>,
    credentials: Option<RequestCredentials>,
    redirect: Option<RedirectMode>,
    cache: Option<RequestCache>,
}

/// `new Request(input, init?)` (WHATWG §5.3).
pub(super) fn native_request_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Request': Please use the 'new' operator",
        ));
    }
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to construct 'Request': 1 argument required, but only 0 present.",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let input = args[0];
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let RequestInputParts {
        url_sid,
        method_sid: base_method_sid,
        source_headers,
        source_body: body_bytes,
        base_mode,
        base_credentials,
        base_redirect,
        base_cache,
    } = resolve_request_input(ctx, input)?;
    let RequestInitParts {
        method: override_method,
        headers: headers_init_arg,
        body: body_init_arg,
        body_ct_default,
        mode: mode_override,
        credentials: credentials_override,
        redirect: redirect_override,
        cache: cache_override,
    } = parse_request_init(ctx, init)?;
    let method_sid = override_method.unwrap_or(base_method_sid);
    let mode = mode_override.unwrap_or(base_mode);
    let credentials = credentials_override.unwrap_or(base_credentials);
    let redirect = redirect_override.unwrap_or(base_redirect);
    let cache = cache_override.unwrap_or(base_cache);

    // Allocate companion Headers under the `Request` guard
    // (WHATWG Fetch §5.3 step 31): forbidden-name mutations from
    // both init.headers parse and post-ctor `req.headers.append`
    // silently no-op per §4.6.
    //
    // Root `headers_id` across `fill_headers_like` / `copy_headers_
    // entries` / `body_data.insert` / `request_states.insert`:
    // `headers_states` is **not** itself a GC root — the Headers
    // object is reached only via `request_states[inst_id]
    // .headers_id`, which isn't installed until the end of this
    // function.  `fill_headers_like` in particular can run arbitrary
    // user code (a user-supplied iterable's `.next()` / `.return()`)
    // whose allocations would otherwise collect `headers_id` under a
    // future GC-enabled path.  `inst_id` is the constructor receiver
    // and is already rooted by the caller.  Same invariant as R16
    // (Response clone) / R18.2 (broker Response) (R18-audit).
    let headers_id = ctx.vm.create_headers(HeadersGuard::Request);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(headers_id));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    // Copy entries from either the source Request's headers or the
    // init dict's `headers` value (if provided, it overrides).
    match headers_init_arg {
        Some(h) => fill_headers_like(ctx, headers_id, h, "Failed to construct 'Request'")?,
        None => {
            if let Some(src_headers_id) = source_headers {
                copy_headers_entries(ctx, src_headers_id, headers_id);
            }
        }
    }
    // WHATWG §5.3 step 38: when the body sets a default Content-
    // Type and the caller did not splice one in via init.headers,
    // populate it.  Only fires for `init.body` paths (a Request-
    // clone path preserves the source's CT through
    // `copy_headers_entries`).
    if let Some(ct_sid) = body_ct_default {
        ensure_content_type(ctx, headers_id, ct_sid);
    }

    // Body: resolve the tri-state `init.body` against the source
    // Request's base body.  `None` preserves the base; `Some(None)`
    // clears it; `Some(Some(b))` replaces.  See `RequestInitParts`
    // doc for the spec rationale (R25.3).
    let final_body: Option<Vec<u8>> = match body_init_arg {
        None => body_bytes,
        Some(None) => None,
        Some(Some(b)) => Some(b),
    };

    // WHATWG Fetch §5.3 step 40: a Request whose method is `GET`
    // or `HEAD` cannot carry a body.  Applies to the *final*
    // state, so a clone path that keeps the source's body but
    // overrides the method to `GET` also fails; an explicit
    // `body: null` that clears the body lets `GET`/`HEAD` pass
    // (R25.1).
    if final_body.is_some() {
        let method = ctx.vm.strings.get_utf8(method_sid);
        if method == "GET" || method == "HEAD" {
            return Err(VmError::type_error(format!(
                "Failed to construct 'Request': Request with {method} method cannot have body"
            )));
        }
    }

    if let Some(bytes) = final_body {
        ctx.vm.body_data.insert(inst_id, bytes);
    }

    // Promote the pre-allocated Ordinary instance into Request.
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::Request;
    ctx.vm.request_states.insert(
        inst_id,
        RequestState {
            method_sid,
            url_sid,
            headers_id,
            redirect,
            mode,
            credentials,
            cache,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// Resolve `input` (first arg of `new Request(...)`) into its
/// URL / default method / optional source-headers / optional
/// source-body.
///
/// - String → parse URL (relative → resolve against
///   `navigation.current_url`), method defaults to `"GET"`, no
///   source Headers, no source body.
/// - Request object → copy its state (URL / method / headers id
///   / body Vec).  Body is "taken" from the source per spec §5.3
///   step 37 — but Phase 2 clones the bytes without marking the
///   source as consumed, because the body-used tracking only
///   applies once the Body mixin read methods land.
/// - Anything else → `TypeError`.
fn resolve_request_input(
    ctx: &mut NativeContext<'_>,
    input: JsValue,
) -> Result<RequestInputParts, VmError> {
    match input {
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            let url = parse_url(ctx.vm, &raw)?;
            let url_sid = ctx.vm.strings.intern(url.as_str());
            Ok(RequestInputParts {
                url_sid,
                method_sid: ctx.vm.well_known.http_get,
                source_headers: None,
                source_body: None,
                base_mode: RequestMode::Cors,
                base_credentials: RequestCredentials::SameOrigin,
                base_redirect: RedirectMode::Follow,
                base_cache: RequestCache::Default,
            })
        }
        JsValue::Object(obj_id) => {
            if !matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Request) {
                return Err(VmError::type_error(
                    "Failed to construct 'Request': input must be a URL string or Request",
                ));
            }
            let state = ctx
                .vm
                .request_states
                .get(&obj_id)
                .expect("Request without request_states entry");
            let url_sid = state.url_sid;
            let method_sid = state.method_sid;
            let headers_id = state.headers_id;
            let base_mode = state.mode;
            let base_credentials = state.credentials;
            let base_redirect = state.redirect;
            let base_cache = state.cache;
            let body = ctx.vm.body_data.get(&obj_id).cloned();
            Ok(RequestInputParts {
                url_sid,
                method_sid,
                source_headers: Some(headers_id),
                source_body: body,
                base_mode,
                base_credentials,
                base_redirect,
                base_cache,
            })
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Request': input must be a URL string or Request",
        )),
    }
}

/// Parse the `init` dict (§5.3 step 27-38).  Each member is
/// independent — unset members map to `None` so the caller can
/// preserve the source Request's value (or fall back to the
/// per-input default for the URL-string path).  Invalid enum
/// strings throw TypeError per WebIDL §3.10.7.
fn parse_request_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<RequestInitParts, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok(RequestInitParts {
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
            let wk = &ctx.vm.well_known;
            let method_key = PropertyKey::String(wk.method);
            let headers_key = PropertyKey::String(wk.headers);
            let body_key = PropertyKey::String(wk.body);
            let mode_key = PropertyKey::String(wk.mode);
            let credentials_key = PropertyKey::String(wk.credentials);
            let redirect_key = PropertyKey::String(wk.redirect);
            let cache_key = PropertyKey::String(wk.cache);

            let method_val = ctx.get_property_value(opts_id, method_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;
            let body_val = ctx.get_property_value(opts_id, body_key)?;
            let mode_val = ctx.get_property_value(opts_id, mode_key)?;
            let credentials_val = ctx.get_property_value(opts_id, credentials_key)?;
            let redirect_val = ctx.get_property_value(opts_id, redirect_key)?;
            let cache_val = ctx.get_property_value(opts_id, cache_key)?;

            let method_override = match method_val {
                JsValue::Undefined => None,
                other => Some(normalise_method(ctx, other)?),
            };
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                other => Some(other),
            };
            let mode_override = parse_request_mode(ctx, mode_val, "Failed to construct 'Request'")?;
            let credentials_override =
                parse_request_credentials(ctx, credentials_val, "Failed to construct 'Request'")?;
            let redirect_override =
                parse_request_redirect(ctx, redirect_val, "Failed to construct 'Request'")?;
            let cache_override =
                parse_request_cache(ctx, cache_val, "Failed to construct 'Request'")?;
            // WebIDL nullable body, tri-state (R25.3):
            // - `undefined` → `None` — field absent, preserve the
            //   Request clone's base body.
            // - `null` → `Some(None)` — explicit clear; the final
            //   Request has no body, distinct from an empty body.
            // - anything else → `Some(Some(bytes))` — explicit
            //   replacement, including `''` (empty string) which
            //   is still "a body" for the GET/HEAD check below.
            //
            // Prior to R25.3 `null` collapsed to `Some(empty)`
            // (R15.2 carry-over), which correctly cleared the
            // base body only because we never re-read "was the
            // body present" afterward.  The GET/HEAD check
            // (`native_request_constructor`) now needs the
            // distinction.
            //
            // The default Content-Type (§5.3 step 38 "If r's
            // header list does not contain `Content-Type`, set
            // it") is computed alongside the body extraction so
            // FormData's encoder-derived boundary string isn't
            // re-derived on the caller side.
            let (body_override, body_ct_default) = match body_val {
                JsValue::Undefined => (None, None),
                JsValue::Null => (Some(None), None),
                _ => match extract_body_bytes(ctx, body_val)? {
                    None => (Some(None), None),
                    Some((bytes, Some(ct_override))) => (Some(Some(bytes)), Some(ct_override)),
                    Some((bytes, None)) => {
                        let ct_default = content_type_for_body(ctx, body_val);
                        (Some(Some(bytes)), ct_default)
                    }
                },
            };
            Ok(RequestInitParts {
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
            "Failed to construct 'Request': init must be an object",
        )),
    }
}

/// WHATWG §5.3 step 23 + §4.6 forbidden-method filter.
/// Uppercases canonical method names; rejects `CONNECT` / `TRACE` /
/// `TRACK` (forbidden).  Other tokens pass through verbatim — spec
/// also requires them to match RFC 7230 token syntax, which
/// Phase 2 defers (unknown methods that violate RFC 7230 are
/// accepted and relayed downstream).
fn normalise_method(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<StringId, VmError> {
    let raw_sid = super::super::coerce::to_string(ctx.vm, val)?;
    let raw = ctx.vm.strings.get_utf8(raw_sid);
    let canonical = validate_http_method(&raw, "Failed to construct 'Request'")?;
    let wk = &ctx.vm.well_known;
    // `validate_http_method` returns the uppercased token for the
    // seven canonical methods and the original casing otherwise —
    // dispatching on the uppercase here is safe because the match
    // arms cover exactly the canonical-uppercase forms.
    Ok(match canonical.as_str() {
        "GET" => wk.http_get,
        "HEAD" => wk.http_head,
        "POST" => wk.http_post,
        "PUT" => wk.http_put,
        "DELETE" => wk.http_delete,
        "OPTIONS" => wk.http_options,
        "PATCH" => wk.http_patch,
        _ => ctx.vm.strings.intern(&canonical),
    })
}
