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
    // [`split_host_port`] returns `None` for inputs that the
    // WHATWG basic URL parser would validation-error on
    // (bracketed IPv6 with trailing garbage like `[::1]abc`,
    // non-bracketed multi-colon like `example.com:1:2`).  Those
    // must leave the URL unchanged — the underlying
    // `url::Url::set_host` is too lenient (it silently truncates
    // at the first `:`), so the strict pre-check here is the
    // only thing keeping the WHATWG "invalid host = no-op"
    // contract honest.
    let Some((host_part, port_part)) = split_host_port(&val) else {
        return Ok(JsValue::Undefined);
    };
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        // Gate the port half on host-parse success so an invalid
        // host can't partially mutate the URL by clearing or
        // rewriting the port.
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

/// Split a `host[:port]` string into `(host, Option<port>)` per
/// the WHATWG URL §6.1 "host setter" expectations.  Returns
/// `None` for inputs that the WHATWG basic URL parser would
/// validation-error on, leaving the caller responsible for
/// no-oping the assignment:
///
/// - **Bracketed IPv6 with trailing garbage**: `[::1]abc`,
///   `[::1]]:80`.  The opening `[` requires a matching `]`
///   immediately followed by either `:port` or end-of-string;
///   anything else is invalid.
/// - **Non-bracketed multi-colon**: `example.com:1:2`.  Without
///   brackets there can be at most one `:` separator (host vs
///   port).  `url::Url::set_host` is too lenient here — it
///   silently truncates at the first `:` rather than rejecting
///   — so this strict pre-check is required to honour the
///   WHATWG "invalid host = no-op" contract.
/// - **Non-digit / overflow port**: `host:not`, `host:99999`.
///   WHATWG basic URL parser port-state-with-override rejects
///   non-ASCII-digit characters and values outside `0..=65535`
///   as a validation error, returning failure for the whole
///   assignment.  Empty port (trailing `:`) is allowed and
///   clears the port.
///
/// Successful return shape: `("[ipv6]", Some("8080"))` for
/// `[::1]:8080`; `("host", None)` for `host`; `("host", Some(""))`
/// for `host:` (trailing `:` clears the port per port-state with
/// state-override).
fn split_host_port(val: &str) -> Option<(String, Option<String>)> {
    let (host, port): (String, Option<String>) = if val.starts_with('[') {
        // Bracketed IPv6 — must be `[…]` or `[…]:port`.
        let end = val.find(']')?;
        let host = val[..=end].to_owned();
        let rest = &val[end + 1..];
        match rest {
            "" => (host, None),
            _ => match rest.strip_prefix(':') {
                Some(p) => (host, Some(p.to_owned())),
                // Trailing garbage after `]` (e.g. `[::1]abc`,
                // `[::1]]:80`) — invalid host.
                None => return None,
            },
        }
    } else {
        // Non-bracketed: at most one `:` separator.  `splitn(3,
        // ':')` surfaces a third part exactly when the input has
        // more than one `:` — that's a multi-colon input which
        // the WHATWG host parser rejects (and `url::Url::set_host`
        // would silently accept by truncating).
        let mut parts = val.splitn(3, ':');
        let h = parts
            .next()
            .expect("splitn always yields at least one part");
        let p = parts.next();
        if parts.next().is_some() {
            return None;
        }
        (h.to_owned(), p.map(str::to_owned))
    };
    // Port string validation: empty (clear-port) is allowed;
    // non-empty must parse as `u16` (digit-only AND ≤ 65535).
    // `url::Url::set_port` would silently drop an invalid port
    // here, leaving the host already-applied and the port
    // untouched — a partial mutation the WHATWG host setter
    // forbids (validation error in port-state returns failure
    // for the whole assignment).
    if let Some(ref p) = port {
        if !p.is_empty() && p.parse::<u16>().is_err() {
            return None;
        }
    }
    Some((host, port))
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
