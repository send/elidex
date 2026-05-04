//! Static factory methods on the `Response` constructor:
//! `Response.error()` (§5.5.6), `Response.redirect()` (§5.5.7),
//! and `Response.json()` (§5.5.8 / ES2023).
//!
//! Split from [`super`] (`request_response/mod.rs`) to keep each
//! file under the project's 1000-line convention.  All three
//! statics share the same Response allocation pattern; only the
//! status code, headers, and body bytes differ.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError,
};
use super::super::headers::HeadersGuard;
use super::response_ctor::build_response_instance;
use super::{parse_url, ResponseState, ResponseType};

/// `Response.error()` (WHATWG §5.5.6).  Network-error response —
/// `status === 0`, `type === "error"`, immutable empty headers.
pub(super) fn native_response_static_error(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Allocate a raw Response instance (not via `new Response()`
    // because the ctor rejects status 0 → "outside [200, 599]").
    //
    // Root `inst_id` across the subsequent `create_headers` call:
    // the new Response is reachable only via this Rust local until
    // `response_states.insert(...)` links it at the end of the
    // function, and `create_headers` itself allocates an object
    // that can trigger GC under a future refactor (R18-audit).
    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst_id));
    let headers_id = g.create_headers(HeadersGuard::Immutable);
    let empty_sid = g.well_known.empty;
    g.response_states.insert(
        inst_id,
        ResponseState {
            status: 0,
            status_text_sid: empty_sid,
            url_sid: empty_sid,
            headers_id,
            response_type: ResponseType::Error,
            redirected: false,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// `Response.redirect(url, status?)` (WHATWG §5.5.7).
pub(super) fn native_response_static_redirect(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to execute 'redirect' on 'Response': 1 argument required, but only 0 present.",
        ));
    }
    let url_sid = super::super::super::coerce::to_string(ctx.vm, args[0])?;
    let raw_url = ctx.vm.strings.get_utf8(url_sid);
    let url = parse_url(ctx.vm, &raw_url).map_err(|_| {
        VmError::type_error(format!(
            "Failed to execute 'redirect' on 'Response': Invalid URL '{raw_url}'"
        ))
    })?;
    let abs_url_sid = ctx.vm.strings.intern(url.as_str());

    let status = if let Some(s) = args.get(1).copied() {
        if matches!(s, JsValue::Undefined) {
            302
        } else {
            // WebIDL `[EnforceRange] unsigned short` — NaN / ±∞ /
            // out-of-[0,65535] is TypeError, the subsequent
            // redirect-code membership check is RangeError.
            let n = super::super::super::coerce::to_number(ctx.vm, s)?;
            let code = super::super::super::coerce::enforce_range_unsigned_short(
                n,
                "Failed to execute 'redirect' on 'Response'",
            )?;
            if !matches!(code, 301 | 302 | 303 | 307 | 308) {
                return Err(VmError::range_error(format!(
                    "Failed to execute 'redirect' on 'Response': Invalid status code {code}"
                )));
            }
            code
        }
    } else {
        302
    };

    // Root `inst_id` across `create_headers` (which allocates an
    // object and would otherwise collect the newly-allocated
    // Response under a future GC-enabled refactor).  `strings
    // .intern` + `headers_states` mutation + `well_known` access
    // after `create_headers` are alloc-free, so `headers_id`
    // itself reaches `response_states` without a separate root
    // (R18-audit).
    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst_id));
    let headers_id = g.create_headers(HeadersGuard::None);
    let location_name = g.strings.intern("location");
    if let Some(state) = g.headers_states.get_mut(&headers_id) {
        state.list.push((location_name, abs_url_sid));
        state.guard = HeadersGuard::Immutable;
    }
    let empty_sid = g.well_known.empty;
    g.response_states.insert(
        inst_id,
        ResponseState {
            status,
            status_text_sid: empty_sid,
            url_sid: empty_sid,
            headers_id,
            // WHATWG Fetch §5.5 step 7: `Response.redirect(...)`
            // produces an opaque-redirect response.  `type` must
            // therefore expose `"opaqueredirect"`; not `"default"`.
            response_type: ResponseType::OpaqueRedirect,
            redirected: false,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// `Response.json(data, init?)` (WHATWG §5.5.8, ES2023
/// addition).  Stringifies `data` via `JSON.stringify`, uses the
/// result as the body, and sets `Content-Type:
/// application/json`.
pub(super) fn native_response_static_json(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Defer to `native_json_stringify` for the serialisation —
    // re-using the spec-compliant path keeps us in sync with
    // `JSON.stringify` semantics (cycle detection, replacer
    // fn / list, Number/BigInt / toJSON etc.).
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let json_val =
        super::super::super::natives_json::native_json_stringify(ctx, JsValue::Undefined, &[data])?;
    let body_bytes = match json_val {
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Some(raw.into_bytes())
        }
        _ => {
            // `JSON.stringify(undefined)` → `undefined` → body is
            // absent.  Matches browsers which pass through
            // undefined literally: `Response.json(undefined).text()`
            // resolves to `""`.
            None
        }
    };
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Root `inst_id` across `build_response_instance` — the helper
    // calls `parse_response_init` (which may run user-supplied
    // getters whose bodies allocate) and `create_headers` (which
    // allocates), and its contract assumes `inst_id` is already
    // rooted by the caller (see its inline comment at the
    // `headers_id` allocation site).  Matches the rooting pattern
    // used by `native_response_static_error` /
    // `native_response_static_redirect` above.  Latent today
    // because `gc_enabled = false` inside natives, but kept
    // uniform so a future GC-enabled native path doesn't surface
    // a use-after-free here.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst_id));
    let mut rooted_holder = super::super::super::value::NativeContext { vm: &mut g };
    let ctx = &mut rooted_holder;
    let json_ct = ctx.vm.well_known.application_json_utf8;
    build_response_instance(
        ctx,
        inst_id,
        body_bytes,
        Some(json_ct),
        init,
        ResponseType::Default,
        0,
        false,
    )?;
    Ok(JsValue::Object(inst_id))
}
