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
    copy_headers_entries, extract_body_bytes, fill_headers_like, parse_url, validate_http_method,
    RedirectMode, RequestCache, RequestCredentials, RequestMode, RequestState,
};

/// Tuple returned by [`resolve_request_input`]: the URL StringId
/// is canonicalised; method defaults to `GET` unless the input was
/// itself a `Request` (then its method carries over); `source_headers`
/// is `Some` for the Request-clone case; `source_body` is the cloned
/// body Vec (may be `None`).
type RequestInputParts = (StringId, StringId, Option<ObjectId>, Option<Vec<u8>>);

/// Tuple returned by [`parse_request_init`]: optional method
/// override, optional headers-init source (copied into the
/// companion Headers), optional body bytes.
///
/// The `body` slot is **tri-state**: `None` means the caller
/// didn't set `body` at all (preserve the Request clone's base
/// body); `Some(None)` means the caller explicitly set `body:
/// null` and expects the base body to be cleared; `Some(Some(b))`
/// is an explicit replacement with `b`.  The distinction matters
/// because WHATWG Fetch §5.3 step 40 forbids `GET`/`HEAD`
/// requests from carrying a body — a cleared body must not
/// trigger that check, while an explicit empty-string body must
/// (R25.1 / R25.3).
type RequestInitParts = (Option<StringId>, Option<JsValue>, Option<Option<Vec<u8>>>);

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

    let (url_sid, method_sid, headers_source, body_bytes) = resolve_request_input(ctx, input)?;
    let (override_method, headers_init_arg, body_init_arg) = parse_request_init(ctx, init)?;
    let method_sid = override_method.unwrap_or(method_sid);

    // Allocate companion Headers (guard = None; a later PR tightens
    // to `request` guard once the forbidden-header list is enforced).
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
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(headers_id));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    // Copy entries from either the source Request's headers or the
    // init dict's `headers` value (if provided, it overrides).
    match headers_init_arg {
        Some(h) => fill_headers_like(ctx, headers_id, h, "Failed to construct 'Request'")?,
        None => {
            if let Some(src_headers_id) = headers_source {
                copy_headers_entries(ctx, src_headers_id, headers_id);
            }
        }
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
            redirect: RedirectMode::Follow,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
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
            let method_sid = ctx.vm.well_known.http_get;
            Ok((url_sid, method_sid, None, None))
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
            let body = ctx.vm.body_data.get(&obj_id).cloned();
            Ok((url_sid, method_sid, Some(headers_id), body))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Request': input must be a URL string or Request",
        )),
    }
}

/// Parse the `init` dict (§5.3 step 27-38).  Returns the
/// optional method override, optional headers source, and
/// optional body bytes.  Unknown members are ignored silently.
fn parse_request_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<RequestInitParts, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok((None, None, None)),
        JsValue::Object(opts_id) => {
            let wk = &ctx.vm.well_known;
            let method_key = PropertyKey::String(wk.method);
            let headers_key = PropertyKey::String(wk.headers);
            let body_key = PropertyKey::String(wk.body);

            let method_val = ctx.get_property_value(opts_id, method_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;
            let body_val = ctx.get_property_value(opts_id, body_key)?;

            let method_override = match method_val {
                JsValue::Undefined => None,
                other => Some(normalise_method(ctx, other)?),
            };
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                other => Some(other),
            };
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
            let body_override = match body_val {
                JsValue::Undefined => None,
                JsValue::Null => Some(None),
                _ => Some(extract_body_bytes(ctx, body_val)?),
            };
            Ok((method_override, headers_override, body_override))
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
