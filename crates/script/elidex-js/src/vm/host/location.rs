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
//! PR5a wires the [`url`] crate in — every getter reads component
//! slices via [`url::Url`] (`scheme()`, `host_str()`, `port()`,
//! `path()`, `query()`, `fragment()`) and setters resolve relative
//! URLs with [`Url::join`] so `location.href = "foo"` against
//! `https://site/a/` lands at `https://site/a/foo`.
//!
//! Canonicalisation notes (inherited from the `url` crate):
//! - Default ports are stripped (`http://host:80/` → `http://host/`).
//! - Host is lowercased (`http://HOST/` → `http://host/`).
//! - Authority-bearing URLs without a path gain a trailing `/`
//!   (`http://host` → `http://host/`).
//! - Percent-encoding is normalised per WHATWG.
//!
//! # Origin
//!
//! `location.origin` mirrors HTML §7.2.3 "origin of a URL".  For
//! schemes whose origin tuple is opaque (`file:`, `about:`, `data:`,
//! `blob:`), the getter returns `"null"` — matching Blink / Gecko.

#![cfg(feature = "engine")]

use url::Url;

use super::super::coerce;
use super::super::shape;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;

/// Resolve `input` against the current document URL.  `Url::join`
/// accepts both absolute and relative inputs per WHATWG URL §4.5 —
/// an absolute input replaces the base, a relative input composes.
/// Returns `None` on parse failure; the caller translates that to a
/// `DOMException("SyntaxError")`.
fn resolve_url(ctx: &NativeContext<'_>, input: &str) -> Option<Url> {
    ctx.vm.navigation.current_url.join(input).ok()
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn intern_current(ctx: &mut NativeContext<'_>, s: &str) -> JsValue {
    let sid = ctx.vm.strings.intern(s);
    JsValue::String(sid)
}

/// Shared body for every URL-component getter: borrow
/// `current_url`, apply `extract` to assemble the output string,
/// intern the result.  Native fn wrappers below differ only in
/// which slice of the [`Url`] they pluck.
fn url_component(
    ctx: &mut NativeContext<'_>,
    extract: impl FnOnce(&Url) -> String,
) -> Result<JsValue, VmError> {
    // Extract produces an owned `String`, so the borrow on
    // `current_url` ends before `intern_current` takes `&mut`.
    // Cloning the URL would cost a full heap allocation (WHATWG
    // `url::Url` is not Arc-backed — the stored serialization is a
    // plain String).
    let s = extract(&ctx.vm.navigation.current_url);
    Ok(intern_current(ctx, &s))
}

pub(super) fn native_location_get_href(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |u| u.as_str().to_string())
}

/// Shared setter body — used by `href = …`, `assign(url)`, and
/// `replace(url)`.  `replace_history` controls whether the mutation
/// pushes a new history entry (`assign` / `href=`) or overwrites the
/// current one (`replace`).  `new_url` has already been parsed.
fn set_location(ctx: &mut NativeContext<'_>, new_url: Url, replace_history: bool) {
    let nav = &mut ctx.vm.navigation;
    nav.current_url = new_url.clone();
    if replace_history {
        // `history_entries` is non-empty by `NavigationState::new`
        // invariant and every code path preserves it — index directly
        // rather than `get_mut`.
        debug_assert!(!nav.history_entries.is_empty());
        nav.history_entries[nav.history_index].url = new_url;
        // `replace` is a navigation, not a state mutation — clear any
        // prior `pushState` data so `history.state` reflects a fresh
        // navigation entry rather than preserving stale state.
        nav.history_entries[nav.history_index].state = JsValue::Null;
    } else {
        nav.push_entry(new_url, JsValue::Null);
    }
}

pub(super) fn native_location_set_href(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let input = ctx.vm.strings.get_utf8(sid);
    let Some(parsed) = resolve_url(ctx, &input) else {
        let syntax = ctx.vm.well_known.dom_exc_syntax_error;
        return Err(VmError::dom_exception(
            syntax,
            format!("Failed to set 'href' on 'Location': invalid URL '{input}'."),
        ));
    };
    set_location(ctx, parsed, false);
    Ok(JsValue::Undefined)
}

pub(super) fn native_location_get_protocol(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `scheme()` returns the scheme without the trailing `:` — spec
    // `location.protocol` includes it (`"http:"`).
    url_component(ctx, |u| format!("{}:", u.scheme()))
}

pub(super) fn native_location_get_host(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |u| match (u.host_str(), u.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_string(),
        (None, _) => String::new(),
    })
}

pub(super) fn native_location_get_hostname(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |u| u.host_str().unwrap_or("").to_string())
}

pub(super) fn native_location_get_port(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Default ports are stripped by the `url` crate (`port()`
    // returns None for `http://host/` even though the semantic port
    // is 80) — matching WHATWG URL and what every browser returns.
    url_component(ctx, |u| match u.port() {
        Some(p) => p.to_string(),
        None => String::new(),
    })
}

pub(super) fn native_location_get_pathname(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    url_component(ctx, |u| u.path().to_string())
}

pub(super) fn native_location_get_search(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `query()` strips the leading `?` — spec `location.search`
    // includes it for non-empty queries, and is the empty string
    // for a missing/empty query.
    url_component(ctx, |u| match u.query() {
        Some(q) if !q.is_empty() => format!("?{q}"),
        _ => String::new(),
    })
}

pub(super) fn native_location_get_hash(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `fragment()` strips the leading `#` — spec `location.hash`
    // includes it for non-empty fragments, empty string for
    // missing/empty.
    url_component(ctx, |u| match u.fragment() {
        Some(f) if !f.is_empty() => format!("#{f}"),
        _ => String::new(),
    })
}

pub(super) fn native_location_get_origin(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `url::Origin` handles the special-scheme list; `.unicode_serialization()`
    // returns `"null"` for opaque origins (matches HTML §7.2.3).
    url_component(ctx, |u| u.origin().unicode_serialization())
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
    let input = ctx.vm.strings.get_utf8(sid);
    let Some(parsed) = resolve_url(ctx, &input) else {
        let syntax = ctx.vm.well_known.dom_exc_syntax_error;
        return Err(VmError::dom_exception(
            syntax,
            format!("Failed to execute 'assign' on 'Location': invalid URL '{input}'."),
        ));
    };
    set_location(ctx, parsed, false);
    Ok(JsValue::Undefined)
}

pub(super) fn native_location_replace(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let input = ctx.vm.strings.get_utf8(sid);
    let Some(parsed) = resolve_url(ctx, &input) else {
        let syntax = ctx.vm.well_known.dom_exc_syntax_error;
        return Err(VmError::dom_exception(
            syntax,
            format!("Failed to execute 'replace' on 'Location': invalid URL '{input}'."),
        ));
    };
    set_location(ctx, parsed, true);
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
