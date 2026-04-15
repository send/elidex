//! `location` global — a subset of the `Location` interface
//! (WHATWG HTML §7.1).
//!
//! # Phase 2 scope
//!
//! - All getters (`href`, `protocol`, `host`, `hostname`, `port`,
//!   `pathname`, `search`, `hash`, `origin`) read from
//!   [`VmInner::navigation`].
//! - Setters (`href`, `assign`, `replace`) update `current_url` and
//!   the history stack **in memory only** — no shell-side navigation
//!   fires yet.  PR6 wires up the real load.
//! - `reload()` is a silent no-op for the same reason.
//!
//! # URL parsing
//!
//! Phase 2 uses an intentionally minimal, dependency-free parser
//! ([`parse_url`]).  It is *not* a WHATWG URL parser — the `url`
//! crate lands in PR5 alongside the network stack, and Location's
//! getters get rewritten against it at that point.
//!
//! Recognised shapes:
//! - `scheme://host[:port]/pathname[?search][#hash]` (any scheme —
//!   the splitter is scheme-agnostic).
//! - `scheme:rest` — scheme-only URIs (`about:blank`, `data:text/...`)
//!   keep the remainder as the pathname; host / port stay empty.
//!
//! [`format_origin`] adopts the HTML §7.2.3 origin tuple for the
//! "special schemes" set (`http` / `https` / `ws` / `wss` / `ftp`)
//! and returns `"null"` (opaque origin) for every other scheme,
//! including `file:` and `about:` — so `location.origin` at
//! `about:blank` is `"null"`.  IDN, percent-encoding, IPv6 hosts,
//! and `file://` host-vs-path nuances are intentionally not
//! handled here; they arrive with the `url` crate in PR5.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;

/// Parsed URL components produced by [`parse_url`].
#[derive(Default)]
struct UrlParts<'a> {
    scheme: &'a str,
    host: &'a str,
    port: &'a str,
    pathname: &'a str,
    search: &'a str,
    hash: &'a str,
}

/// Minimal URL splitter — see the module-level docs for its
/// scope.  Returns empty strings for components that are absent.
///
/// Shape handled:
/// ```text
/// scheme "://" host [":" port] pathname ["?" search] ["#" hash]
/// ```
///
/// For schemes without an authority (`about:`, `data:` fallback), the
/// remainder (everything after the colon) becomes the pathname and the
/// host / port slots stay empty.
fn parse_url(url: &str) -> UrlParts<'_> {
    let mut parts = UrlParts::default();
    // Split off #hash first — hash is defined to come after everything
    // (including the query) and `#` in query/authority is illegal.
    let (before_hash, hash) = match url.find('#') {
        Some(idx) => (&url[..idx], &url[idx..]),
        None => (url, ""),
    };
    parts.hash = hash;

    // Split off ?search.
    let (before_search, search) = match before_hash.find('?') {
        Some(idx) => (&before_hash[..idx], &before_hash[idx..]),
        None => (before_hash, ""),
    };
    parts.search = search;

    // scheme:rest
    let (scheme, rest) = if let Some(idx) = before_search.find(':') {
        (&before_search[..idx], &before_search[idx + 1..])
    } else {
        parts.pathname = before_search;
        return parts;
    };
    parts.scheme = scheme;

    if let Some(after_slashes) = rest.strip_prefix("//") {
        // scheme:// → host[:port] pathname.  Per WHATWG URL §4.4,
        // absolute URLs with an authority but no explicit path
        // segment (`https://example.com`) have an effective pathname
        // of `/`, not the empty string — `location.pathname` returns
        // `/` in every conforming browser.
        let authority = if let Some(path_start) = after_slashes.find('/') {
            parts.pathname = &after_slashes[path_start..];
            &after_slashes[..path_start]
        } else {
            parts.pathname = "/";
            after_slashes
        };
        if let Some(colon) = authority.rfind(':') {
            parts.host = &authority[..colon];
            parts.port = &authority[colon + 1..];
        } else {
            parts.host = authority;
        }
    } else {
        // scheme-only URLs (`about:blank`, `data:text/...`, …) keep
        // the remainder as the pathname.
        parts.pathname = rest;
    }
    parts
}

/// Compute the `host` serialization: `hostname[":"port]`.
fn format_host(parts: &UrlParts<'_>) -> String {
    if parts.port.is_empty() {
        parts.host.to_string()
    } else {
        format!("{}:{}", parts.host, parts.port)
    }
}

/// Compute `origin` per HTML §7.2.3 "the origin of a URL".
fn format_origin(parts: &UrlParts<'_>) -> String {
    match parts.scheme {
        "http" | "https" | "ws" | "wss" | "ftp" => {
            let mut s = format!("{}://{}", parts.scheme, parts.host);
            if !parts.port.is_empty() {
                s.push(':');
                s.push_str(parts.port);
            }
            s
        }
        _ => "null".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn read_url(ctx: &NativeContext<'_>) -> String {
    ctx.vm.navigation.current_url.clone()
}

fn intern_current(ctx: &mut NativeContext<'_>, s: &str) -> JsValue {
    let sid = ctx.vm.strings.intern(s);
    JsValue::String(sid)
}

pub(super) fn native_location_get_href(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let url = read_url(ctx);
    Ok(intern_current(ctx, &url))
}

/// Shared setter body — used by `href = …`, `assign(url)`, and
/// `replace(url)`.  `replace_history` controls whether the mutation
/// pushes a new history entry (`assign` / `href=`) or overwrites the
/// current one (`replace`).
fn set_location(ctx: &mut NativeContext<'_>, new_url: &str, replace_history: bool) {
    let nav = &mut ctx.vm.navigation;
    nav.current_url = new_url.to_string();
    if replace_history {
        // `history_entries` is non-empty by `NavigationState::new`
        // invariant and every code path preserves it — index directly
        // rather than `get_mut`.
        debug_assert!(!nav.history_entries.is_empty());
        nav.history_entries[nav.history_index]
            .url
            .clone_from(&nav.current_url);
    } else {
        nav.push_entry(new_url.to_string(), JsValue::Null);
    }
}

pub(super) fn native_location_set_href(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let url = ctx.vm.strings.get_utf8(sid);
    set_location(ctx, &url, false);
    Ok(JsValue::Undefined)
}

/// Shared body for every URL-component getter: read `current_url`,
/// parse, apply `extract`, intern the result.  Native fn wrappers
/// below differ only in which slice of `UrlParts` they pluck, so
/// passing a closure is the clearest way to share the scaffolding
/// (no macro needed — regular Rust syntax handles it).
fn url_component(
    ctx: &mut NativeContext<'_>,
    extract: impl FnOnce(&UrlParts<'_>) -> String,
) -> Result<JsValue, VmError> {
    let url = read_url(ctx);
    let s = extract(&parse_url(&url));
    Ok(intern_current(ctx, &s))
}

pub(super) fn native_location_get_protocol(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| {
        if p.scheme.is_empty() {
            String::new()
        } else {
            format!("{}:", p.scheme)
        }
    })
}

pub(super) fn native_location_get_host(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, format_host)
}

pub(super) fn native_location_get_hostname(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| p.host.to_string())
}

pub(super) fn native_location_get_port(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| p.port.to_string())
}

pub(super) fn native_location_get_pathname(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| p.pathname.to_string())
}

pub(super) fn native_location_get_search(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| p.search.to_string())
}

pub(super) fn native_location_get_hash(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |p| p.hash.to_string())
}

pub(super) fn native_location_get_origin(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, format_origin)
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

pub(super) fn native_location_assign(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let url = ctx.vm.strings.get_utf8(sid);
    set_location(ctx, &url, false);
    Ok(JsValue::Undefined)
}

pub(super) fn native_location_replace(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let url = ctx.vm.strings.get_utf8(sid);
    set_location(ctx, &url, true);
    Ok(JsValue::Undefined)
}

pub(super) fn native_location_reload(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: no shell-side reload.  PR6 wires this up to the
    // network / rendering stack.  The web-visible behaviour is that
    // the JS observable state (scripts, variables) is unchanged,
    // which is what "no-op reload stub" means.
    Ok(JsValue::Undefined)
}

pub(super) fn native_location_to_string(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Spec: `Location.prototype.toString` returns the URL (same as
    // `href`'s getter).
    native_location_get_href(ctx, JsValue::Undefined, &[])
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `globalThis.location` (WHATWG HTML §7.1).
    ///
    /// The object is an ordinary plain object with accessors + methods;
    /// it does **not** live on an ECS entity because Location is not
    /// an EventTarget in Phase 2 scope.  (The spec defines `Location`
    /// as a distinct non-EventTarget interface; a future change that
    /// adds events on Location would upgrade this to a `HostObject`.)
    pub(in crate::vm) fn register_location_global(&mut self) {
        let obj_id = self.create_object_with_methods(LOCATION_METHODS);

        // `href` — RW accessor.  `writable` is irrelevant for accessors
        // but the structure requires a value; WebIDL defaults to
        // `{enumerable, configurable}`.
        let rw_attrs = shape::PropertyAttrs {
            writable: false,
            enumerable: true,
            configurable: true,
            is_accessor: true,
        };
        let gid_href = self.create_native_function("get href", native_location_get_href);
        let sid_href = self.create_native_function("set href", native_location_set_href);
        let href_key = PropertyKey::String(self.strings.intern("href"));
        self.define_shaped_property(
            obj_id,
            href_key,
            PropertyValue::Accessor {
                getter: Some(gid_href),
                setter: Some(sid_href),
            },
            rw_attrs,
        );

        self.install_ro_accessors(obj_id, LOCATION_RO_ACCESSORS);

        let name = self.well_known.location;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}

const LOCATION_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("assign", native_location_assign),
    ("replace", native_location_replace),
    ("reload", native_location_reload),
    ("toString", native_location_to_string),
];

const LOCATION_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("protocol", native_location_get_protocol),
    ("host", native_location_get_host),
    ("hostname", native_location_get_hostname),
    ("port", native_location_get_port),
    ("pathname", native_location_get_pathname),
    ("search", native_location_get_search),
    ("hash", native_location_get_hash),
    ("origin", native_location_get_origin),
];
