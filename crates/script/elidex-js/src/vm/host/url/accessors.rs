//! `URL.prototype` IDL accessor getters (WHATWG URL §6.1).
//!
//! Split out of [`super`] so the parent module stays under the
//! project's 1000-line convention.  Each function is the read half
//! of an IDL accessor pair installed by
//! [`super::VmInner::install_url_members`]; the matching write
//! halves live in [`super::setters`].
//!
//! All functions delegate the formatting + intern dance to
//! [`super::url_component`], which holds the shared GC-safe
//! borrow-then-intern pattern.  The brand check
//! [`super::require_url_this`] runs first and converts the
//! receiver to a `URL`-typed `ObjectId` before any state read.

use super::super::super::value::{JsValue, NativeContext, VmError};
use super::{require_url_this, url_component};

pub(super) fn native_url_get_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "href")?;
    url_component(ctx, id, |u| u.as_str().to_string())
}

/// `URL.prototype.origin` (WHATWG URL §6.1).  Read-only.  Returns
/// the ASCII serialisation of the URL's origin tuple — for opaque
/// origins (`file:` / `data:` / `blob:` / `about:`) this is the
/// literal `"null"` string.
pub(super) fn native_url_get_origin(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "origin")?;
    url_component(ctx, id, |u| u.origin().ascii_serialization())
}

pub(super) fn native_url_get_protocol(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "protocol")?;
    // Spec mandates the trailing `:` (WHATWG URL §6.1 "protocol getter").
    url_component(ctx, id, |u| format!("{}:", u.scheme()))
}

pub(super) fn native_url_get_username(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "username")?;
    url_component(ctx, id, |u| u.username().to_string())
}

pub(super) fn native_url_get_password(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "password")?;
    url_component(ctx, id, |u| u.password().unwrap_or("").to_string())
}

/// `host` IDL attribute — `hostname[:port]`.  Port is omitted when
/// the URL has no explicit port (default-port stripping is handled
/// by `url::Url` at parse time).
pub(super) fn native_url_get_host(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "host")?;
    url_component(ctx, id, |u| match (u.host_str(), u.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_string(),
        (None, _) => String::new(),
    })
}

pub(super) fn native_url_get_hostname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hostname")?;
    url_component(ctx, id, |u| u.host_str().unwrap_or("").to_string())
}

pub(super) fn native_url_get_port(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "port")?;
    url_component(ctx, id, |u| {
        u.port().map_or(String::new(), |p| p.to_string())
    })
}

pub(super) fn native_url_get_pathname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "pathname")?;
    url_component(ctx, id, |u| u.path().to_string())
}

/// `search` IDL attribute — `?`-prefixed query when non-empty,
/// empty string when the query is absent or empty (matches WHATWG
/// URL §6.1 "search getter").
pub(super) fn native_url_get_search(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "search")?;
    url_component(ctx, id, |u| match u.query() {
        Some(q) if !q.is_empty() => format!("?{q}"),
        _ => String::new(),
    })
}

/// `searchParams` IDL attribute (read-only) — return the linked
/// `URLSearchParams` `ObjectId` allocated eagerly by the
/// constructor (Phase 4 of slot #9.5).  WHATWG URL §6.1 mandates
/// that `url.searchParams === url.searchParams` holds, so the same
/// `ObjectId` is returned on every access.
pub(super) fn native_url_get_search_params(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "searchParams")?;
    let sp = ctx
        .vm
        .url_states
        .get(&id)
        .and_then(|state| state.search_params);
    match sp {
        Some(sp_id) => Ok(JsValue::Object(sp_id)),
        // The constructor always populates `search_params`, so the
        // `None` branch is only reachable if a downstream caller
        // synthesises a `URL` instance without going through
        // `native_url_constructor`.  Treat as a defensive
        // TypeError rather than silently returning undefined —
        // matches the brand-check intent.
        None => Err(VmError::type_error(
            "URL.prototype.searchParams: missing internal slot",
        )),
    }
}

/// `hash` IDL attribute — `#`-prefixed fragment when non-empty,
/// empty string when absent or empty (WHATWG URL §6.1 "hash getter").
pub(super) fn native_url_get_hash(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hash")?;
    url_component(ctx, id, |u| match u.fragment() {
        Some(f) if !f.is_empty() => format!("#{f}"),
        _ => String::new(),
    })
}
