//! `URL.prototype` IDL accessor setters (WHATWG URL §6.1).
//!
//! Split out of [`super`] so the parent module stays under the
//! project's 1000-line convention.  Each function is the write
//! half of an IDL accessor pair installed by
//! [`super::VmInner::install_url_members`]; the matching read
//! halves live in [`super::accessors`].
//!
//! Setters share two helpers private to this module:
//! [`take_url_setter_arg`] (argument coercion) and
//! [`split_host_port`] (parsing `host[:port]` for the `host`
//! setter, with bracketed-IPv6 awareness).  The cross-module
//! [`super::rebuild_linked_search_params`] is invoked by the
//! `href` and `search` setters to refresh the linked
//! `URLSearchParams` entry list after a query mutation.

use super::super::super::value::{JsValue, NativeContext, VmError};
use super::{rebuild_linked_search_params, require_url_this};

/// `url.href = …` — full re-parse.  Throws `TypeError` when the
/// new value does not parse as an absolute URL (matches V8 / Firefox
/// 2023+; the WHATWG spec is to silently ignore but every browser
/// throws here, and boa-side throws too).  Re-parse refreshes the
/// linked `URLSearchParams` entry list (Phase 4 of slot #9.5).
pub(super) fn native_url_set_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "href")?;
    let val = take_url_setter_arg(ctx, args)?;
    let parsed = url::Url::parse(&val)
        .map_err(|_| VmError::type_error(format!("URL: invalid URL: {val}")))?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        state.url = parsed;
    }
    rebuild_linked_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `url.protocol = …` — strip a single trailing `:` then call
/// `url::Url::set_scheme`.  WHATWG URL §6.1 silently ignores
/// scheme changes that violate the URL parser invariants
/// (mismatched special / non-special schemes); we mirror that by
/// discarding the `Result<(), ()>` return.
pub(super) fn native_url_set_protocol(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "protocol")?;
    let val = take_url_setter_arg(ctx, args)?;
    let scheme = val.strip_suffix(':').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_scheme(&scheme);
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_url_set_username(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "username")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_username(&val);
    }
    Ok(JsValue::Undefined)
}

/// `url.password = …` — empty string clears the password
/// (WHATWG URL §6.1 "password setter": `Some(empty)` would emit
/// the trailing `:` separator, so map to `None`).
pub(super) fn native_url_set_password(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "password")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let pw = if val.is_empty() {
            None
        } else {
            Some(val.as_str())
        };
        let _ = state.url.set_password(pw);
    }
    Ok(JsValue::Undefined)
}

/// `url.host = …` — WHATWG URL §6.1 "host setter" parses the
/// input as `host[:port]`.  The `url` crate's `set_host` only
/// understands the host portion (it accepts bracketed IPv6 like
/// `[::1]` but not the trailing `:port`), so [`split_host_port`]
/// peels the port off correctly for both bracketed IPv6 and
/// regular hostnames.  An explicit empty port (trailing `:`)
/// clears the existing port — matching the WHATWG basic URL
/// parser's port state with state-override.
pub(super) fn native_url_set_host(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "host")?;
    let val = take_url_setter_arg(ctx, args)?;
    let (host_part, port_part) = split_host_port(&val);
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        // WHATWG URL §6.1 host setter — basic URL parser in
        // host-state-with-override returns failure on an invalid
        // host and leaves the URL unchanged.  Gate the port half
        // on host-parse success so an invalid host can't partially
        // mutate the URL by clearing or rewriting the port.
        if state.url.set_host(Some(&host_part)).is_ok() {
            if let Some(p) = port_part {
                if p.is_empty() {
                    // Trailing `:` with empty buffer in port state
                    // clears the port (WHATWG basic URL parser §4.4
                    // "port state" with state override).
                    let _ = state.url.set_port(None);
                } else if let Ok(parsed_port) = p.parse::<u16>() {
                    let _ = state.url.set_port(Some(parsed_port));
                }
                // else: invalid port — silently ignore, matching
                // WHATWG validation-error short-circuit.
            }
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_url_set_hostname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hostname")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_host(Some(&val));
    }
    Ok(JsValue::Undefined)
}

/// `url.port = …` — empty string clears the port; otherwise parse
/// as `u16` and silently ignore parse failures (matches WHATWG URL
/// §6.1 "port setter" which short-circuits on invalid values).
pub(super) fn native_url_set_port(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "port")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if val.is_empty() {
            let _ = state.url.set_port(None);
        } else if let Ok(p) = val.parse::<u16>() {
            let _ = state.url.set_port(Some(p));
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_url_set_pathname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "pathname")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        state.url.set_path(&val);
    }
    Ok(JsValue::Undefined)
}

/// `url.search = …` — strip a single leading `?` then update the
/// query.  Empty value clears the query entirely.  Refreshes the
/// linked `URLSearchParams` entry list so
/// `url.search = "x=1"; url.searchParams.get("x")` returns `"1"`
/// (Phase 4 of slot #9.5).
pub(super) fn native_url_set_search(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "search")?;
    let val = take_url_setter_arg(ctx, args)?;
    let query = val.strip_prefix('?').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if query.is_empty() {
            state.url.set_query(None);
        } else {
            state.url.set_query(Some(&query));
        }
    }
    rebuild_linked_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `url.hash = …` — strip a single leading `#` then update the
/// fragment.  Empty value clears the fragment entirely.
pub(super) fn native_url_set_hash(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hash")?;
    let val = take_url_setter_arg(ctx, args)?;
    let frag = val.strip_prefix('#').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if frag.is_empty() {
            state.url.set_fragment(None);
        } else {
            state.url.set_fragment(Some(&frag));
        }
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Helpers (private to this module)
// ---------------------------------------------------------------------------

/// Split a `host[:port]` string into its component parts per the
/// WHATWG URL §6.1 "host setter" expectations.  Recognises
/// bracketed IPv6 literals so `[::1]:8080` splits to `("[::1]",
/// Some("8080"))` rather than `("[", Some(":1]:8080"))` (which is
/// what a naive `split_once(':')` would produce).
///
/// `Some("")` for the port half encodes a trailing `:` with no
/// digits — the caller maps that to "clear the port" per the
/// basic URL parser's port-state-with-override semantics.
/// `None` means "no port separator at all", which leaves the
/// existing port untouched.
fn split_host_port(val: &str) -> (String, Option<String>) {
    if val.starts_with('[') {
        // Bracketed IPv6 — split after the matching `]`.
        if let Some(end) = val.find(']') {
            let host = val[..=end].to_owned();
            let rest = &val[end + 1..];
            return match rest.strip_prefix(':') {
                Some(port) => (host, Some(port.to_owned())),
                None => (host, None),
            };
        }
        // Malformed bracketed input (no closing `]`) — fall
        // through to the colon-split path; the underlying
        // `set_host` call will validation-error on the malformed
        // host and silently leave the URL unchanged.
    }
    match val.split_once(':') {
        Some((h, p)) => (h.to_owned(), Some(p.to_owned())),
        None => (val.to_owned(), None),
    }
}

/// Coerce the first positional argument to a `String` via
/// [`super::super::super::coerce::to_string`].  Used by every URL
/// setter (the WHATWG IDL declares all these IDL attrs `attribute
/// USVString …` so the receiver-side coercion shape matches
/// `to_string`'s abstract semantics).  An owned `String` is
/// returned because each setter then drops the `&strings` borrow
/// before calling `url_states.get_mut` — overlap on `vm.strings`
/// and `vm.url_states` would conflict with the simultaneous `&mut
/// state.url` operation otherwise.
fn take_url_setter_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<String, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}
