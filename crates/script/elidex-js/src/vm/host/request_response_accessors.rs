//! `Request` / `Response` prototype accessors + `clone` methods.
//!
//! Split out of `request_response.rs` to keep each file under the
//! project's 1000-line convention.  This file only holds the
//! per-instance IDL getter bodies plus the two `clone()`
//! implementations; enums, state structs, constructors, and
//! static factories live in the sibling `request_response`
//! module.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::headers::HeadersGuard;
use super::request_response::{
    copy_headers_entries, RedirectMode, RequestCache, RequestCredentials, RequestMode,
    RequestState, ResponseState, ResponseType,
};

// ---------------------------------------------------------------------------
// Request accessors
// ---------------------------------------------------------------------------

pub(super) fn require_request_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Request.prototype.{method} called on non-Request"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::Request) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "Request.prototype.{method} called on non-Request"
        )))
    }
}

pub(super) fn native_request_get_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "method")?;
    let sid = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(ctx.vm.well_known.http_get, |s| s.method_sid);
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_get_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "url")?;
    let sid = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(ctx.vm.well_known.empty, |s| s.url_sid);
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_get_headers(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "headers")?;
    let headers_id = ctx.vm.request_states.get(&id).map(|s| s.headers_id);
    match headers_id {
        Some(h) => Ok(JsValue::Object(h)),
        None => Ok(JsValue::Null),
    }
}

pub(super) fn native_request_get_body(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: non-streaming — the `.body` getter always returns
    // `null`.  Body bytes remain retrievable via the Body mixin
    // (`.text()` / `.json()` / `.arrayBuffer()` / `.blob()` —
    // see `body_mixin.rs`).  Fetch spec technically types `.body`
    // as `ReadableStream?`; the Phase-2 `null` fallback is
    // intentional until `ReadableStream` is implemented (planned
    // in the later PR5-streams tranche of the M4-12 boa → VM
    // cutover).
    Ok(JsValue::Null)
}

pub(super) fn native_request_get_body_used(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "bodyUsed")?;
    Ok(JsValue::Boolean(ctx.vm.body_used.contains(&id)))
}

pub(super) fn native_request_get_redirect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "redirect")?;
    let mode = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(RedirectMode::Follow, |s| s.redirect);
    let sid = ctx.vm.strings.intern(match mode {
        RedirectMode::Follow => "follow",
        RedirectMode::Error => "error",
        RedirectMode::Manual => "manual",
    });
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_get_mode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "mode")?;
    let mode = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(RequestMode::Cors, |s| s.mode);
    let sid = ctx.vm.strings.intern(match mode {
        RequestMode::Cors => "cors",
        RequestMode::NoCors => "no-cors",
        RequestMode::SameOrigin => "same-origin",
        RequestMode::Navigate => "navigate",
    });
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_get_credentials(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "credentials")?;
    let c = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(RequestCredentials::SameOrigin, |s| s.credentials);
    let sid = ctx.vm.strings.intern(match c {
        RequestCredentials::Omit => "omit",
        RequestCredentials::SameOrigin => "same-origin",
        RequestCredentials::Include => "include",
    });
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_get_cache(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "cache")?;
    let c = ctx
        .vm
        .request_states
        .get(&id)
        .map_or(RequestCache::Default, |s| s.cache);
    let sid = ctx.vm.strings.intern(match c {
        RequestCache::Default => "default",
        RequestCache::NoStore => "no-store",
        RequestCache::Reload => "reload",
        RequestCache::NoCache => "no-cache",
        RequestCache::ForceCache => "force-cache",
        RequestCache::OnlyIfCached => "only-if-cached",
    });
    Ok(JsValue::String(sid))
}

pub(super) fn native_request_clone(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "clone")?;
    // Spec §5.3 step "clone a request": a cloned body is not
    // permitted if `bodyUsed === true`.  Phase 2 observes this
    // guard even though the Body mixin isn't yet exposed.
    if ctx.vm.body_used.contains(&id) {
        return Err(VmError::type_error(
            "Failed to execute 'clone' on 'Request': Request body is already used",
        ));
    }
    let (method_sid, url_sid, src_headers_id, redirect, mode, credentials, cache) = {
        let state = ctx
            .vm
            .request_states
            .get(&id)
            .expect("Request without request_states entry");
        (
            state.method_sid,
            state.url_sid,
            state.headers_id,
            state.redirect,
            state.mode,
            state.credentials,
            state.cache,
        )
    };
    let body = ctx.vm.body_data.get(&id).map(Arc::clone);
    let new_headers = ctx.vm.create_headers(HeadersGuard::None);
    // Root `new_headers` across the subsequent allocations — the
    // `copy_headers_entries` entry-splice path and the cloned
    // Request's `alloc_object` can each trigger GC, and
    // `new_headers` is only reachable from a Rust local until
    // `request_states.insert` links it into the cloned Request's
    // state below.  Same defensive invariant as R10 / R13 / R16:
    // `alloc_object`'s contract demands caller-side rooting of
    // any `ObjectId` reachable only via a local.  Unreachable
    // today (`gc_enabled = false` inside natives) but preserved
    // uniformly.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(new_headers));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    copy_headers_entries(ctx, src_headers_id, new_headers);
    // Propagate the source guard so a cloned Request built from
    // an immutable-companion (extremely unusual — only happens
    // when a future tightening flips this to `request` guard with
    // forbidden-header enforcement) stays immutable.
    if let Some(src_guard) = ctx.vm.headers_states.get(&src_headers_id).map(|s| s.guard) {
        if let Some(dst) = ctx.vm.headers_states.get_mut(&new_headers) {
            dst.guard = src_guard;
        }
    }

    let proto = ctx.vm.request_prototype;
    let new_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Request,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    if let Some(bytes) = body {
        ctx.vm.body_data.insert(new_id, bytes);
    }
    ctx.vm.request_states.insert(
        new_id,
        RequestState {
            method_sid,
            url_sid,
            headers_id: new_headers,
            redirect,
            mode,
            credentials,
            cache,
        },
    );
    Ok(JsValue::Object(new_id))
}

// ---------------------------------------------------------------------------
// Response accessors
// ---------------------------------------------------------------------------

pub(super) fn require_response_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Response.prototype.{method} called on non-Response"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::Response) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "Response.prototype.{method} called on non-Response"
        )))
    }
}

pub(super) fn native_response_get_status(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "status")?;
    let status = ctx.vm.response_states.get(&id).map_or(200, |s| s.status);
    Ok(JsValue::Number(f64::from(status)))
}

pub(super) fn native_response_get_ok(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "ok")?;
    let status = ctx.vm.response_states.get(&id).map_or(200, |s| s.status);
    Ok(JsValue::Boolean((200..300).contains(&status)))
}

pub(super) fn native_response_get_status_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "statusText")?;
    let sid = ctx
        .vm
        .response_states
        .get(&id)
        .map_or(ctx.vm.well_known.empty, |s| s.status_text_sid);
    Ok(JsValue::String(sid))
}

pub(super) fn native_response_get_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "url")?;
    let sid = ctx
        .vm
        .response_states
        .get(&id)
        .map_or(ctx.vm.well_known.empty, |s| s.url_sid);
    Ok(JsValue::String(sid))
}

pub(super) fn native_response_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "type")?;
    let ty = ctx
        .vm
        .response_states
        .get(&id)
        .map_or(ResponseType::Default, |s| s.response_type);
    let wk = &ctx.vm.well_known;
    let sid = match ty {
        ResponseType::Basic => wk.response_type_basic,
        ResponseType::Cors => wk.response_type_cors,
        ResponseType::Default => wk.response_type_default,
        ResponseType::Error => wk.response_type_error,
        ResponseType::Opaque => wk.response_type_opaque,
        ResponseType::OpaqueRedirect => wk.response_type_opaqueredirect,
    };
    Ok(JsValue::String(sid))
}

pub(super) fn native_response_get_headers(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "headers")?;
    let headers_id = ctx.vm.response_states.get(&id).map(|s| s.headers_id);
    match headers_id {
        Some(h) => Ok(JsValue::Object(h)),
        None => Ok(JsValue::Null),
    }
}

pub(super) fn native_response_get_body(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: see `native_request_get_body` for the rationale.
    Ok(JsValue::Null)
}

pub(super) fn native_response_get_body_used(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "bodyUsed")?;
    Ok(JsValue::Boolean(ctx.vm.body_used.contains(&id)))
}

pub(super) fn native_response_get_redirected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "redirected")?;
    Ok(JsValue::Boolean(
        ctx.vm
            .response_states
            .get(&id)
            .is_some_and(|s| s.redirected),
    ))
}

pub(super) fn native_response_clone(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_response_this(ctx, this, "clone")?;
    if ctx.vm.body_used.contains(&id) {
        return Err(VmError::type_error(
            "Failed to execute 'clone' on 'Response': Response body is already used",
        ));
    }
    let (status, status_text_sid, url_sid, src_headers_id, response_type, redirected) = {
        let state = ctx
            .vm
            .response_states
            .get(&id)
            .expect("Response without response_states entry");
        (
            state.status,
            state.status_text_sid,
            state.url_sid,
            state.headers_id,
            state.response_type,
            state.redirected,
        )
    };
    let body = ctx.vm.body_data.get(&id).map(Arc::clone);

    // New companion Headers: start mutable, splice source
    // entries, flip to Immutable to match the original.
    let new_headers = ctx.vm.create_headers(HeadersGuard::None);
    // Root `new_headers` across the splice + clone-alloc window
    // (R16 GC-safety invariant — mirrors `native_request_clone`).
    let mut g = ctx.vm.push_temp_root(JsValue::Object(new_headers));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    copy_headers_entries(ctx, src_headers_id, new_headers);
    if let Some(src_guard) = ctx.vm.headers_states.get(&src_headers_id).map(|s| s.guard) {
        if let Some(dst) = ctx.vm.headers_states.get_mut(&new_headers) {
            dst.guard = src_guard;
        }
    }

    let proto = ctx.vm.response_prototype;
    let new_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    if let Some(bytes) = body {
        ctx.vm.body_data.insert(new_id, bytes);
    }
    ctx.vm.response_states.insert(
        new_id,
        ResponseState {
            status,
            status_text_sid,
            url_sid,
            headers_id: new_headers,
            response_type,
            redirected,
        },
    );
    Ok(JsValue::Object(new_id))
}
