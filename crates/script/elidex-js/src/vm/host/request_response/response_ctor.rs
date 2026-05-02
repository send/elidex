//! `new Response(body?, init?)` constructor + the shared
//! `build_response_instance` / `parse_response_init` helpers used
//! by both the constructor and the static factories
//! ([`super::response_statics`]).
//!
//! Split from [`super`] (`request_response/mod.rs`) to keep each
//! file under the project's 1000-line convention.

#![cfg(feature = "engine")]

use super::super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, StringId, VmError,
};
use super::super::headers::HeadersGuard;
use super::{
    content_type_for_body, ensure_content_type, extract_body_bytes, fill_headers_like,
    ResponseState, ResponseType,
};

/// `new Response(body?, init?)` (WHATWG §5.5).
pub(super) fn native_response_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Response': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    let body_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let extracted = extract_body_bytes(ctx, body_arg)?;
    let body_default_content_type = match &extracted {
        // FormData returns a boundary-bearing override that
        // supersedes the static `content_type_for_body` mapping
        // (the boundary is only known after serialisation).  For
        // every other body kind the helper agrees with
        // `content_type_for_body`, so falling back to the latter
        // keeps the wiring deterministic.
        Some((_, Some(ct))) => Some(*ct),
        _ => content_type_for_body(ctx, body_arg),
    };
    let body_bytes = extracted.map(|(b, _)| b);
    build_response_instance(
        ctx,
        inst_id,
        body_bytes,
        body_default_content_type,
        init_arg,
        ResponseType::Default,
        0,
        false,
    )?;
    Ok(JsValue::Object(inst_id))
}

/// Build a Response on `inst_id` from parsed body bytes + init
/// dict.  Shared between the public `new Response(...)` path and
/// the `Response.redirect` / `Response.json` static factories.
///
/// `redirected` / `synthetic_status` override the init status when
/// non-zero (used by `Response.redirect(...)`).
#[allow(clippy::too_many_arguments)]
pub(super) fn build_response_instance(
    ctx: &mut NativeContext<'_>,
    inst_id: ObjectId,
    body_bytes: Option<Vec<u8>>,
    body_default_content_type: Option<StringId>,
    init_arg: JsValue,
    response_type: ResponseType,
    synthetic_status: u16,
    redirected: bool,
) -> Result<(), VmError> {
    let (status_from_init, status_text_sid, init_headers) = parse_response_init(ctx, init_arg)?;
    let status = if synthetic_status != 0 {
        synthetic_status
    } else {
        status_from_init.unwrap_or(200)
    };

    // WHATWG §5.5 step "initialize a response" → reject null body
    // statuses (204 / 205 / 304) with an attached body (spec
    // prescribes `TypeError`).
    if matches!(status, 204 | 205 | 304) && body_bytes.is_some() {
        return Err(VmError::type_error(
            "Failed to construct 'Response': Response with null body status cannot have body",
        ));
    }

    // Allocate the companion Headers as `None` (mutable) so the
    // subsequent `init.headers` copy and default `Content-Type`
    // splice can succeed, then flip the guard to `Immutable` in
    // the block below — WHATWG Fetch §5.5 step 11 demands the
    // post-ctor surface be immutable so `resp.headers.append(...)`
    // throws TypeError.
    // Root `headers_id` across `fill_headers_like` (may invoke
    // user-supplied iterables' `.next()` / `.return()`) +
    // `ensure_content_type` + `body_data.insert` +
    // `response_states.insert`.  `headers_states` is not a GC
    // root on its own — the Headers is reached only via
    // `response_states[inst_id].headers_id`, which isn't installed
    // until the end of this helper.  `inst_id` is the ctor receiver
    // and is already rooted by the caller.  Same invariant as
    // R18.2 / Audit 1 (R18-audit).
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(headers_id));
    let mut rooted_holder = super::super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    if let Some(hval) = init_headers {
        fill_headers_like(ctx, headers_id, hval, "Failed to construct 'Response'")?;
    }
    // If the caller supplied a default `Content-Type` and the user
    // didn't already set one via `init.headers`, populate it —
    // mirrors §5.5 "initialize a response" extract-body step 2.
    if let Some(ct_sid) = body_default_content_type {
        ensure_content_type(ctx, headers_id, ct_sid);
    }
    // Promote the guard to immutable only after we're done
    // mutating — the public Headers handle will refuse further
    // mutation from script.
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.guard = HeadersGuard::Immutable;
    }

    if let Some(bytes) = body_bytes {
        ctx.vm.body_data.insert(inst_id, bytes);
    }

    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::Response;
    let url_sid = ctx.vm.well_known.empty;
    ctx.vm.response_states.insert(
        inst_id,
        ResponseState {
            status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type,
            redirected,
        },
    );
    Ok(())
}

/// Parse a `ResponseInit` dict.  Returns `(status, statusText,
/// headers)` (each optional where spec permits a default).
fn parse_response_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<(Option<u16>, StringId, Option<JsValue>), VmError> {
    let default_status_text = ctx.vm.well_known.empty;
    match init {
        JsValue::Undefined | JsValue::Null => Ok((None, default_status_text, None)),
        JsValue::Object(opts_id) => {
            let wk = &ctx.vm.well_known;
            let status_key = PropertyKey::String(wk.status);
            let status_text_key = PropertyKey::String(wk.status_text);
            let headers_key = PropertyKey::String(wk.headers);

            let status_val = ctx.get_property_value(opts_id, status_key)?;
            let status_text_val = ctx.get_property_value(opts_id, status_text_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;

            let status = if matches!(status_val, JsValue::Undefined) {
                None
            } else {
                // WebIDL `[EnforceRange] unsigned short` — reject
                // NaN / ±∞ / out-of-[0,65535] as TypeError *before*
                // the spec's 200..=599 RangeError check (§5.5
                // "initialize a response" step 1 implicitly relies
                // on the earlier conversion rejecting wraps).
                let n = super::super::super::coerce::to_number(ctx.vm, status_val)?;
                let code = super::super::super::coerce::enforce_range_unsigned_short(
                    n,
                    "Failed to construct 'Response'",
                )?;
                if !(200..=599).contains(&code) {
                    return Err(VmError::range_error(format!(
                        "Failed to construct 'Response': The status provided ({code}) is outside the range [200, 599]."
                    )));
                }
                Some(code)
            };
            let status_text_sid = match status_text_val {
                JsValue::Undefined => default_status_text,
                other => {
                    let sid = super::super::super::coerce::to_string(ctx.vm, other)?;
                    // WHATWG §5.5 statusText must match HTTP reason-phrase
                    // grammar (ASCII without CR/LF/NUL).  Phase 2 only
                    // rejects the obvious CR/LF/NUL case to match the
                    // spec's normative error path.
                    let raw = ctx.vm.strings.get_utf8(sid);
                    if raw.bytes().any(|b| matches!(b, 0x00 | 0x0A | 0x0D)) {
                        return Err(VmError::type_error(
                            "Failed to construct 'Response': Invalid statusText",
                        ));
                    }
                    sid
                }
            };
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                other => Some(other),
            };
            Ok((status, status_text_sid, headers_override))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Response': init must be an object",
        )),
    }
}
